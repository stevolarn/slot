//! Game picker mode: slot shows its shelf but hands the chosen game to
//! another emulator instead of running a core itself. NextUI's pak loop
//! reads the handoff file, runs the game, and starts slot again after.

use std::path::{Path, PathBuf};

pub const HANDOFF_VAR: &str = "SLOT_HANDOFF";
pub const ROMS_VAR: &str = "SLOT_ROMS";

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

pub fn write(file: &Path, line: &str) {
    match slot_store::atomic_write(file, format!("{line}\n").as_bytes()) {
        Ok(()) => eprintln!("slot: handoff: {line}"),
        Err(e) => eprintln!("slot: handoff: {}: {e}", file.display()),
    }
}
