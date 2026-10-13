use std::path::Path;

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
}

impl Platform {
    pub const ALL: [Platform; 8] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Gg,
        Platform::Nes,
        Platform::Sms,
        Platform::Md,
        Platform::Snes,
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
        }
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}
