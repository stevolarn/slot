#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, Ordering};
use std::sync::Arc;

use shelf::app::App;
use shelf::session::Session;
use shelf_input::Action;
use shelf_power::{Battery, Charge, Platform, Power, SimPlatform};
use shelf_store::{write_shelf_state, Platform as CartPlatform};
use tempfile::TempDir;

/// The folders a test card has: the picker's own, and the `Games` and
/// `Labels` ones carts are put in.
const CARD_DIRS: [&str; 12] = [
    "Config",
    "System",
    "Games",
    "Games/GBA",
    "Games/GB",
    "Games/GBC",
    "Labels",
    "Labels/GBA",
    "Labels/GB",
    "Labels/GBC",
    "Wallpapers",
    "Carts",
];

pub fn tmp_root_with_carts(stems: &[&str]) -> TempDir {
    let d = tmp_root();
    for stem in stems {
        let mut rom = vec![0u8; 0x100];
        let title = stem.to_uppercase();
        rom[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
        std::fs::write(rom_path(&d, stem), rom).expect("write rom");
    }
    d
}

pub fn tmp_root_with_gb_carts(stems: &[&str]) -> TempDir {
    let d = tmp_root();
    for stem in stems {
        let title = stem.to_uppercase();
        write_gb_cart(&d, stem, &title[..title.len().min(11)]);
    }
    d
}

pub fn write_gb_cart(d: &TempDir, stem: &str, title: &str) {
    assert!(
        title.len() <= 11,
        "a Game Boy header title is eleven bytes, and {title:?} is longer"
    );
    let mut rom = vec![0u8; 0x150];
    rom[0x134..0x134 + title.len()].copy_from_slice(title.as_bytes());
    std::fs::write(cart_path(d, CartPlatform::Gb, stem), rom).expect("write rom");
}

pub fn tmp_root_with_real_carts(stems: &[&str]) -> TempDir {
    let d = tmp_root();
    for stem in stems {
        std::fs::write(rom_path(&d, stem), gba_rom()).expect("write rom");
    }
    d
}

fn tmp_root() -> TempDir {
    let d = tempfile::tempdir().expect("tempdir");
    for sub in CARD_DIRS {
        std::fs::create_dir(d.path().join(sub)).expect("create content dir");
    }
    d
}

fn rom_path(d: &TempDir, stem: &str) -> PathBuf {
    cart_path(d, CartPlatform::Gba, stem)
}

fn cart_path(d: &TempDir, platform: CartPlatform, stem: &str) -> PathBuf {
    d.path()
        .join("Games")
        .join(platform.dir_name())
        .join(format!("{stem}.{}", platform.extensions()[0]))
}

pub fn write_retail_header(d: &TempDir, stem: &str, title: &str, code: &str) {
    let mut rom = vec![0u8; 0x100];
    rom[3] = 0xEA;
    rom[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
    rom[0xac..0xac + code.len()].copy_from_slice(code.as_bytes());
    rom[0xb2] = 0x96;
    std::fs::write(rom_path(d, stem), rom).expect("write rom");
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn gba_rom() -> Vec<u8> {
    const CODE: [u32; 15] = [
        0xe3a00404, 0xe3a01c04, 0xe3811003, 0xe5801000, 0xe3a02406, 0xe3a03000, 0xe1d040b6,
        0xe35400a0, 0x1afffffc, 0xe2833001, 0xe1c230b0, 0xe1d040b6, 0xe35400a0, 0x0afffffc,
        0xeafffff6,
    ];
    let mut rom = vec![0u8; 0x8000];
    rom[0..4].copy_from_slice(&0xea00002eu32.to_le_bytes());
    rom[0xa0..0xac].copy_from_slice(b"SLOT TEST\0\0\0");
    rom[0xac..0xb0].copy_from_slice(b"SLTE");
    rom[0xb0..0xb2].copy_from_slice(b"00");
    rom[0xb2] = 0x96;
    let sum = rom[0xa0..0xbd].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    rom[0xbd] = 0u8.wrapping_sub(sum).wrapping_sub(0x19);
    for (i, w) in CODE.iter().enumerate() {
        let o = 0xc0 + i * 4;
        rom[o..o + 4].copy_from_slice(&w.to_le_bytes());
    }
    rom
}

pub fn write_real_cart_as(d: &TempDir, stem: &str, title: &str, code: &str) {
    let mut rom = gba_rom();
    rom[0xa0..0xac].fill(0);
    rom[0xa0..0xa0 + title.len()].copy_from_slice(title.as_bytes());
    rom[0xac..0xac + code.len()].copy_from_slice(code.as_bytes());
    let sum = rom[0xa0..0xbd].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    rom[0xbd] = 0u8.wrapping_sub(sum).wrapping_sub(0x19);
    std::fs::write(rom_path(d, stem), rom).expect("write rom");
}

#[derive(Clone, Default)]
pub struct Clock(Arc<AtomicI64>);

impl Clock {
    pub fn at(secs: i64) -> Self {
        Clock(Arc::new(AtomicI64::new(secs)))
    }

    pub fn get(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }

    pub fn advance(&self, secs: i64) {
        self.0.fetch_add(secs, Ordering::Relaxed);
    }
}

pub struct StubPlatform {
    backlight: Arc<AtomicU8>,
    root: PathBuf,
    clock: Clock,
    charge: Arc<AtomicU8>,
    percent: Arc<AtomicU8>,
    read_only: Arc<AtomicBool>,
}

pub const CLOCK_IS_SET: i64 = 1_786_568_000;

pub fn panel(root: &Path) -> (Power, Arc<AtomicU8>) {
    let (power, backlight, _, _, _) = rig_with_charge(root, CLOCK_IS_SET, 0, 50);
    (power, backlight)
}

pub fn panel_with_battery(root: &Path, charge: u8, percent: u8) -> (Power, Arc<AtomicU8>) {
    let (power, backlight, _, _, _) = rig_with_charge(root, CLOCK_IS_SET, charge, percent);
    (power, backlight)
}

fn rig(root: &Path, secs: i64) -> (Power, Arc<AtomicU8>, Clock) {
    let (power, backlight, clock, _, _) = rig_with_charge(root, secs, 0, 50);
    (power, backlight, clock)
}

#[allow(clippy::type_complexity)]
fn rig_with_charge(
    root: &Path,
    secs: i64,
    charge: u8,
    percent: u8,
) -> (Power, Arc<AtomicU8>, Clock, Arc<AtomicU8>, Arc<AtomicU8>) {
    let backlight = Arc::new(AtomicU8::new(0));
    let clock = Clock::at(secs);
    let charge = Arc::new(AtomicU8::new(charge));
    let percent = Arc::new(AtomicU8::new(percent));
    let platform = StubPlatform {
        backlight: backlight.clone(),
        root: root.to_path_buf(),
        clock: clock.clone(),
        charge: charge.clone(),
        percent: percent.clone(),
        read_only: Arc::new(AtomicBool::new(false)),
    };
    (
        Power::new(Box::new(platform)),
        backlight,
        clock,
        charge,
        percent,
    )
}

pub fn session_with_platform(root: &Path) -> Session {
    clocked(root);
    let platform = SimPlatform::at(root.to_path_buf());
    let mut session = Session::boot(root.to_path_buf());
    session.app_mut().set_power(Power::new(Box::new(platform)));
    session
}

pub fn app_booting_with_clock(root: &Path) -> (App, Clock) {
    app_booting_at(root, 0)
}

pub fn app_booting_at(root: &Path, secs: i64) -> (App, Clock) {
    let mut a = App::boot(root);
    let (power, _, clock) = rig(root, secs);
    a.set_power(power);
    (a, clock)
}

impl Platform for StubPlatform {
    fn set_backlight(&mut self, step: u8) {
        self.backlight.store(step, Ordering::Relaxed);
    }

    fn charge(&self) -> Charge {
        match self.charge.load(Ordering::Relaxed) {
            1 => Charge::Discharging,
            2 => Charge::Charging,
            3 => Charge::Full,
            _ => Charge::Unknown,
        }
    }

    fn battery(&self) -> Option<Battery> {
        Some(Battery {
            percent: self.percent.load(Ordering::Relaxed),
            charge: self.charge(),
        })
    }

    fn restart(&mut self) -> ! {
        panic!("the stub platform never powers off")
    }

    fn poweroff(&mut self) -> ! {
        panic!("the stub platform never powers off")
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn now(&self) -> i64 {
        self.clock.get()
    }

    fn set_clock(&mut self, secs: i64) {
        self.clock.0.store(secs, Ordering::Relaxed);
    }

    fn card_read_only(&self) -> bool {
        self.read_only.load(Ordering::Relaxed)
    }
}

pub fn clocked(root: &Path) {
    let mut s = shelf_store::read_shelf_state(root);
    s.clock_set = true;
    write_shelf_state(root, &s).expect("write shelf.state");
}

pub fn boot(root: &Path) -> App {
    clocked(root);
    App::boot(root)
}

/// Press and let go of A on the shelf, which puts the cart in the slot.
pub fn insert(a: &mut App) {
    a.apply(Action::GbaDown(shelf_input::Btn::A));
    a.apply(Action::GbaUp(shelf_input::Btn::A));
}

/// A booted app that finds its card unwritable once `read_only` is set.
pub fn app_with_card(root: &Path) -> (App, Arc<AtomicBool>) {
    let mut a = boot(root);
    let read_only = Arc::new(AtomicBool::new(false));
    let platform = StubPlatform {
        backlight: Arc::new(AtomicU8::new(0)),
        root: root.to_path_buf(),
        clock: Clock::at(CLOCK_IS_SET),
        charge: Arc::new(AtomicU8::new(0)),
        percent: Arc::new(AtomicU8::new(50)),
        read_only: read_only.clone(),
    };
    a.set_power(Power::new(Box::new(platform)));
    (a, read_only)
}

/// A booted app on a battery whose charge and percent the test sets.
pub fn app_with_charge(root: &Path) -> (App, Arc<AtomicU8>, Arc<AtomicU8>) {
    let mut a = boot(root);
    let (power, _backlight, _clock, charge, percent) = rig_with_charge(root, 0, 1, 50);
    a.set_power(power);
    (a, charge, percent)
}

pub fn app_with_volume(root: &Path, volume: u8) -> App {
    let mut state = shelf_store::read_shelf_state(root);
    state.volume = volume;
    state.clock_set = true;
    write_shelf_state(root, &state).expect("write shelf.state");
    App::boot(root)
}
