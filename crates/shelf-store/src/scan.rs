use std::collections::{HashMap, HashSet};
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
            Platform::Gb | Platform::Gbc => {
                // One read of the header serves the title now and the cart's
                // class and shell later.
                let title = match crate::gb::header_cached(&self.rom) {
                    Some(h) => Some(h.title).filter(|t| !t.is_empty()),
                    None => crate::gb::title(&self.rom),
                };
                if let Some(title) = title {
                    self.title = title;
                }
            }
            Platform::Gg | Platform::Nes | Platform::Sms | Platform::Md | Platform::Snes => {}
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
/// Only `roms` folders show all the platforms, and zip or 7z archives, since
/// those games are handed to NextUI's emulators. There a cart with no label
/// of its own takes NextUI's box art from the folder's `.media/`.
pub fn scan_in(root: &Path, roms: Option<&Path>) -> Result<Vec<Cart>, StoreError> {
    Ok(scan_platforms(root, roms, &Platform::ALL))
}

/// `scan_in` for just these platforms.
fn scan_platforms(root: &Path, roms: Option<&Path>, platforms: &[Platform]) -> Vec<Cart> {
    let shelving = Shelving::new(root);
    let mut carts = Vec::new();
    let mut tagged = roms.map(tagged_dirs).unwrap_or_default();
    for &platform in platforms {
        let dirs = match roms {
            Some(_) => tagged.remove(&platform).unwrap_or_default(),
            None if platform.has_games_folder() => {
                vec![root.join("Games").join(platform.dir_name())]
            }
            None => continue,
        };
        for dir in dirs {
            let entries = match std::fs::read_dir(&dir) {
                Ok(d) => d,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {
                    eprintln!("shelf: scan: {}: {e}", dir.display());
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
                carts.extend(shelving.cart(platform, rom, roms.is_some()));
            }
        }
    }
    sort_carts(&mut carts);
    carts
}

/// Carts for just the games named in NextUI collection lists (see
/// [`listed`]), wherever they sit under `roms`, subfolders included. Each
/// takes the platform of the tagged folder it is in. A console no list
/// names a game of shows all its games, so a console newly shelved is
/// not left empty.
pub fn scan_listed(root: &Path, roms: &Path, lists: &Path) -> Vec<Cart> {
    let shelving = Shelving::new(root);
    let mut folders = Folders::default();
    let mut carts: Vec<Cart> = listed(lists, roms)
        .into_iter()
        .filter(|rom| !is_hidden(rom) && folders.has_file(rom))
        .filter_map(|rom| {
            let system = rom.strip_prefix(roms).ok()?.components().next()?;
            let platform = system_platform(system.as_os_str().to_str()?)?;
            let takes = platform.accepts(&rom) || crate::is_archive(&rom);
            takes.then(|| shelving.cart(platform, rom, true)).flatten()
        })
        .collect();
    let unlisted: Vec<Platform> = Platform::ALL
        .into_iter()
        .filter(|p| !carts.iter().any(|c| c.platform == *p))
        .collect();
    carts.extend(scan_platforms(root, Some(roms), &unlisted));
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
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for text in files.iter().filter_map(|f| std::fs::read_to_string(f).ok()) {
        for line in text.lines().map(|l| l.trim().trim_start_matches('/')) {
            let rom = card.join(line);
            if !line.is_empty() && seen.insert(rom.clone()) {
                roms.push(rom);
            }
        }
    }
    roms
}

/// Which files each folder holds, one directory read per folder, in place
/// of a stat for every file asked about.
#[derive(Default)]
struct Folders {
    files: HashMap<PathBuf, HashSet<std::ffi::OsString>>,
}

impl Folders {
    fn has_file(&mut self, path: &Path) -> bool {
        let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
            return false;
        };
        if !self.files.contains_key(dir) {
            let names = std::fs::read_dir(dir)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(is_file)
                        .map(|e| e.file_name())
                        .collect()
                })
                .unwrap_or_default();
            self.files.insert(dir.to_path_buf(), names);
        }
        self.files[dir].contains(name)
    }
}

struct Shelving {
    root: PathBuf,
    shells: HashMap<String, crate::ShellChoice>,
    labels: HashMap<Platform, HashMap<String, String>>,
    media: std::cell::RefCell<HashMap<PathBuf, HashMap<String, String>>>,
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
            media: Default::default(),
        }
    }

    fn labels_dir(root: &Path, platform: Platform) -> PathBuf {
        root.join("Labels").join(platform.dir_name())
    }

    /// With `media`, a cart with no label of its own takes the NextUI box
    /// art in the `.media/` folder beside its rom.
    fn cart(&self, platform: Platform, rom: PathBuf, media: bool) -> Option<Cart> {
        let stem = rom.file_stem()?.to_str()?.to_string();
        let png = format!("{stem}.png").to_lowercase();
        let label = self.labels[&platform]
            .get(&png)
            .map(|name| Self::labels_dir(&self.root, platform).join(name))
            .or_else(|| {
                let dir = media.then(|| rom.parent())??.join(".media");
                let mut cache = self.media.borrow_mut();
                let art = cache.entry(dir.clone()).or_insert_with(|| listing(&dir));
                art.get(&png).map(|name| dir.join(name))
            });
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
    carts.sort_by_cached_key(|c| (c.platform as u8, sort_key(&c.stem)));
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
        "GG" => Some(Platform::Gg),
        "FC" => Some(Platform::Nes),
        "SMS" => Some(Platform::Sms),
        "MD" => Some(Platform::Md),
        "SFC" => Some(Platform::Snes),
        _ => None,
    }
}

/// The platform a NextUI system folder under Roms is shelved on. Satellaview
/// shares the SNES tag but is left off the shelves.
pub fn system_platform(name: &str) -> Option<Platform> {
    if name.to_ascii_lowercase().contains("satellaview") {
        return None;
    }
    tag_platform(folder_tag(name)?)
}

/// The folders under `roms` named for a platform, found with one read of
/// `roms`, each platform's in name order.
fn tagged_dirs(roms: &Path) -> HashMap<Platform, Vec<PathBuf>> {
    let Ok(entries) = std::fs::read_dir(roms) else {
        eprintln!("shelf: scan: no roms folder at {}", roms.display());
        return HashMap::new();
    };
    let mut dirs: HashMap<Platform, Vec<PathBuf>> = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if is_hidden(&path) {
            continue;
        }
        let Some(platform) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(system_platform)
        else {
            continue;
        };
        let is_dir = match entry.file_type() {
            Ok(t) if t.is_symlink() => path.is_dir(),
            Ok(t) => t.is_dir(),
            Err(_) => false,
        };
        if is_dir {
            dirs.entry(platform).or_default().push(path);
        }
    }
    for list in dirs.values_mut() {
        list.sort();
    }
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
