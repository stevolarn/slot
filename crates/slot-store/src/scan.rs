use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::gba::header_title_code;
use crate::platform::Platform;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cart {
    pub platform: Platform,
    pub stem: String,
    pub rom: PathBuf,
    pub label: Option<PathBuf>,
    pub title: String,
    pub code: String,
    pub shell: Option<crate::ShellChoice>,
}

impl Cart {
    pub fn read_header(&mut self) {
        if !self.platform.accepts(&self.rom) {
            return;
        }
        match self.platform {
            Platform::Gba => {
                if let Some((title, code)) = header_title_code(&self.rom) {
                    self.title = title;
                    self.code = code;
                }
            }
            _ => {
                if let Some(title) = crate::gb::title(&self.rom) {
                    self.title = title;
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

pub fn scan(root: &Path) -> Result<Vec<Cart>, StoreError> {
    scan_in(root, None)
}

/// Scan for carts. With `roms` set, games come from that folder's
/// NextUI-style subfolders, picked by the tag in brackets at the end of
/// their name, eg `Game Boy Advance (GBA)`, instead of `Games/` under root.
/// Those games may be zipped, since NextUI opens them, and a cart with no
/// label of its own takes NextUI's box art from the folder's `.media/`.
pub fn scan_in(root: &Path, roms: Option<&Path>) -> Result<Vec<Cart>, StoreError> {
    let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
    let shells = crate::cart_shell::layered(
        &read(crate::CART_SHELL_FILE),
        &read(crate::LABELS_SHELL_FILE),
    );
    let mut carts = Vec::new();
    for platform in Platform::ALL {
        let labels_dir = root.join("Labels").join(platform.dir_name());
        let labels = listing(&labels_dir);
        let dirs = match roms {
            Some(roms) => tagged_dirs(roms, platform),
            None => vec![root.join("Games").join(platform.dir_name())],
        };
        for dir in dirs {
            let entries = match std::fs::read_dir(&dir) {
                Ok(d) => d,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    eprintln!("slot: scan: {}: {e}", dir.display());
                    continue;
                }
            };
            let media_dir = dir.join(".media");
            let media = match roms {
                Some(_) => listing(&media_dir),
                None => HashMap::new(),
            };
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                let rom = entry.path();
                let takes = platform.accepts(&rom) || (roms.is_some() && is_archive(&rom));
                if is_hidden(&rom) || !takes || !is_file(&entry) {
                    continue;
                }
                let Some(stem) = rom.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let png = format!("{stem}.png").to_lowercase();
                let label = labels
                    .get(&png)
                    .map(|name| labels_dir.join(name))
                    .or_else(|| media.get(&png).map(|name| media_dir.join(name)));
                carts.push(Cart {
                    platform,
                    stem: stem.to_string(),
                    title: String::new(),
                    code: String::new(),
                    shell: shells.get(&crate::cart_shell::key(stem)).copied(),
                    label,
                    rom,
                });
            }
        }
    }
    carts.sort_by(|a, b| {
        (a.platform as u8, sort_key(&a.stem)).cmp(&(b.platform as u8, sort_key(&b.stem)))
    });
    Ok(carts)
}

pub fn folder_tag(name: &str) -> Option<&str> {
    let open = name.rfind('(')?;
    name[open + 1..].strip_suffix(')')
}

pub fn tag_platform(tag: &str) -> Option<Platform> {
    match tag.to_ascii_uppercase().as_str() {
        "GBA" | "MGBA" => Some(Platform::Gba),
        "GB" => Some(Platform::Gb),
        "GBC" => Some(Platform::Gbc),
        _ => None,
    }
}

fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("7z"))
}

fn tagged_dirs(roms: &Path, platform: Platform) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(roms) else {
        eprintln!("slot: scan: no roms folder at {}", roms.display());
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !is_hidden(p))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(folder_tag)
                .and_then(tag_platform)
                == Some(platform)
        })
        .collect();
    dirs.sort();
    dirs
}

pub fn listing(dir: &Path) -> HashMap<String, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .map(|name| (name.to_lowercase(), name))
        .collect()
}

fn is_file(entry: &std::fs::DirEntry) -> bool {
    match entry.file_type() {
        Ok(t) if t.is_symlink() => entry.path().is_file(),
        Ok(t) => t.is_file(),
        Err(_) => false,
    }
}

pub fn sort_key(stem: &str) -> (u8, String) {
    (group_of(stem), stem.to_uppercase())
}

fn group_of(stem: &str) -> u8 {
    match stem.chars().find(|c| !c.is_whitespace()) {
        Some(c) if c.is_ascii_digit() => 0,
        Some(c) if c.is_alphabetic() => 1,
        _ => 2,
    }
}

pub fn initial(stem: &str) -> char {
    match group_of(stem) {
        1 => stem
            .chars()
            .find(|c| !c.is_whitespace())
            .and_then(|c| c.to_uppercase().next())
            .unwrap_or('#'),
        _ => '#',
    }
}

pub fn is_hidden(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}
