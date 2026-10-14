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
            Platform::Gg | Platform::Nes | Platform::Sms | Platform::Md
            | Platform::Snes
            | Platform::N64
            | Platform::Pce
            | Platform::PceCd
            | Platform::SegaCd
            | Platform::Ps
            | Platform::Ngp
            | Platform::Ngpc
            | Platform::Other(_) => {}
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
    Ok(scan_platforms(root, roms, &Platform::ALL, Some(&[])))
}

/// `scan_in` for just these platforms, and with `others` set, the generic
/// systems in `roms` too, bar those it lists.
fn scan_platforms(
    root: &Path,
    roms: Option<&Path>,
    platforms: &[Platform],
    others: Option<&[Platform]>,
) -> Vec<Cart> {
    let mut carts = Vec::new();
    let mut tagged = roms.map(tagged_dirs).unwrap_or_default();
    let mut platforms = platforms.to_vec();
    if let Some(skip) = others {
        let mut extra: Vec<Platform> = tagged
            .keys()
            .copied()
            .filter(|p| p.is_other() && !skip.contains(p))
            .collect();
        extra.sort_by_key(|p| p.order());
        platforms.extend(extra);
    }
    let shelving = Shelving::new(root, &platforms);
    for &platform in &platforms {
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
    let mut carts = thin_tracks(thin_discs(carts));
    sort_carts(&mut carts);
    carts
}

/// Carts for just the games named in NextUI collection lists (see
/// [`listed`]), wherever they sit under `roms`, subfolders included. Each
/// takes the platform of the tagged folder it is in. A console no list
/// names a game of shows all its games, so a console newly shelved is
/// not left empty.
pub fn scan_listed(root: &Path, roms: &Path, lists: &Path) -> Vec<Cart> {
    let shelving = Shelving::new(root, &Platform::all());
    let mut folders = Folders::default();
    let mut carts: Vec<Cart> = listed(lists, roms)
        .into_iter()
        .filter(|rom| !is_hidden(rom) && folders.has_file(rom))
        .filter_map(|rom| {
            let system = rom.strip_prefix(roms).ok()?.components().next()?;
            let platform = shelved_platform(system.as_os_str().to_str()?, None)?;
            let takes = platform.accepts(&rom) || crate::is_archive(&rom);
            takes.then(|| shelving.cart(platform, rom, true)).flatten()
        })
        .collect();
    let named: Vec<Platform> = carts.iter().map(|c| c.platform).collect();
    let unlisted: Vec<Platform> = Platform::ALL
        .into_iter()
        .filter(|p| !named.contains(p))
        .collect();
    carts.extend(scan_platforms(root, Some(roms), &unlisted, Some(&named)));
    let mut carts = thin_tracks(thin_discs(carts));
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
    fn new(root: &Path, platforms: &[Platform]) -> Shelving {
        let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
        Shelving {
            root: root.to_path_buf(),
            shells: crate::cart_shell::layered(
                &read(crate::CART_SHELL_FILE),
                &read(crate::LABELS_SHELL_FILE),
            ),
            labels: platforms
                .iter()
                .map(|&p| (p, listing(&Self::labels_dir(root, p))))
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
        let label = self
            .labels
            .get(&platform)
            .and_then(|l| l.get(&png))
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

/// The disc number in a stem's `(Disc N)` tag, and the stem with the tag
/// taken out (and the space before it).
fn disc_tag(stem: &str) -> Option<(u32, String)> {
    let at = stem.find("(Disc ")?;
    let rest = &stem[at + 6..];
    let end = rest.find(')')?;
    let n: u32 = rest[..end].trim().parse().ok()?;
    let bare = format!("{}{}", stem[..at].trim_end(), &rest[end + 1..]);
    Some((n, bare.trim().to_string()))
}

/// The files an `.m3u` playlist names, as paths beside the playlist: blank
/// lines and `#` comments skipped. None when it can't be read.
fn playlist_files(m3u: &Path) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(m3u).ok()?;
    let dir = m3u.parent()?;
    Some(
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| dir.join(l.replace('\\', "/")))
            .collect(),
    )
}

/// A multi-disc game shelved once: a playlist hides the discs it lists, a
/// playlist beats a loose `(Disc 1)` of the same game, and a loose
/// `(Disc 2+)` goes when its `(Disc 1)` is beside it.
/// Drops the data files of a generic system's disc image: a `.bin` (or
/// `.img`, `.sub`) next to a `.cue` of the same name is the cue's, not a game.
fn thin_tracks(carts: Vec<Cart>) -> Vec<Cart> {
    let ext = |c: &Cart| {
        c.rom
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default()
    };
    let dir_of = |c: &Cart| c.rom.parent().map(Path::to_path_buf).unwrap_or_default();
    let cues: HashSet<(PathBuf, String)> = carts
        .iter()
        .filter(|c| c.platform.is_other() && ext(c) == "cue")
        .map(|c| (dir_of(c), c.stem.clone()))
        .collect();
    carts
        .into_iter()
        .filter(|c| {
            !(c.platform.is_other()
                && matches!(ext(c).as_str(), "bin" | "img" | "sub")
                && cues.contains(&(dir_of(c), c.stem.clone())))
        })
        .collect()
}

fn thin_discs(carts: Vec<Cart>) -> Vec<Cart> {
    let is_m3u = |c: &Cart| {
        c.rom.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3u"))
    };
    let dir_of = |c: &Cart| c.rom.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut listed: HashSet<PathBuf> = HashSet::new();
    let mut playlists: HashSet<(PathBuf, String)> = HashSet::new();
    let mut disc_ones: HashSet<(PathBuf, String)> = HashSet::new();
    for c in &carts {
        if is_m3u(c) {
            listed.extend(playlist_files(&c.rom).unwrap_or_default());
            playlists.insert((dir_of(c), c.stem.clone()));
        } else if let Some((1, bare)) = disc_tag(&c.stem) {
            disc_ones.insert((dir_of(c), bare));
        }
    }
    carts
        .into_iter()
        .filter(|c| {
            if is_m3u(c) {
                return true;
            }
            if listed.contains(&c.rom) {
                return false;
            }
            match disc_tag(&c.stem) {
                Some((n, bare)) => {
                    let key = (dir_of(c), bare);
                    !playlists.contains(&key) && (n == 1 || !disc_ones.contains(&key))
                }
                None => true,
            }
        })
        .collect()
}

fn sort_carts(carts: &mut [Cart]) {
    carts.sort_by_cached_key(|c| (c.platform.order(), sort_key(&c.stem)));
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
        "N64" => Some(Platform::N64),
        "PCE" => Some(Platform::Pce),
        "PCECD" => Some(Platform::PceCd),
        "SEGACD" => Some(Platform::SegaCd),
        "PS" => Some(Platform::Ps),
        "NGP" => Some(Platform::Ngp),
        "NGPC" => Some(Platform::Ngpc),
        _ => None,
    }
}

/// The platform a NextUI system folder under Roms is shelved on. Satellaview
/// shares the SNES tag and is left off the shelves. TurboGrafx-CD shares the
/// PCE tag but has a shelf of its own.
pub fn system_platform(name: &str) -> Option<Platform> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("satellaview") {
        return None;
    }
    if lower.contains("turbografx-cd") || lower.contains("pc engine cd") {
        return Some(Platform::PceCd);
    }
    tag_platform(folder_tag(name)?)
}

/// The platform a system folder is shelved on: its own, or for a tagged
/// folder with no look of its own (Lynx, Atari 2600...) the generic one,
/// when a game is in it and its emulator pak is installed. Folders whose
/// name starts with `_` (collections, links) are never shelved.
fn shelved_platform(name: &str, dir: Option<&Path>) -> Option<Platform> {
    if let Some(p) = system_platform(name) {
        return Some(p);
    }
    let lower = name.to_ascii_lowercase();
    if name.starts_with('_') || lower.contains("satellaview") {
        return None;
    }
    let tag = folder_tag(name)?;
    let title = name[..name.rfind('(')?].trim();
    if tag.is_empty() || title.is_empty() || !pak_installed(tag) {
        return None;
    }
    if dir.is_some_and(|d| !has_games(d)) {
        return None;
    }
    Platform::other(tag, title)
}

/// Whether the emulator pak for a folder tag is installed, going by the
/// `SHELF_EMU_DIRS` folders `launch.sh` sets. With none set (a laptop
/// preview) every tag counts.
fn pak_installed(tag: &str) -> bool {
    let Some(dirs) = std::env::var_os("SHELF_EMU_DIRS").filter(|v| !v.is_empty()) else {
        return true;
    };
    std::env::split_paths(&dirs).any(|d| {
        [tag.to_string(), tag.to_ascii_uppercase()]
            .iter()
            .any(|t| d.join(format!("{t}.pak")).join("launch.sh").is_file())
    })
}

/// Whether a folder holds a file a generic platform takes, so an empty
/// system folder gets no shelf.
fn has_games(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|d| {
        d.flatten()
            .any(|e| is_file(&e) && !is_hidden(&e.path()) && crate::platform::generic_game(&e.path()))
    })
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
        let is_dir = match entry.file_type() {
            Ok(t) if t.is_symlink() => path.is_dir(),
            Ok(t) => t.is_dir(),
            Err(_) => false,
        };
        if !is_dir {
            continue;
        }
        let Some(platform) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| shelved_platform(n, Some(&path)))
        else {
            continue;
        };
        dirs.entry(platform).or_default().push(path);
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
