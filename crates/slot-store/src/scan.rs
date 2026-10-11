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
        match self.platform {
            Platform::Gba => {
                if let Some((title, code)) = header_title_code(&self.rom) {
                    self.title = title;
                    self.code = code;
                }
            }
            Platform::Gb | Platform::Gbc => {
                if let Some(title) = crate::gb::title(&self.rom) {
                    self.title = title;
                }
            }
            Platform::Nes | Platform::Md | Platform::Snes => {}
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
/// Only `roms` folders show platforms slot has no core for, and zip or 7z
/// archives, since those games are handed to another emulator.
pub fn scan_in(root: &Path, roms: Option<&Path>) -> Result<Vec<Cart>, StoreError> {
    let shelving = Shelving::new(root);
    let mut carts = Vec::new();
    for platform in Platform::ALL {
        let dirs = match roms {
            Some(roms) => tagged_dirs(roms, platform),
            None if platform.built_in() => vec![root.join("Games").join(platform.dir_name())],
            None => continue,
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
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                let rom = entry.path();
                let takes = platform.accepts(&rom) || (roms.is_some() && crate::is_archive(&rom));
                if is_hidden(&rom) || !takes || !is_file(&entry) {
                    continue;
                }
                carts.extend(shelving.cart(platform, rom));
            }
        }
    }
    sort_carts(&mut carts);
    Ok(carts)
}

/// Carts for just the games named in NextUI collection lists (see
/// [`listed`]), wherever they sit under `roms`, subfolders included. Each
/// takes the platform of the tagged folder it is in.
pub fn scan_listed(root: &Path, roms: &Path, lists: &Path) -> Vec<Cart> {
    let shelving = Shelving::new(root);
    let mut carts: Vec<Cart> = listed(lists, roms)
        .into_iter()
        .filter(|rom| rom.is_file() && !is_hidden(rom))
        .filter_map(|rom| {
            let system = rom.strip_prefix(roms).ok()?.components().next()?;
            let platform = tag_platform(folder_tag(system.as_os_str().to_str()?)?)?;
            let takes = platform.accepts(&rom) || crate::is_archive(&rom);
            takes.then(|| shelving.cart(platform, rom)).flatten()
        })
        .collect();
    sort_carts(&mut carts);
    carts
}

/// The roms named in NextUI collection lists: every `.txt` in `lists` (or
/// `lists` itself when it is a file), one path per line, written from the
/// root of the SD card that holds `roms`, eg `/Roms/Game Boy (GB)/Tetris.zip`.
pub fn listed(lists: &Path, roms: &Path) -> Vec<PathBuf> {
    let card = roms.parent().unwrap_or(roms);
    let mut files: Vec<PathBuf> = if lists.is_file() {
        vec![lists.to_path_buf()]
    } else {
        std::fs::read_dir(lists)
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect())
            .unwrap_or_default()
    };
    files.retain(|p| !is_hidden(p) && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")));
    files.sort();
    let mut roms: Vec<PathBuf> = Vec::new();
    for text in files.iter().filter_map(|f| std::fs::read_to_string(f).ok()) {
        for line in text.lines().map(|l| l.trim().trim_start_matches('/')) {
            let rom = card.join(line);
            if !line.is_empty() && !roms.contains(&rom) {
                roms.push(rom);
            }
        }
    }
    roms
}

struct Shelving {
    root: PathBuf,
    shells: HashMap<String, crate::ShellChoice>,
    labels: HashMap<Platform, HashMap<String, String>>,
}

impl Shelving {
    fn new(root: &Path) -> Shelving {
        let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
        Shelving {
            root: root.to_path_buf(),
            shells: crate::cart_shell::layered(
                &read(crate::CART_SHELL_FILE),
                &read(crate::LABELS_SHELL_FILE),
            ),
            labels: Platform::ALL
                .into_iter()
                .map(|p| (p, listing(&Self::labels_dir(root, p))))
                .collect(),
        }
    }

    fn labels_dir(root: &Path, platform: Platform) -> PathBuf {
        root.join("Labels").join(platform.dir_name())
    }

    fn cart(&self, platform: Platform, rom: PathBuf) -> Option<Cart> {
        let stem = rom.file_stem()?.to_str()?.to_string();
        let label = self.labels[&platform]
            .get(&format!("{stem}.png").to_lowercase())
            .map(|name| Self::labels_dir(&self.root, platform).join(name));
        Some(Cart {
            platform,
            shell: self.shells.get(&crate::cart_shell::key(&stem)).copied(),
            stem,
            title: String::new(),
            code: String::new(),
            label,
            rom,
        })
    }
}

fn sort_carts(carts: &mut [Cart]) {
    carts.sort_by(|a, b| {
        (a.platform as u8, sort_key(&a.stem)).cmp(&(b.platform as u8, sort_key(&b.stem)))
    });
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
        "FC" => Some(Platform::Nes),
        "MD" => Some(Platform::Md),
        "SFC" => Some(Platform::Snes),
        _ => None,
    }
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
