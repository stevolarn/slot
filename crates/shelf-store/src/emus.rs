//! Which emulator pak runs a game, kept in `Config/emus.txt` as
//! `<pak tag>\t<rom path>` lines. A game with no line runs on the pak its
//! Roms folder is tagged for, as NextUI's own menu would run it; the pak
//! `launch.sh` looks the tag up under `Emus/<platform>/<tag>.pak` and in
//! the system's own paks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::config::CONFIG_DIR;
use crate::platform::Platform;

pub const EMUS_FILE: &str = "emus.txt";

/// An emulator pak a game of a console can be run on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Emu {
    /// The pak's name without `.pak`, as NextUI tags the Roms folders.
    pub tag: &'static str,
    pub name: &'static str,
}

const fn emu(tag: &'static str, name: &'static str) -> Emu {
    Emu { tag, name }
}

const GBA: [Emu; 2] = [emu("GBA", "gpSP"), emu("MGBA", "mGBA")];
const GB: [Emu; 2] = [emu("GB", "Gambatte"), emu("SGB", "Super GB")];
const GBC: [Emu; 2] = [emu("GBC", "Gambatte"), emu("SGB", "Super GB")];
const SNES: [Emu; 2] = [emu("SFC", "Snes9x"), emu("SUPA", "Supafaust")];
const PCE: [Emu; 2] = [emu("PCE", "PCE"), emu("SGX", "SuperGrafx")];

/// The paks a console's games can run on, NextUI's usual one first. A
/// console with one pak (or none known) has no choice to make.
pub fn emus_for(platform: Platform) -> &'static [Emu] {
    match platform {
        Platform::Gba => &GBA,
        Platform::Gb => &GB,
        Platform::Gbc => &GBC,
        Platform::Snes => &SNES,
        Platform::Pce => &PCE,
        _ => &[],
    }
}

pub fn emus_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(EMUS_FILE)
}

/// The pak tag chosen for each game, by rom path.
pub fn read_emus(root: &Path) -> HashMap<PathBuf, String> {
    let text = std::fs::read_to_string(emus_path(root)).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let (tag, rom) = line.split_once('\t')?;
            (!tag.is_empty() && !rom.is_empty()).then(|| (PathBuf::from(rom), tag.to_string()))
        })
        .collect()
}

pub fn write_emus(root: &Path, emus: &HashMap<PathBuf, String>) -> std::io::Result<()> {
    let mut lines: Vec<String> = emus
        .iter()
        .map(|(rom, tag)| format!("{tag}\t{}\n", rom.display()))
        .collect();
    lines.sort();
    let path = emus_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, lines.concat().as_bytes())
}

/// The pak a game's Roms folder is tagged for: the system folder directly
/// under `roms`, even for a game in a subfolder of it.
pub fn folder_pak(roms: &Path, rom: &Path) -> Option<String> {
    let rel = rom.strip_prefix(roms).ok()?;
    let folder = rel.components().next()?.as_os_str().to_str()?;
    crate::scan::folder_tag(folder).map(str::to_string)
}

/// The next pak after `current` among `choices`, wrapping round, or the
/// first when `current` is not among them.
pub fn next_tag<'a>(choices: &[&'a str], current: &str, forward: bool) -> Option<&'a str> {
    if choices.is_empty() {
        return None;
    }
    let at = choices.iter().position(|t| t.eq_ignore_ascii_case(current));
    let n = choices.len();
    let to = match (at, forward) {
        (Some(i), true) => (i + 1) % n,
        (Some(i), false) => (i + n - 1) % n,
        (None, _) => 0,
    };
    Some(choices[to])
}
