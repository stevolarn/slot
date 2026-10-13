//! Every game handed back to Shelf, kept in `Config/receipts.txt` as
//! `<returned>\t<seconds>\t<total seconds>\t<rom path>` lines, oldest
//! first: the return receipts on the spike.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::CONFIG_DIR;

pub const RECEIPTS_FILE: &str = "receipts.txt";
/// How long a rental is out for before it is due back.
pub const DUE_BACK_DAYS: i64 = 3;

/// One game returned: when, in unix seconds on the wall clock's UTC, how
/// long it was out for, and the game's whole play time after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub rom: PathBuf,
    pub returned: i64,
    pub secs: u64,
    pub total: u64,
}

impl Receipt {
    pub fn rented(&self) -> i64 {
        self.returned - self.secs as i64
    }

    pub fn due_back(&self) -> i64 {
        self.returned + DUE_BACK_DAYS * 86_400
    }
}

pub fn receipts_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(RECEIPTS_FILE)
}

/// Add `r` to the end of the spike.
pub fn append_receipt(root: &Path, r: &Receipt) -> std::io::Result<()> {
    let path = receipts_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(
        file,
        "{}\t{}\t{}\t{}",
        r.returned,
        r.secs,
        r.total,
        r.rom.display()
    )
}

/// The receipts on the spike, newest first.
pub fn read_receipts(root: &Path) -> Vec<Receipt> {
    let text = std::fs::read_to_string(receipts_path(root)).unwrap_or_default();
    let mut out: Vec<Receipt> = text.lines().filter_map(parse_line).collect();
    out.reverse();
    out
}

fn parse_line(line: &str) -> Option<Receipt> {
    let mut parts = line.splitn(4, '\t');
    let returned = parts.next()?.parse().ok()?;
    let secs = parts.next()?.parse().ok()?;
    let total = parts.next()?.parse().ok()?;
    let rom = parts.next().filter(|r| !r.is_empty())?;
    Some(Receipt {
        rom: PathBuf::from(rom),
        returned,
        secs,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipts_come_back_newest_first_and_bad_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let first = Receipt {
            rom: PathBuf::from("/Roms/MD/Streets of Rage 3 (USA).md"),
            returned: 1_760_000_000,
            secs: 4_320,
            total: 9_000,
        };
        let second = Receipt {
            rom: PathBuf::from("/Roms/GBA/Advance Wars.gba"),
            returned: 1_760_100_000,
            secs: 600,
            total: 600,
        };
        append_receipt(dir.path(), &first).unwrap();
        let path = receipts_path(dir.path());
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("garbage\n1\t2\tx\t/rom\n");
        std::fs::write(&path, text).unwrap();
        append_receipt(dir.path(), &second).unwrap();
        assert_eq!(read_receipts(dir.path()), vec![second, first.clone()]);
        assert_eq!(first.rented(), 1_760_000_000 - 4_320);
        assert_eq!(first.due_back(), 1_760_000_000 + 3 * 86_400);
    }
}
