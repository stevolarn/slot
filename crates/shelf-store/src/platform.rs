use std::path::Path;
use std::sync::Mutex;

/// Archive formats NextUI's emulators open, so picker mode shelves them for
/// any platform.
pub const ARCHIVES: [&str; 2] = ["zip", "7z"];

/// Whether `path` is a zip or 7z archive rather than a bare rom, so it has
/// no header to read.
pub fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ARCHIVES.iter().any(|k| ext.eq_ignore_ascii_case(k)))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    Gg,
    Nes,
    Sms,
    Md,
    Snes,
    N64,
    Pce,
    PceCd,
    SegaCd,
    Ps,
    Ngp,
    Ngpc,
    /// A NextUI system with no look of its own, an index into the folders
    /// [`Platform::other`] has registered.
    Other(u8),
}

/// A NextUI system folder shelved with the generic cart and box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Folder {
    tag: &'static str,
    name: &'static str,
    /// Registered only from a saved tag, before a scan has found the folder.
    placeholder: bool,
}

static OTHERS: Mutex<Vec<Folder>> = Mutex::new(Vec::new());

/// Most generic systems that can be registered; more are left off the shelves.
const OTHERS_MAX: usize = 64;

/// Extensions that are never games, so a generic folder's notes, saves and
/// pictures stay off its shelf.
const NOT_GAMES: [&str; 15] = [
    "txt", "nfo", "srm", "sav", "state", "bak", "jpg", "jpeg", "bmp", "gif", "ini", "cfg", "xml",
    "log", "db",
];

/// Whether a file in a generic system's folder is a game rather than a
/// note, save or picture.
pub(crate) fn generic_game(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| !NOT_GAMES.iter().any(|k| ext.eq_ignore_ascii_case(k)))
}

fn others() -> Vec<Folder> {
    OTHERS.lock().map(|o| o.clone()).unwrap_or_default()
}

impl Platform {
    /// The generic platform for a system folder's tag and name, made on
    /// first use. The same tag (any case) is always the same platform.
    pub fn other(tag: &str, name: &str) -> Option<Platform> {
        Self::register(tag, name, false)
    }

    /// The generic platform for a tag read from saved state, before a scan
    /// has found its folder. It has no shelf until `other` names it.
    pub fn other_saved(tag: &str) -> Option<Platform> {
        Self::register(tag, tag, true)
    }

    fn register(tag: &str, name: &str, placeholder: bool) -> Option<Platform> {
        let mut all = OTHERS.lock().ok()?;
        let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
        if let Some(at) = all.iter().position(|f| f.tag.eq_ignore_ascii_case(tag)) {
            if all[at].placeholder && !placeholder {
                all[at].name = leak(name);
                all[at].placeholder = false;
            }
            return Some(Platform::Other(at as u8));
        }
        if all.len() >= OTHERS_MAX {
            return None;
        }
        all.push(Folder {
            tag: leak(&tag.to_ascii_uppercase()),
            name: leak(name),
            placeholder,
        });
        Some(Platform::Other((all.len() - 1) as u8))
    }

    /// Every platform: the fixed ones, then the generic ones registered so far.
    pub fn all() -> Vec<Platform> {
        let named: Vec<Platform> = others()
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.placeholder)
            .map(|(i, _)| Platform::Other(i as u8))
            .collect();
        Platform::ALL.into_iter().chain(named).collect()
    }

    /// Where this platform sorts: the fixed ones in `ALL` order, then the
    /// generic ones in the order they were found.
    pub fn order(self) -> u16 {
        match self {
            Platform::Other(i) => Platform::ALL.len() as u16 + i as u16,
            _ => Platform::ALL.iter().position(|p| *p == self).unwrap_or(0) as u16,
        }
    }

    /// Whether this is a generic system with no look of its own.
    pub fn is_other(self) -> bool {
        matches!(self, Platform::Other(_))
    }

    fn folder(self) -> Folder {
        match self {
            Platform::Other(i) => others().get(i as usize).copied().unwrap_or(Folder {
                tag: "?",
                name: "Other",
                placeholder: true,
            }),
            _ => Folder {
                tag: "",
                name: "",
                placeholder: false,
            },
        }
    }

    pub const ALL: [Platform; 15] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Gg,
        Platform::Nes,
        Platform::Sms,
        Platform::Md,
        Platform::Snes,
        Platform::N64,
        Platform::Pce,
        Platform::PceCd,
        Platform::SegaCd,
        Platform::Ps,
        Platform::Ngp,
        Platform::Ngpc,
    ];

    /// Whether this platform's games are looked for in `Games/` under the
    /// content root when there is no NextUI Roms folder to scan. The other
    /// platforms are only found in a Roms folder.
    pub fn has_games_folder(self) -> bool {
        matches!(self, Platform::Gba | Platform::Gb | Platform::Gbc)
    }

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Gg => "Game Gear",
            Platform::Nes => "NES",
            Platform::Sms => "Master System",
            Platform::Md => "Mega Drive",
            Platform::Snes => "Super Nintendo",
            Platform::N64 => "Nintendo 64",
            Platform::Pce => "TurboGrafx-16",
            Platform::PceCd => "TurboGrafx-CD",
            Platform::SegaCd => "Sega CD",
            Platform::Ps => "PlayStation",
            Platform::Ngp => "Neo Geo Pocket",
            Platform::Ngpc => "Neo Geo Pocket Color",
            Platform::Other(_) => self.folder().name,
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Gg => "GG",
            Platform::Nes => "FC",
            Platform::Sms => "SMS",
            Platform::Md => "MD",
            Platform::Snes => "SFC",
            Platform::N64 => "N64",
            Platform::Pce => "PCE",
            Platform::PceCd => "PCECD",
            Platform::SegaCd => "SEGACD",
            Platform::Ps => "PS",
            Platform::Ngp => "NGP",
            Platform::Ngpc => "NGPC",
            Platform::Other(_) => self.folder().tag,
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Gg => &["gg"],
            Platform::Nes => &["nes", "unf", "unif"],
            Platform::Sms => &["sms"],
            Platform::Md => &["md", "gen", "smd", "bin"],
            Platform::Snes => &["sfc", "smc", "fig", "swc", "bs"],
            Platform::N64 => &["z64", "n64", "v64"],
            Platform::Pce => &["pce"],
            Platform::PceCd => &["chd", "cue", "m3u"],
            Platform::SegaCd => &["chd", "cue", "iso", "m3u"],
            Platform::Ps => &["chd", "cue", "m3u", "pbp"],
            Platform::Ngp => &["ngp"],
            Platform::Ngpc => &["ngc", "ngpc"],
            Platform::Other(_) => &[],
        }
    }

    pub fn accepts(self, path: &Path) -> bool {
        if self.is_other() {
            return generic_game(path);
        }
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}
