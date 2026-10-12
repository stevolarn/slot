use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const TITLE_OFF: u64 = 0x134;
const TITLE_LEN: usize = 11;

const CGB_OFF: u64 = 0x143;

const CODE_OFF: usize = 0x13f;
const DEST_OFF: usize = 0x14a;
pub const HEADER_LEN: usize = 0x150;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub title: String,
    pub code: String,
    pub cgb: u8,
    pub japan: bool,
}

impl Header {
    pub fn parse(bytes: &[u8]) -> Option<Header> {
        let bytes = bytes.get(..HEADER_LEN)?;
        let title = &bytes[TITLE_OFF as usize..][..TITLE_LEN];
        let end = title.iter().position(|b| *b == 0).unwrap_or(TITLE_LEN);
        let code = &bytes[CODE_OFF..CODE_OFF + 4];
        let is_code = code
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
        Some(Header {
            title: String::from_utf8_lossy(&title[..end]).trim().to_string(),
            code: if is_code {
                String::from_utf8_lossy(code).into_owned()
            } else {
                String::new()
            },
            cgb: bytes[CGB_OFF as usize],
            japan: bytes[DEST_OFF] == 0,
        })
    }

    pub fn class(&self) -> Class {
        class_of(Some(self.cgb))
    }
}

pub fn header(rom: &Path) -> Option<Header> {
    let mut buf = [0u8; HEADER_LEN];
    read_at(rom, 0, &mut buf)?;
    Header::parse(&buf)
}

/// The header of `rom`, read from the card the first time it is asked for
/// and remembered after that, so a game's header is read once however many
/// faces, shells and shapes want it. A rom that has no readable header is
/// remembered too.
pub fn header_cached(rom: &Path) -> Option<Header> {
    static HEADERS: OnceLock<Mutex<HashMap<PathBuf, Option<Header>>>> = OnceLock::new();
    let headers = HEADERS.get_or_init(Mutex::default);
    let lock = || headers.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(known) = lock().get(rom) {
        return known.clone();
    }
    // Not under the lock: reading a header from the card is slow.
    READS.fetch_add(1, Ordering::Relaxed);
    let read = header(rom);
    lock().insert(rom.to_path_buf(), read.clone());
    read
}

/// How many headers [`header_cached`] has had to read from the card, for
/// tests of the cache.
pub fn cached_reads() -> usize {
    READS.load(Ordering::Relaxed)
}

static READS: AtomicUsize = AtomicUsize::new(0);

/// [`class`], from the remembered header.
pub fn class_cached(rom: &Path) -> Class {
    class_of(header_cached(rom).map(|h| h.cgb))
}

pub fn title(rom: &Path) -> Option<String> {
    let mut buf = [0u8; TITLE_LEN];
    read_at(rom, TITLE_OFF, &mut buf)?;
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let text = std::str::from_utf8(&buf[..end]).ok()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

pub fn cgb_flag(rom: &Path) -> Option<u8> {
    let mut buf = [0u8; 1];
    read_at(rom, CGB_OFF, &mut buf)?;
    Some(buf[0])
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Class {
    Original,
    DualMode,
    ColourOnly,
}

pub fn class(rom: &Path) -> Class {
    class_of(cgb_flag(rom))
}

fn class_of(flag: Option<u8>) -> Class {
    match flag {
        Some(0xc0) => Class::ColourOnly,
        Some(0x80) => Class::DualMode,
        _ => Class::Original,
    }
}

fn read_at(rom: &Path, off: u64, buf: &mut [u8]) -> Option<()> {
    if crate::platform::is_archive(rom) {
        let head = crate::zip::head(rom, HEADER_LEN)?;
        buf.copy_from_slice(head.get(off as usize..off as usize + buf.len())?);
        return Some(());
    }
    let mut f = File::open(rom).ok()?;
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(buf).ok()
}
