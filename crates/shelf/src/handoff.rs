//! Handing the chosen game to NextUI: Shelf shows its shelf and writes the
//! path of the game picked to the handoff file, then exits. NextUI's pak
//! loop reads the file, runs the game in its own emulator, and starts Shelf
//! again after.

use std::path::{Path, PathBuf};

pub const HANDOFF_VAR: &str = "SHELF_HANDOFF";
pub const ROMS_VAR: &str = "SHELF_ROMS";
pub const SHARED_USERDATA_VAR: &str = "SHARED_USERDATA_PATH";
/// A NextUI collection list, or a folder of them: when set, only the games
/// they name are shelved.
pub const ONLY_VAR: &str = "SHELF_ONLY";

pub const POWER_OFF: &str = "!poweroff";
pub const REBOOT: &str = "!reboot";
/// The second line of a game's handoff when its save state should load.
pub const RESUME: &str = "resume";

/// The text handed off for a game: its path, then a second line `resume`
/// when NextUI should continue from the game's last save state.
pub fn rom_line(rom: &Path, resume: bool) -> String {
    if resume {
        format!("{}\n{RESUME}", rom.display())
    } else {
        rom.display().to_string()
    }
}

pub fn file() -> Option<PathBuf> {
    std::env::var_os(HANDOFF_VAR)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// NextUI's shared userdata folder, which keeps each game's save-state
/// slot. Set by NextUI for its paks.
pub fn shared_userdata() -> Option<PathBuf> {
    std::env::var_os(SHARED_USERDATA_VAR)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn roms() -> Option<PathBuf> {
    std::env::var_os(ROMS_VAR)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn only() -> Option<PathBuf> {
    std::env::var_os(ONLY_VAR)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn write(file: &Path, line: &str) {
    match shelf_store::atomic_write(file, format!("{line}\n").as_bytes()) {
        Ok(()) => eprintln!("shelf: handoff: {line}"),
        Err(e) => eprintln!("shelf: handoff: {}: {e}", file.display()),
    }
}

/// NextUI's settings file in its shared userdata folder.
pub const NEXTUI_SETTINGS_FILE: &str = "minuisettings.txt";

/// NextUI's screen timeout, from its settings in `shared`: how long it
/// waits with no button pressed before it sleeps. None when it is set to
/// never, or not set.
pub fn screen_timeout(shared: &Path) -> Option<u64> {
    let text = std::fs::read_to_string(shared.join(NEXTUI_SETTINGS_FILE)).ok()?;
    screen_timeout_in(&text)
}

/// The screen timeout in the text of NextUI's settings file.
pub fn screen_timeout_in(text: &str) -> Option<u64> {
    let secs: u64 = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("screentimeout="))?
        .trim()
        .parse()
        .ok()?;
    (secs > 0).then_some(secs * 1000)
}
