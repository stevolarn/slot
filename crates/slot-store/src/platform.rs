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
    Nes,
    Md,
    Snes,
}

impl Platform {
    pub const ALL: [Platform; 6] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Nes,
        Platform::Md,
        Platform::Snes,
    ];

    /// Whether slot can run this platform's games with its own cores. The
    /// others are only shown in game picker mode, where another emulator
    /// runs them.
    pub fn built_in(self) -> bool {
        matches!(self, Platform::Gba | Platform::Gb | Platform::Gbc)
    }

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Nes => "NES",
            Platform::Md => "Mega Drive",
            Platform::Snes => "Super Nintendo",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Nes => "FC",
            Platform::Md => "MD",
            Platform::Snes => "SFC",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Nes => &["nes", "unf", "unif"],
            Platform::Md => &["md", "gen", "smd", "bin"],
            Platform::Snes => &["sfc", "smc", "fig", "swc", "bs"],
        }
    }

    pub fn picture(self) -> (u32, u32) {
        match self {
            Platform::Gba => (240, 160),
            Platform::Gb | Platform::Gbc => (160, 144),
            Platform::Nes => (256, 240),
            Platform::Md => (320, 224),
            Platform::Snes => (256, 224),
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
