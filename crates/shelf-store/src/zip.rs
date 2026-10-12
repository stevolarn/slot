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
const MAX_IN: usize = 1 << 16;
/// The first read of a file's data, which gives back a rom header unless it
/// is unusually hard to compress.
const FIRST_READ: usize = 4096;
/// The first read of the directory.
const DIR_FIRST: usize = 1024;

/// The first `len` bytes of the first file in the zip at `path`, or None
/// when it is not a zip, is damaged, or holds something shorter.
///
/// A zip written without a comment ends in its 22-byte end record, so the
/// common case is three small reads: that record, the start of the
/// directory, and the first file's local header with the start of its data.
/// Only a zip with a comment, or a long first entry, takes the bigger reads.
pub fn head(path: &Path, len: usize) -> Option<Vec<u8>> {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("7z"))
    {
        return None;
    }
    let mut f = File::open(path).ok()?;
    let size = f.seek(SeekFrom::End(0)).ok()?;
    let end = end_record(&mut f, size)?;
    let entries = u16_at(&end, 10)?;
    let dir_len = u32_at(&end, 12)? as usize;
    let dir_at = u32_at(&end, 16)? as u64;
    // The first entry nearly always fits in a short read of the directory.
    let mut want = dir_len.min(DIR_FIRST);
    loop {
        let dir = read_upto(&mut f, dir_at, want)?;
        match first_file(&dir, entries) {
            Some(Some(entry)) => return data_head(&mut f, &entry, len),
            Some(None) => return None,
            // The directory read ended inside the entry.
            None if want < dir_len => want = dir_len,
            None => return None,
        }
    }
}

/// The end-of-directory record: the last 22 bytes of a zip without a
/// comment, else found by scanning back through the longest comment.
fn end_record(f: &mut File, size: u64) -> Option<Vec<u8>> {
    if size >= 22 {
        let last = read(f, size - 22, 22)?;
        if u32_at(&last, 0) == Some(END_SIG) {
            return Some(last);
        }
    }
    let from = size.saturating_sub(TAIL);
    let tail = read(f, from, (size - from) as usize)?;
    let at = (0..tail.len().checked_sub(21)?)
        .rev()
        .find(|&i| u32_at(&tail, i) == Some(END_SIG))?;
    Some(tail[at..].to_vec())
}

struct Entry {
    method: u16,
    packed: u64,
    local: u64,
}

/// The first real file in a directory: `Some(None)` when the directory is
/// whole and holds none, `None` when it is cut short or damaged.
fn first_file(dir: &[u8], entries: u16) -> Option<Option<Entry>> {
    let mut at = 0;
    for _ in 0..entries {
        if u32_at(dir, at)? != ENTRY_SIG {
            return None;
        }
        let method = u16_at(dir, at + 10)?;
        let packed = u32_at(dir, at + 20)? as u64;
        let name_len = u16_at(dir, at + 28)? as usize;
        let skip = name_len + u16_at(dir, at + 30)? as usize + u16_at(dir, at + 32)? as usize;
        let local = u32_at(dir, at + 42)? as u64;
        let name = dir.get(at + 46..at + 46 + name_len)?;
        at += 46 + skip;
        if name.ends_with(b"/") {
            continue;
        }
        return Some(Some(Entry {
            method,
            packed,
            local,
        }));
    }
    Some(None)
}

fn data_head(f: &mut File, entry: &Entry, len: usize) -> Option<Vec<u8>> {
    let mut take = FIRST_READ;
    loop {
        // The local header, then the start of the data, in one read.
        let buf = read_upto(f, entry.local, 30 + take)?;
        if u32_at(&buf, 0)? != LOCAL_SIG {
            return None;
        }
        let start = 30 + u16_at(&buf, 26)? as usize + u16_at(&buf, 28)? as usize;
        let data = buf.get(start..).unwrap_or_default();
        let whole = data.len() as u64 >= entry.packed;
        let out = match entry.method {
            STORED if data.len() >= len => data[..len].to_vec(),
            STORED => Vec::new(),
            DEFLATED => {
                let input = &data[..(data.len() as u64).min(entry.packed) as usize];
                match miniz_oxide::inflate::decompress_to_vec_with_limit(input, len) {
                    Ok(v) => v,
                    Err(e) => e.output,
                }
            }
            _ => return None,
        };
        if out.len() >= len {
            return Some(out[..len].to_vec());
        }
        // Short: either the read stopped before enough input, or the file
        // holds less than `len`.
        if whole || take >= MAX_IN {
            return None;
        }
        take = MAX_IN.max(start + len);
    }
}

fn read(f: &mut File, at: u64, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(at)).ok()?;
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// Up to `len` bytes from `at`, fewer at the end of the file.
fn read_upto(f: &mut File, at: u64, len: usize) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(len);
    f.seek(SeekFrom::Start(at)).ok()?;
    f.by_ref().take(len as u64).read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
