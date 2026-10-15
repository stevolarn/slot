//! When each game was last played or blown on, kept in `Config/played.txt`
//! as `<unix seconds>\t<rom path>` lines, so carts left alone gather dust.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::config::CONFIG_DIR;

pub const PLAYED_FILE: &str = "played.txt";

/// Days a cart stays clean, and the days after that it takes to go from
/// clean to fully dusty.
const CLEAN_DAYS: f32 = 7.0;
const DUSTY_DAYS: f32 = 60.0;
/// The dust on a cart nobody has played or blown on yet.
const UNTOUCHED_DUST: f32 = 0.35;

const DAY_SECS: i64 = 86_400;

/// Last-touched time of each rom, in unix seconds.
pub type Played = HashMap<PathBuf, i64>;

pub fn played_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(PLAYED_FILE)
}

pub fn read_played(root: &Path) -> Played {
    let text = std::fs::read_to_string(played_path(root)).unwrap_or_default();
    let mut out = Played::new();
    for line in text.lines() {
        let Some((secs, rom)) = line.split_once('\t') else {
            continue;
        };
        if let (Ok(secs), false) = (secs.trim().parse::<i64>(), rom.is_empty()) {
            out.insert(PathBuf::from(rom), secs);
        }
    }
    out
}

pub fn write_played(root: &Path, played: &Played) -> std::io::Result<()> {
    let mut lines: Vec<(&PathBuf, &i64)> = played.iter().collect();
    lines.sort();
    let text: String = lines
        .iter()
        .map(|(rom, secs)| format!("{secs}\t{}\n", rom.display()))
        .collect();
    let path = played_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, text.as_bytes())
}

/// How dusty a cart is, 0 (clean) to 1 (fully dusty), given when it was
/// last played or blown on and the time now, both in unix seconds. Clean
/// for a week, then fully dusty about two months on; a cart with no time
/// yet starts lightly dusty.
pub fn dust_amount(last: Option<i64>, now: i64) -> f32 {
    let Some(last) = last else {
        return UNTOUCHED_DUST;
    };
    let days = (now - last) as f32 / DAY_SECS as f32;
    ((days - CLEAN_DAYS) / DUSTY_DAYS).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn days_ago(d: i64) -> Option<i64> {
        Some(NOW - d * DAY_SECS)
    }

    #[test]
    fn a_cart_is_clean_for_a_week() {
        assert_eq!(dust_amount(days_ago(0), NOW), 0.0);
        assert_eq!(dust_amount(days_ago(7), NOW), 0.0);
        assert!(dust_amount(days_ago(8), NOW) > 0.0);
    }

    #[test]
    fn dust_builds_up_to_full_in_about_two_months() {
        let half = dust_amount(days_ago(37), NOW);
        assert!((half - 0.5).abs() < 1e-4, "{half}");
        assert_eq!(dust_amount(days_ago(67), NOW), 1.0);
        assert_eq!(dust_amount(days_ago(900), NOW), 1.0);
    }

    #[test]
    fn a_cart_never_touched_starts_lightly_dusty() {
        assert_eq!(dust_amount(None, NOW), 0.35);
    }

    #[test]
    fn a_time_in_the_future_is_clean() {
        assert_eq!(dust_amount(Some(NOW + 5 * DAY_SECS), NOW), 0.0);
    }

    #[test]
    fn played_times_round_trip() {
        let dir = std::env::temp_dir().join(format!("shelf-played-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut played = Played::new();
        played.insert(PathBuf::from("/Roms/Game Boy (GB)/Tetris.gb"), NOW);
        played.insert(PathBuf::from("/Roms/GBA (GBA)/A B.gba"), NOW - 5);
        write_played(&dir, &played).unwrap();
        assert_eq!(read_played(&dir), played);
        let text = std::fs::read_to_string(played_path(&dir)).unwrap();
        assert!(text.contains(&format!("{NOW}\t/Roms/Game Boy (GB)/Tetris.gb\n")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_damaged_file_reads_as_nothing_played() {
        let dir = std::env::temp_dir().join(format!("shelf-played-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read_played(&dir).is_empty());
        std::fs::create_dir_all(dir.join(CONFIG_DIR)).unwrap();
        std::fs::write(played_path(&dir), "junk\nnope\t/x\n12\t\n34\t/ok\n").unwrap();
        let got = read_played(&dir);
        assert_eq!(got.len(), 1);
        assert_eq!(got[&PathBuf::from("/ok")], 34);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
