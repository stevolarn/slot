//! Every game handed back to Shelf, kept in `Config/receipts.txt` as
//! `<returned>\t<seconds>\t<total seconds>\t<rented>\t<rom path>` lines,
//! oldest first: the return receipts on the spike. The first receipts had
//! no `<rented>`; it is taken as the play time before the return.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::CONFIG_DIR;

pub const RECEIPTS_FILE: &str = "receipts.txt";
/// How long a rental is out for before it is due back.
pub const DUE_BACK_DAYS: i64 = 3;

/// One game rented and returned: when, in unix seconds on the wall
/// clock's UTC, how long it was played for, which leaves out sleep, and
/// the game's whole play time after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub rom: PathBuf,
    pub rented: i64,
    pub returned: i64,
    pub secs: u64,
    pub total: u64,
}

impl Receipt {
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
        "{}\t{}\t{}\t{}\t{}",
        r.returned,
        r.secs,
        r.total,
        r.rented,
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
    let returned: i64 = parts.next()?.parse().ok()?;
    let secs: u64 = parts.next()?.parse().ok()?;
    let total = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    // A rom path never starts with a digit: it is absolute.
    let (rented, rom) = match rest.split_once('\t') {
        Some((rented, rom)) if rented.parse::<i64>().is_ok() => (rented.parse().ok()?, rom),
        _ => (returned - secs as i64, rest),
    };
    (!rom.is_empty()).then(|| Receipt {
        rom: PathBuf::from(rom),
        rented,
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
            rented: 1_759_990_000,
            returned: 1_760_000_000,
            secs: 4_320,
            total: 9_000,
        };
        let second = Receipt {
            rom: PathBuf::from("/Roms/GBA/Advance Wars.gba"),
            rented: 1_760_099_400,
            returned: 1_760_100_000,
            secs: 600,
            total: 600,
        };
        append_receipt(dir.path(), &first).unwrap();
        let path = receipts_path(dir.path());
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("garbage\n1\t2\tx\t/rom\n500\t100\t100\t/Roms/GB/Old.gb\n");
        std::fs::write(&path, text).unwrap();
        append_receipt(dir.path(), &second).unwrap();
        let old = Receipt {
            rom: PathBuf::from("/Roms/GB/Old.gb"),
            rented: 400,
            returned: 500,
            secs: 100,
            total: 100,
        };
        assert_eq!(read_receipts(dir.path()), vec![second, old, first.clone()]);
        assert_eq!(first.due_back(), 1_760_000_000 + 3 * 86_400);
    }
}
