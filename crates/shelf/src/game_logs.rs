//! NextUI's own record of games played, `game_logs.sqlite` in its shared
//! userdata folder: one row for each time a game was started from NextUI.
//! Games played before Shelf took over were only counted there, so the
//! rental tags add these plays to Shelf's own.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use shelf_store::{PlayTimes, Rental, Rentals};

pub const GAME_LOGS_FILE: &str = "game_logs.sqlite";

/// The plays NextUI logged in `shared/game_logs.sqlite`, by rom path under
/// `roms`: how many times each game was started and when last. Empty if
/// there is no log or it will not read; the log is only ever read.
pub fn nextui_plays(shared: &Path, roms: &Path) -> Rentals {
    read(&shared.join(GAME_LOGS_FILE), roms).unwrap_or_else(|e| {
        eprintln!("shelf: game logs: {e}");
        Rentals::new()
    })
}

fn read(db: &Path, roms: &Path) -> rusqlite::Result<Rentals> {
    let mut out = Rentals::new();
    if !db.exists() {
        return Ok(out);
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(db, flags)?;
    let mut rows = conn.prepare(
        "SELECT r.file_path, COUNT(*), MAX(p.created_at) \
         FROM rom r JOIN play_activity p ON p.rom_id = r.id GROUP BY r.id",
    )?;
    let plays = rows.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;
    for (path, times, last) in plays.flatten() {
        let path = path.trim_start_matches('/');
        if path.is_empty() || times <= 0 {
            continue;
        }
        let r = out.entry(roms.join(path)).or_insert(Rental { times: 0, last: 0 });
        r.times += times as u32;
        r.last = r.last.max(last.unwrap_or(0));
    }
    Ok(out)
}

/// The seconds NextUI logged each game being played for, by rom path under
/// `roms`. Empty if there is no log or it will not read.
pub fn nextui_play_times(shared: &Path, roms: &Path) -> PlayTimes {
    read_times(&shared.join(GAME_LOGS_FILE), roms).unwrap_or_else(|e| {
        eprintln!("shelf: game logs: {e}");
        PlayTimes::new()
    })
}

fn read_times(db: &Path, roms: &Path) -> rusqlite::Result<PlayTimes> {
    let mut out = PlayTimes::new();
    if !db.exists() {
        return Ok(out);
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(db, flags)?;
    let mut rows = conn.prepare(
        "SELECT r.file_path, SUM(p.play_time) \
         FROM rom r JOIN play_activity p ON p.rom_id = r.id GROUP BY r.id",
    )?;
    let times = rows.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;
    for (path, secs) in times.flatten() {
        let path = path.trim_start_matches('/');
        let secs = secs.unwrap_or(0);
        if path.is_empty() || secs <= 0 {
            continue;
        }
        *out.entry(roms.join(path)).or_insert(0) += secs as u64;
    }
    Ok(out)
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

    #[test]
    fn plays_are_counted_per_game_under_the_roms_folder() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join(GAME_LOGS_FILE);
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE rom(id INTEGER PRIMARY KEY, type TEXT, name TEXT, file_path TEXT, \
               image_path TEXT, created_at INTEGER, updated_at INTEGER);
             CREATE TABLE play_activity(rom_id INTEGER, play_time INTEGER, created_at INTEGER, \
               updated_at INTEGER);
             INSERT INTO rom(id, file_path) VALUES (1, 'Sega Genesis (MD)/Sonic.md'), \
               (2, 'Game Boy (GB)/Tetris.zip');
             INSERT INTO play_activity VALUES (1, 60, 100, 160), (1, 30, 300, 330), \
               (2, 5, 200, 205);",
        )
        .unwrap();
        drop(conn);
        let roms = Path::new("/mnt/SDCARD/Roms");
        let plays = nextui_plays(dir.path(), roms);
        assert_eq!(
            plays.get(&roms.join("Sega Genesis (MD)/Sonic.md")),
            Some(&Rental { times: 2, last: 300 })
        );
        assert_eq!(plays.get(&roms.join("Game Boy (GB)/Tetris.zip")).map(|r| r.times), Some(1));
        assert!(nextui_plays(&dir.path().join("none"), roms).is_empty());
        let times = nextui_play_times(dir.path(), roms);
        assert_eq!(times.get(&roms.join("Sega Genesis (MD)/Sonic.md")), Some(&90));
        assert_eq!(times.get(&roms.join("Game Boy (GB)/Tetris.zip")), Some(&5));
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
