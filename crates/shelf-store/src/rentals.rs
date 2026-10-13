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

/// Seconds each game has been played for, kept in `Config/playtime.txt`
/// as `<seconds>\t<rom path>` lines.
pub type PlayTimes = HashMap<PathBuf, u64>;

pub const PLAYTIME_FILE: &str = "playtime.txt";
/// The game handed to NextUI and when, on the clock that stops while the
/// Brick is suspended and on the wall clock, in UTC unix seconds:
/// `<seconds>\t<wall seconds>\t<rom path>`, until Shelf comes back. Older
/// Shelfs wrote `<seconds>\t<rom path>`.
pub const PLAYING_FILE: &str = "playing.txt";
/// Longer than this between a game going in and Shelf coming back is not
/// taken for play: the Brick was left on, or the clock went wrong.
pub const PLAY_SESSION_MAX: u64 = 12 * 3600;

pub fn read_play_times(root: &Path) -> PlayTimes {
    let text = std::fs::read_to_string(root.join(CONFIG_DIR).join(PLAYTIME_FILE))
        .unwrap_or_default();
    let mut out = PlayTimes::new();
    for line in text.lines() {
        let Some((secs, rom)) = line.split_once('\t') else {
            continue;
        };
        if let (Ok(secs), false) = (secs.parse::<u64>(), rom.is_empty()) {
            *out.entry(PathBuf::from(rom)).or_insert(0) += secs;
        }
    }
    out
}

pub fn write_play_times(root: &Path, times: &PlayTimes) -> std::io::Result<()> {
    let mut lines: Vec<(&PathBuf, &u64)> = times.iter().collect();
    lines.sort_by(|a, b| a.0.cmp(b.0));
    let text: String = lines
        .iter()
        .map(|(rom, secs)| format!("{secs}\t{}\n", rom.display()))
        .collect();
    let dir = root.join(CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    atomic_write(&dir.join(PLAYTIME_FILE), text.as_bytes())
}

/// Note that `rom` went in at `clock` seconds, and at `wall` on the wall
/// clock, for `finish_playing`.
pub fn start_playing(root: &Path, rom: &Path, clock: u64, wall: i64) -> std::io::Result<()> {
    let dir = root.join(CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    let text = format!("{clock}\t{wall}\t{}\n", rom.display());
    atomic_write(&dir.join(PLAYING_FILE), text.as_bytes())
}

/// A game back from NextUI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub rom: PathBuf,
    /// Seconds since it went in, on the clock that stops only while the
    /// Brick is suspended: NextUI's screen-off sleep is counted in it.
    pub secs: u64,
    /// When it went in on the wall clock, if known.
    pub since: Option<i64>,
}

/// As Shelf comes back at `clock`: the game that went in and how long ago,
/// if a believable session was found. Forgets the game either way.
pub fn finish_playing(root: &Path, clock: u64) -> Option<Session> {
    let path = root.join(CONFIG_DIR).join(PLAYING_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let (start, rest) = text.trim_end_matches('\n').split_once('\t')?;
    let start: u64 = start.parse().ok()?;
    // A rom path never starts with a digit: it is absolute.
    let (since, rom) = match rest.split_once('\t') {
        Some((wall, rom)) if wall.parse::<i64>().is_ok() => (wall.parse().ok(), rom),
        _ => (None, rest),
    };
    let secs = clock.checked_sub(start).filter(|s| *s <= PLAY_SESSION_MAX)?;
    if rom.is_empty() || secs == 0 {
        return None;
    }
    Some(Session {
        rom: PathBuf::from(rom),
        secs,
        since,
    })
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
    fn a_session_adds_its_time_once_and_an_unbelievable_one_adds_none() {
        let dir = tempfile::tempdir().unwrap();
        let rom = Path::new("/Roms/MD/Sonic.md");
        let mut times = PlayTimes::new();
        start_playing(dir.path(), rom, 1_000, 1_760_000_000).unwrap();
        assert_eq!(
            finish_playing(dir.path(), 1_600),
            Some(Session {
                rom: rom.to_path_buf(),
                secs: 600,
                since: Some(1_760_000_000)
            })
        );
        assert_eq!(finish_playing(dir.path(), 9_000), None, "counted twice");
        start_playing(dir.path(), rom, 5_000, 0).unwrap();
        assert_eq!(finish_playing(dir.path(), 100), None, "the Brick restarted");
        start_playing(dir.path(), rom, 0, 0).unwrap();
        assert_eq!(finish_playing(dir.path(), PLAY_SESSION_MAX + 1), None);
        // What an older Shelf left behind still reads.
        let playing = dir.path().join(CONFIG_DIR).join(PLAYING_FILE);
        std::fs::write(&playing, "1000\t/Roms/MD/Sonic.md\n").unwrap();
        let old = finish_playing(dir.path(), 1_300).unwrap();
        assert_eq!((old.rom.as_path(), old.secs, old.since), (rom, 300, None));
        times.insert(rom.to_path_buf(), 600);
        write_play_times(dir.path(), &times).unwrap();
        assert_eq!(read_play_times(dir.path()), times);
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
