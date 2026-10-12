use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::platform::Platform;

pub const BRIGHTNESS_MAX: u8 = 9;
pub const VOLUME_MAX: u8 = 100;

pub const UTC_OFFSET_MIN: i16 = -720;
pub const UTC_OFFSET_MAX: i16 = 840;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShelfState {
    pub cart: Option<String>,
    pub cart_platform: Option<Platform>,
    pub brightness: u8,
    pub volume: u8,
    pub muted: bool,
    pub clock_set: bool,
    pub utc_offset_min: i16,
    /// Scanlines and curved corners over the shelf, like an old TV.
    pub crt: bool,
    /// Carts gather dust on the shelf the longer they go unplayed.
    pub dusty: bool,
    /// Games continue from NextUI's last save state when played.
    pub resume: bool,
}

impl Default for ShelfState {
    fn default() -> Self {
        ShelfState {
            cart: None,
            cart_platform: None,
            brightness: 5,
            volume: 60,
            muted: false,
            clock_set: false,
            utc_offset_min: 0,
            crt: false,
            dusty: true,
            resume: true,
        }
    }
}

fn state_path(root: &Path) -> PathBuf {
    root.join("Config").join("shelf.state")
}

/// The settings file's name before the rename; read when `shelf.state` has
/// not been written yet, so settings survive the upgrade.
pub const OLD_STATE_FILE: &str = "slot.state";

pub fn read_shelf_state(root: &Path) -> ShelfState {
    let path = state_path(root);
    let path = if path.exists() {
        path
    } else {
        root.join("Config").join(OLD_STATE_FILE)
    };
    std::fs::read(path)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| parse(&s))
        .unwrap_or_default()
}

pub fn write_shelf_state(root: &Path, s: &ShelfState) -> std::io::Result<()> {
    let text = format!(
        "cart={}\ncart_platform={}\nbrightness={}\nvolume={}\nmuted={}\nclock_set={}\nutc_offset_min={}\ncrt={}\ndusty={}\nresume={}\n",
        s.cart.as_deref().unwrap_or(""),
        s.cart_platform.map_or(String::new(), platform_key),
        s.brightness,
        s.volume,
        s.muted as u8,
        s.clock_set as u8,
        s.utc_offset_min,
        s.crt as u8,
        s.dusty as u8,
        s.resume as u8
    );
    atomic_write(&state_path(root), text.as_bytes())
}

/// Reads what `write_shelf_state` wrote. Keys it does not know, such as the
/// game settings older versions kept here, are left alone.
fn parse(text: &str) -> Option<ShelfState> {
    let mut cart = None;
    let mut cart_platform = None;
    let mut brightness = None;
    let mut volume = None;
    let mut muted = None;
    let mut clock_set = None;
    let mut utc_offset_min = None;
    let mut crt = None;
    let mut dusty = None;
    let mut resume = None;
    for line in text.lines().filter(|l| !l.is_empty()) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "cart" => cart = Some(value.to_string()),
            "cart_platform" => cart_platform = platform_value(value),
            "brightness" => brightness = Some(level(value, BRIGHTNESS_MAX)?),
            "volume" => volume = Some(level(value, VOLUME_MAX)?),
            "muted" => muted = Some(level(value, 1)? == 1),
            "clock_set" => clock_set = Some(level(value, 1)? == 1),
            "utc_offset_min" => utc_offset_min = Some(offset(value)?),
            "crt" => crt = flag(value),
            "dusty" => dusty = flag(value),
            "resume" => resume = flag(value),
            _ => {}
        }
    }
    let cart = cart?;
    let fallback = ShelfState::default();
    Some(ShelfState {
        cart: (!cart.is_empty()).then_some(cart),
        cart_platform,
        brightness: brightness?,
        volume: volume?,
        muted: muted?,
        clock_set: clock_set?,
        utc_offset_min: utc_offset_min?,
        crt: crt.unwrap_or(fallback.crt),
        dusty: dusty.unwrap_or(fallback.dusty),
        resume: resume.unwrap_or(fallback.resume),
    })
}

fn platform_key(platform: Platform) -> String {
    platform.dir_name().to_ascii_lowercase()
}

fn platform_value(value: &str) -> Option<Platform> {
    Platform::ALL
        .into_iter()
        .find(|p| value.eq_ignore_ascii_case(p.dir_name()))
}

fn offset(value: &str) -> Option<i16> {
    value
        .parse()
        .ok()
        .filter(|n| (UTC_OFFSET_MIN..=UTC_OFFSET_MAX).contains(n))
}

fn level(value: &str, max: u8) -> Option<u8> {
    value.parse().ok().filter(|n| *n <= max)
}

fn flag(value: &str) -> Option<bool> {
    level(value, 1).map(|n| n == 1)
}
