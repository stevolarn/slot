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
/// The game handed to NextUI: `<clock>\t<wall>\t<boot>\t<rom path>`, until
/// Shelf comes back. `<clock>` is seconds on the clock that stops only
/// while the Brick is suspended, or `-` when not known; `<wall>` is UTC
/// unix seconds; `<boot>` is the kernel's boot id, or `-`. Older Shelfs
/// wrote `<clock>\t<rom path>` and `<clock>\t<wall>\t<rom path>`.
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

/// Note that `rom` went in at `clock` seconds, at `wall` on the wall
/// clock and in the boot `boot`, for `finish_playing`.
pub fn start_playing(
    root: &Path,
    rom: &Path,
    clock: u64,
    wall: i64,
    boot: Option<&str>,
) -> std::io::Result<()> {
    let dir = root.join(CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    let text = format!("{clock}\t{wall}\t{}\t{}\n", boot.unwrap_or("-"), rom.display());
    atomic_write(&dir.join(PLAYING_FILE), text.as_bytes())
}

/// A game back from NextUI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub rom: PathBuf,
    /// Seconds since it went in, on the clock that stops only while the
    /// Brick is suspended: NextUI's screen-off sleep is counted in it.
    /// `None` if not known or not believable.
    pub secs: Option<u64>,
    /// When it went in on the wall clock, if known.
    pub since: Option<i64>,
    /// The boot it went in during, if known.
    pub boot: Option<String>,
}

/// As Shelf comes back at `clock`: the game that went in, and what is
/// known of when. Forgets the game either way.
pub fn finish_playing(root: &Path, clock: u64) -> Option<Session> {
    let path = root.join(CONFIG_DIR).join(PLAYING_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let fields: Vec<&str> = text.trim_end_matches('\n').splitn(4, '\t').collect();
    let (start, wall, boot, rom) = match fields[..] {
        [start, rom] => (start, None, None, rom),
        [start, wall, rom] => (start, Some(wall), None, rom),
        [start, wall, boot, rom] => (start, Some(wall), Some(boot), rom),
        _ => return None,
    };
    if rom.is_empty() {
        return None;
    }
    let secs = start
        .parse::<u64>()
        .ok()
        .and_then(|start| clock.checked_sub(start))
        .filter(|s| (1..=PLAY_SESSION_MAX).contains(s));
    Some(Session {
        rom: PathBuf::from(rom),
        secs,
        since: wall.and_then(|w| w.parse().ok()),
        boot: boot.filter(|b| *b != "-" && !b.is_empty()).map(str::to_string),
    })
}

/// When the Brick was powered off in a game: `<unix seconds>\t<rom path>`
/// lines, written by the pak's boot hook from NextUI's resume note.
pub const POWER_OFFS_FILE: &str = "poweroffs.txt";

/// Each game powered off in, and when, in UTC unix seconds.
pub fn read_power_offs(root: &Path) -> Vec<(PathBuf, i64)> {
    let text = std::fs::read_to_string(root.join(CONFIG_DIR).join(POWER_OFFS_FILE))
        .unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let (at, rom) = line.split_once('\t')?;
            let at = at.trim().parse().ok()?;
            (!rom.is_empty()).then(|| (PathBuf::from(rom), at))
        })
        .collect()
}

/// The kernel's id for this boot, to tell a game handed out before a
/// power-off from one handed out since.
pub fn boot_id() -> Option<String> {
    let id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    Some(id.trim().to_string()).filter(|id| !id.is_empty())
}

/// How long a game with a save can sit before it is overdue, in days.
pub const OVERDUE_DAYS: i64 = 14;

/// Whether a game last played at `last` is overdue at `now`: more than
/// two weeks since, with a clock that is not behind the last play.
pub fn is_overdue(last: Option<i64>, now: i64) -> bool {
    last.is_some_and(|l| l > 0 && now - l > OVERDUE_DAYS * 86_400)
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
    fn a_game_is_overdue_after_two_weeks() {
        let now = 1_800_000_000;
        let days = |d: i64| Some(now - d * 86_400);
        assert!(!is_overdue(days(13), now));
        assert!(!is_overdue(days(14), now));
        assert!(is_overdue(days(15), now));
        assert!(!is_overdue(None, now));
        assert!(!is_overdue(Some(now + 100), now), "clock behind the last play");
    }

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
    fn a_session_is_read_once_and_an_unbelievable_clock_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let rom = Path::new("/Roms/MD/Sonic.md");
        let mut times = PlayTimes::new();
        start_playing(dir.path(), rom, 1_000, 1_760_000_000, Some("b-1")).unwrap();
        assert_eq!(
            finish_playing(dir.path(), 1_600),
            Some(Session {
                rom: rom.to_path_buf(),
                secs: Some(600),
                since: Some(1_760_000_000),
                boot: Some("b-1".to_string()),
            })
        );
        assert_eq!(finish_playing(dir.path(), 9_000), None, "read twice");
        start_playing(dir.path(), rom, 5_000, 0, None).unwrap();
        let restarted = finish_playing(dir.path(), 100).unwrap();
        assert_eq!((restarted.secs, restarted.boot), (None, None), "the Brick restarted");
        start_playing(dir.path(), rom, 0, 0, None).unwrap();
        assert_eq!(finish_playing(dir.path(), PLAY_SESSION_MAX + 1).unwrap().secs, None);
        // What older Shelfs, and the pak resuming a game, leave behind.
        let playing = dir.path().join(CONFIG_DIR).join(PLAYING_FILE);
        std::fs::write(&playing, "1000\t/Roms/MD/Sonic.md\n").unwrap();
        let old = finish_playing(dir.path(), 1_300).unwrap();
        assert_eq!((old.rom.as_path(), old.secs, old.since), (rom, Some(300), None));
        std::fs::write(&playing, "1000\t1760000000\t/Roms/MD/Sonic.md\n").unwrap();
        assert_eq!(finish_playing(dir.path(), 1_300).unwrap().since, Some(1_760_000_000));
        std::fs::write(&playing, "-\t1760000000\tb-2\t/Roms/MD/Sonic.md\n").unwrap();
        let resumed = finish_playing(dir.path(), 1_300).unwrap();
        assert_eq!((resumed.secs, resumed.boot.as_deref()), (None, Some("b-2")));
        times.insert(rom.to_path_buf(), 600);
        write_play_times(dir.path(), &times).unwrap();
        assert_eq!(read_play_times(dir.path()), times);
    }

    #[test]
    fn power_offs_are_read_and_bad_lines_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(CONFIG_DIR)).unwrap();
        std::fs::write(
            dir.path().join(CONFIG_DIR).join(POWER_OFFS_FILE),
            "1760000000\t/mnt/SDCARD/Roms/MD/Sonic.md\nx\t/a\n5\t\n",
        )
        .unwrap();
        assert_eq!(
            read_power_offs(dir.path()),
            vec![(PathBuf::from("/mnt/SDCARD/Roms/MD/Sonic.md"), 1_760_000_000)]
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
