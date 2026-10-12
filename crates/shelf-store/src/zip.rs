//! Just enough of zip to read the start of the rom inside a zipped game, so
//! its header can pick the cart it is drawn as.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const END_SIG: u32 = 0x0605_4b50;
const ENTRY_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
/// The end record sits within its own length plus the longest comment.
const TAIL: u64 = 22 + 0xffff;
/// Enough deflated input to give back any rom header.
const MAX_IN: u64 = 1 << 16;

/// The first `len` bytes of the first file in the zip at `path`, or None
/// when it is not a zip, is damaged, or holds something shorter.
pub fn head(path: &Path, len: usize) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    let size = f.seek(SeekFrom::End(0)).ok()?;
    let from = size.saturating_sub(TAIL);
    let tail = read(&mut f, from, (size - from) as usize)?;
    let end = (0..tail.len().checked_sub(21)?)
        .rev()
        .find(|&i| u32_at(&tail, i) == Some(END_SIG))?;
    let entries = u16_at(&tail, end + 10)?;
    let dir_len = u32_at(&tail, end + 12)? as usize;
    let dir = read(&mut f, u32_at(&tail, end + 16)? as u64, dir_len)?;

    let mut at = 0;
    for _ in 0..entries {
        if u32_at(&dir, at)? != ENTRY_SIG {
            return None;
        }
        let method = u16_at(&dir, at + 10)?;
        let packed = u32_at(&dir, at + 20)? as u64;
        let name_len = u16_at(&dir, at + 28)? as usize;
        let skip = name_len + u16_at(&dir, at + 30)? as usize + u16_at(&dir, at + 32)? as usize;
        let local = u32_at(&dir, at + 42)? as u64;
        let name = dir.get(at + 46..at + 46 + name_len)?;
        at += 46 + skip;
        if name.ends_with(b"/") {
            continue;
        }
        let lh = read(&mut f, local, 30)?;
        if u32_at(&lh, 0)? != LOCAL_SIG {
            return None;
        }
        let data = local + 30 + u16_at(&lh, 26)? as u64 + u16_at(&lh, 28)? as u64;
        let out = match method {
            STORED => read(&mut f, data, len)?,
            DEFLATED => {
                let input = read(&mut f, data, packed.min(MAX_IN) as usize)?;
                match miniz_oxide::inflate::decompress_to_vec_with_limit(&input, len) {
                    Ok(v) => v,
                    Err(e) => e.output,
                }
            }
            _ => return None,
        };
        return (out.len() >= len).then(|| out[..len].to_vec());
    }
    None
}

fn read(f: &mut File, at: u64, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(at)).ok()?;
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
