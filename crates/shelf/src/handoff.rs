//! Handing the chosen game to NextUI: slot shows its shelf and writes the
//! path of the game picked to the handoff file, then exits. NextUI's pak
//! loop reads the file, runs the game in its own emulator, and starts slot
//! again after.

use std::path::{Path, PathBuf};

pub const HANDOFF_VAR: &str = "SLOT_HANDOFF";
pub const ROMS_VAR: &str = "SLOT_ROMS";
/// A NextUI collection list, or a folder of them: when set, only the games
/// they name are shelved.
pub const ONLY_VAR: &str = "SLOT_ONLY";

pub const POWER_OFF: &str = "!poweroff";
pub const REBOOT: &str = "!reboot";

pub fn file() -> Option<PathBuf> {
    std::env::var_os(HANDOFF_VAR)
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
        Ok(()) => eprintln!("slot: handoff: {line}"),
        Err(e) => eprintln!("slot: handoff: {}: {e}", file.display()),
    }
}
