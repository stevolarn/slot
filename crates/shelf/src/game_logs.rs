//! NextUI's own record of games played, `game_logs.sqlite` in its shared
//! userdata folder: one row for each time a game was started from NextUI.
//! Games played before Shelf took over were only counted there, so the
//! rental tags add these plays to Shelf's own.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use shelf_store::{PlayTimes, Rental, Rentals};

pub const GAME_LOGS_FILE: &str = "game_logs.sqlite";

/// One stretch of play NextUI logged: a game going in, or coming back
/// from sleep, until it was stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRow {
    pub rom: PathBuf,
    /// When it started, in UTC unix seconds.
    pub at: i64,
    /// How long it lasted; `None` while still running.
    pub secs: Option<u64>,
}

/// A game Shelf handed out from `.1` to `.2` in UTC unix seconds. NextUI
/// logs these too once Shelf starts its timer, and they are not counted
/// twice: Shelf has them in its own rentals and play times.
pub type ShelfWindow = (PathBuf, i64, i64);

/// How far either side of a Shelf session a row is still taken as in it.
const WINDOW_SLACK: i64 = 5;

/// Every stretch of play in NextUI's `shared/game_logs.sqlite`, by rom
/// path under `roms`. Empty if there is no log or it will not read; the
/// log is only ever read.
pub fn nextui_rows(shared: &Path, roms: &Path) -> Vec<LogRow> {
    read(&shared.join(GAME_LOGS_FILE), roms).unwrap_or_else(|e| {
        eprintln!("shelf: game logs: {e}");
        Vec::new()
    })
}

fn read(db: &Path, roms: &Path) -> rusqlite::Result<Vec<LogRow>> {
    if !db.exists() {
        return Ok(Vec::new());
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(db, flags)?;
    let mut rows = conn.prepare(
        "SELECT r.file_path, p.created_at, p.play_time \
         FROM rom r JOIN play_activity p ON p.rom_id = r.id",
    )?;
    let found = rows.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;
    Ok(found
        .flatten()
        .filter_map(|(path, at, secs)| {
            let path = path.trim_start_matches('/');
            (!path.is_empty()).then(|| LogRow {
                rom: roms.join(path),
                at: at.unwrap_or(0),
                secs: secs.map(|s| s.max(0) as u64),
            })
        })
        .collect())
}

fn in_shelf(row: &LogRow, shelf: &[ShelfWindow]) -> bool {
    shelf.iter().any(|(rom, from, to)| {
        *rom == row.rom && row.at >= from - WINDOW_SLACK && row.at <= to + WINDOW_SLACK
    })
}

/// How many times NextUI started each game and when last, leaving out
/// what it logged of Shelf's own sessions.
pub fn nextui_plays(rows: &[LogRow], shelf: &[ShelfWindow]) -> Rentals {
    let mut out = Rentals::new();
    for row in rows.iter().filter(|r| !in_shelf(r, shelf)) {
        let r = out.entry(row.rom.clone()).or_insert(Rental { times: 0, last: 0 });
        r.times += 1;
        r.last = r.last.max(row.at);
    }
    out
}

/// The seconds NextUI logged each game being played for, leaving out
/// Shelf's own sessions.
pub fn nextui_play_times(rows: &[LogRow], shelf: &[ShelfWindow]) -> PlayTimes {
    let mut out = PlayTimes::new();
    for row in rows.iter().filter(|r| !in_shelf(r, shelf)) {
        if let Some(secs) = row.secs.filter(|s| *s > 0) {
            *out.entry(row.rom.clone()).or_insert(0) += secs;
        }
    }
    out
}

/// The seconds NextUI's timer counted for `rom` handed out from `from` to
/// `to`: the stretches between sleeps, added up. `None` if it logged none,
/// as when its timer was not started.
pub fn session_secs(rows: &[LogRow], rom: &Path, from: i64, to: i64) -> Option<u64> {
    let window = [(rom.to_path_buf(), from, to)];
    let mut found = rows.iter().filter(|r| in_shelf(r, &window)).peekable();
    found.peek()?;
    Some(found.filter_map(|r| r.secs).sum())
}

/// A game's plays counted by Shelf and by NextUI together.
pub fn combined(shelf: Option<Rental>, nextui: Option<Rental>) -> Option<Rental> {
    match (shelf, nextui) {
        (Some(a), Some(b)) => Some(Rental {
            times: a.times + b.times,
            last: a.last.max(b.last),
        }),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(dir: &Path, rows: &str) {
        let conn = Connection::open(dir.join(GAME_LOGS_FILE)).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE rom(id INTEGER PRIMARY KEY, type TEXT, name TEXT, file_path TEXT, \
               image_path TEXT, created_at INTEGER, updated_at INTEGER);
             CREATE TABLE play_activity(rom_id INTEGER, play_time INTEGER, created_at INTEGER, \
               updated_at INTEGER);
             INSERT INTO rom(id, file_path) VALUES (1, 'Sega Genesis (MD)/Sonic.md'), \
               (2, 'Game Boy (GB)/Tetris.zip');
             INSERT INTO play_activity VALUES {rows};"
        ))
        .unwrap();
    }

    #[test]
    fn plays_are_counted_per_game_under_the_roms_folder() {
        let dir = tempfile::tempdir().unwrap();
        log(dir.path(), "(1, 60, 100, 160), (1, 30, 300, 330), (2, 5, 200, 205)");
        let roms = Path::new("/mnt/SDCARD/Roms");
        let rows = nextui_rows(dir.path(), roms);
        let plays = nextui_plays(&rows, &[]);
        assert_eq!(
            plays.get(&roms.join("Sega Genesis (MD)/Sonic.md")),
            Some(&Rental { times: 2, last: 300 })
        );
        assert_eq!(plays.get(&roms.join("Game Boy (GB)/Tetris.zip")).map(|r| r.times), Some(1));
        assert!(nextui_rows(&dir.path().join("none"), roms).is_empty());
        let times = nextui_play_times(&rows, &[]);
        assert_eq!(times.get(&roms.join("Sega Genesis (MD)/Sonic.md")), Some(&90));
        assert_eq!(times.get(&roms.join("Game Boy (GB)/Tetris.zip")), Some(&5));
    }

    #[test]
    fn a_shelf_session_is_its_stretches_between_sleeps_and_is_not_counted_twice() {
        let dir = tempfile::tempdir().unwrap();
        // NextUI's own play of Sonic, then Shelf's: in at 1000, asleep from
        // 1100 to 1700, back at Shelf at 1750.
        log(dir.path(), "(1, 60, 100, 160), (1, 100, 1001, 1101), (1, 50, 1700, 1750)");
        let roms = Path::new("/mnt/SDCARD/Roms");
        let sonic = roms.join("Sega Genesis (MD)/Sonic.md");
        let rows = nextui_rows(dir.path(), roms);
        assert_eq!(session_secs(&rows, &sonic, 1000, 1750), Some(150));
        assert_eq!(session_secs(&rows, &roms.join("Game Boy (GB)/Tetris.zip"), 1000, 1750), None);
        let shelf = [(sonic.clone(), 1000, 1750)];
        assert_eq!(nextui_plays(&rows, &shelf)[&sonic], Rental { times: 1, last: 100 });
        assert_eq!(nextui_play_times(&rows, &shelf)[&sonic], 60);
    }

    #[test]
    fn shelf_and_nextui_plays_add_up() {
        let a = Rental { times: 2, last: 500 };
        let b = Rental { times: 3, last: 100 };
        assert_eq!(combined(Some(a), Some(b)), Some(Rental { times: 5, last: 500 }));
        assert_eq!(combined(None, Some(b)), Some(b));
        assert_eq!(combined(None, None), None);
    }
}
