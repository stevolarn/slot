mod common;

use std::path::Path;

use common::{boot, tmp_root_with_carts};
use shelf::handoff::{rom_line, RESUME};
use shelf_store::{write_shelf_state, ShelfState};

#[test]
fn a_resumed_game_is_handed_off_with_a_second_line() {
    let rom = Path::new("/mnt/SDCARD/Roms/Game Boy Advance (GBA)/Minish Cap.gba");
    let line = rom_line(rom, true);
    let mut lines = line.lines();
    assert_eq!(lines.next(), Some(rom.to_str().unwrap()));
    assert_eq!(lines.next(), Some(RESUME));
    assert_eq!(lines.next(), None);
    assert_eq!(RESUME, "resume");
}

#[test]
fn without_resume_the_handoff_is_just_the_rom_path() {
    let rom = Path::new("/Roms/Game Boy (GB)/Tetris.gb");
    assert_eq!(rom_line(rom, false), "/Roms/Game Boy (GB)/Tetris.gb");
}

#[test]
fn resume_saves_follows_the_setting_and_starts_on() {
    let d = tmp_root_with_carts(&["Emerald"]);
    assert!(boot(d.path()).resume_saves(), "resume is not on by default");
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            resume: false,
            ..ShelfState::default()
        },
    )
    .unwrap();
    assert!(!boot(d.path()).resume_saves());
}
