//! How often each game has been played and when it was last played, kept
//! in `Config/rentals.txt` as `<times>\t<unix seconds>\t<rom path>` lines,
//! for the rental sticker on the box. Unlike `played.txt`, blowing on a
//! cart does not count.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::config::CONFIG_DIR;

pub const RENTALS_FILE: &str = "rentals.txt";

/// How many times a game has been played, and when last, in unix seconds.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rental {
    pub times: u32,
    pub last: i64,
}

pub type Rentals = HashMap<PathBuf, Rental>;

pub fn rentals_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(RENTALS_FILE)
}

pub fn read_rentals(root: &Path) -> Rentals {
    let text = std::fs::read_to_string(rentals_path(root)).unwrap_or_default();
    let mut out = Rentals::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(times), Some(last), Some(rom)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if let (Ok(times), Ok(last), false) = (times.parse(), last.parse(), rom.is_empty()) {
            out.insert(PathBuf::from(rom), Rental { times, last });
        }
    }
    out
}

pub fn write_rentals(root: &Path, rentals: &Rentals) -> std::io::Result<()> {
    let mut lines: Vec<(&PathBuf, &Rental)> = rentals.iter().collect();
    lines.sort_by(|a, b| a.0.cmp(b.0));
    let text: String = lines
        .iter()
        .map(|(rom, r)| format!("{}\t{}\t{}\n", r.times, r.last, rom.display()))
        .collect();
    let path = rentals_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, text.as_bytes())
}

/// Count one more play of `rom` at `now`.
pub fn rent(rentals: &mut Rentals, rom: &Path, now: i64) {
    let r = rentals.entry(rom.to_path_buf()).or_insert(Rental {
        times: 0,
        last: now,
    });
    r.times += 1;
    r.last = now;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_are_counted_and_survive_a_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut r = Rentals::new();
        rent(&mut r, Path::new("/Roms/GB/Tetris.zip"), 100);
        rent(&mut r, Path::new("/Roms/GB/Tetris.zip"), 200);
        rent(&mut r, Path::new("/Roms/GB/Kirby, The.gb"), 150);
        write_rentals(dir.path(), &r).unwrap();
        let back = read_rentals(dir.path());
        assert_eq!(back, r);
        assert_eq!(
            back[Path::new("/Roms/GB/Tetris.zip")],
            Rental {
                times: 2,
                last: 200
            }
        );
    }

    #[test]
    fn bad_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(CONFIG_DIR)).unwrap();
        std::fs::write(
            rentals_path(dir.path()),
            "x\t1\t/a\n3\t9\t\n2\t5\t/Roms/b.gb\n",
        )
        .unwrap();
        let r = read_rentals(dir.path());
        assert_eq!(r.len(), 1);
        assert_eq!(r[Path::new("/Roms/b.gb")], Rental { times: 2, last: 5 });
    }
}
