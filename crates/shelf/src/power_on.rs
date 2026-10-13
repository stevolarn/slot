//! Opening at power-on: the boot hook (written by `launch.sh`) starts Shelf
//! ahead of NextUI's menu when the "Open at Power-On" setting is on, with
//! `SHELF_BOOT` set. Two things send the Brick on to NextUI's menu instead:
//! Select or Menu held while it starts, and two starts in a row that never
//! got as far as drawing a frame.

use std::path::{Path, PathBuf};

use shelf_input::Btn;
use shelf_store::CONFIG_DIR;

/// Set by the boot hook for a start at power-on.
pub const BOOT_VAR: &str = "SHELF_BOOT";

/// The hook counts each start here before running Shelf; Shelf clears it
/// once it has drawn, and the hook gives up after two left standing.
pub const TRIES_FILE: &str = "boot-tries";

pub fn at_power_on() -> bool {
    std::env::var_os(BOOT_VAR).is_some_and(|v| !v.is_empty())
}

pub fn tries_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(TRIES_FILE)
}

/// Forget the starts counted against Shelf: it is up, or was left on purpose.
pub fn clear_tries(root: &Path) {
    let _ = std::fs::remove_file(tries_path(root));
}

/// Whether the buttons held as Shelf starts ask for NextUI's menu.
pub fn skips_shelf(held: &[Btn]) -> bool {
    held.iter().any(|b| matches!(b, Btn::Select | Btn::Menu))
}
