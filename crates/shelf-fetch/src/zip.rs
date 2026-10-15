//! One file out of a zip that is read in ranges, such as LaunchBox's
//! Metadata.zip on its web server: the directory at the end is read first,
//! then only the wanted file's bytes, inflated as they arrive, so nothing
//! of the zip is kept on the card.

use std::io::{self, Read};

/// Somewhere a zip's bytes can be read from, a range at a time.
#[allow(clippy::len_without_is_empty)]
pub trait Source {
    /// How many bytes the zip is.
    fn len(&self) -> io::Result<u64>;
    /// Bytes `start` to `end`, not including `end`.
    fn range(&self, start: u64, end: u64) -> io::Result<Box<dyn Read + Send>>;
}

/// A zip on disk, for tests and for a Metadata.zip copied to the card.
pub struct FileSource(pub std::path::PathBuf);

impl Source for FileSource {
    fn len(&self) -> io::Result<u64> {
        Ok(std::fs::metadata(&self.0)?.len())
    }

    fn range(&self, start: u64, end: u64) -> io::Result<Box<dyn Read + Send>> {
        use std::io::{Seek, SeekFrom};
        let mut f = std::fs::File::open(&self.0)?;
        f.seek(SeekFrom::Start(start))?;
        Ok(Box::new(f.take(end.saturating_sub(start))))
    }
}

/// Where a file's compressed bytes start in the zip, how many there are,
/// and how they are packed (0 stored, 8 deflated).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub header: u64,
    pub compressed: u64,
    pub method: u16,
}

/// How much of the end of the zip is read to find its directory. The
/// directory of a zip of a few files is a few hundred bytes.
const TAIL: u64 = 64 * 1024;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The entry for `name` in the directory found in `tail`, the last bytes of
/// a zip, which start at `tail_start` in it.
pub fn find_entry(tail: &[u8], tail_start: u64, name: &str) -> Option<Entry> {
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| u32_at(tail, i) == Some(0x0605_4b50))?;
    let count = u16_at(tail, end + 10)? as usize;
    let dir = u64::from(u32_at(tail, end + 16)?);
    let mut at = usize::try_from(dir.checked_sub(tail_start)?).ok()?;
    for _ in 0..count {
        if u32_at(tail, at)? != 0x0201_4b50 {
            return None;
        }
        let method = u16_at(tail, at + 10)?;
        let compressed = u64::from(u32_at(tail, at + 20)?);
        let name_len = u16_at(tail, at + 28)? as usize;
        let extra_len = u16_at(tail, at + 30)? as usize;
        let comment_len = u16_at(tail, at + 32)? as usize;
        let header = u64::from(u32_at(tail, at + 42)?);
        let entry_name = tail.get(at + 46..at + 46 + name_len)?;
        if entry_name == name.as_bytes() {
            return Some(Entry {
                header,
                compressed,
                method,
            });
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    None
}

/// Counts the bytes read through it, for how far along a download is.
pub struct Counted<R> {
    inner: R,
    count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count
            .fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(n)
    }
}

/// The file `name` in the zip, unpacked as it is read, and how many
/// compressed bytes it is. `read` counts the compressed bytes read so far.
pub fn open(
    source: &dyn Source,
    name: &str,
    read: std::sync::Arc<std::sync::atomic::AtomicU64>,
) -> io::Result<(Box<dyn Read + Send>, u64)> {
    let len = source.len()?;
    let tail_start = len.saturating_sub(TAIL);
    let mut tail = Vec::new();
    source.range(tail_start, len)?.read_to_end(&mut tail)?;
    let entry = find_entry(&tail, tail_start, name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no {name} in the zip")))?;
    // The local header: 30 bytes, then the name and an extra field whose
    // lengths it gives, which may differ from the directory's.
    let mut body = source.range(entry.header, len)?;
    let mut head = [0u8; 30];
    body.read_exact(&mut head)?;
    if u32_at(&head, 0) != Some(0x0403_4b50) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a zip entry"));
    }
    let skip = u64::from(u16_at(&head, 26).unwrap_or(0)) + u64::from(u16_at(&head, 28).unwrap_or(0));
    io::copy(&mut (&mut body).take(skip), &mut io::sink())?;
    let packed = Counted {
        inner: body.take(entry.compressed),
        count: read,
    };
    let reader: Box<dyn Read + Send> = match entry.method {
        0 => Box::new(packed),
        8 => Box::new(flate2::read::DeflateDecoder::new(packed)),
        m => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("zip method {m} is not supported"),
            ))
        }
    };
    Ok((reader, entry.compressed))
}
