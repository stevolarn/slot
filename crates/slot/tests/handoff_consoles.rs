mod common;

use common::{session_with_platform, tmp_root_with_carts};
use slot::session::Session;
use slot_input::{Btn, Millis, RawEvent};

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn press(s: &mut Session, now: &mut Millis, btn: Btn) {
    step(s, now, vec![RawEvent::Down(btn)]);
    step(s, now, vec![RawEvent::Up(btn)]);
}

fn write_in(dir: &std::path::Path, rel: &str) -> std::path::PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("rom has a parent")).expect("create dir");
    std::fs::write(&path, vec![0u8; 0x200]).expect("write rom");
    path
}

#[test]
fn nes_mega_drive_and_snes_games_get_shelves_and_are_handed_off() {
    let root = tmp_root_with_carts(&[]);
    let roms = tempfile::TempDir::new().expect("roms");
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba");
    let nes = write_in(
        roms.path(),
        "Nintendo Entertainment System (FC)/Metroid.nes",
    );
    let md = write_in(roms.path(), "Sega Genesis (MD)/Ristar (USA, Europe).md");
    let snes = write_in(
        roms.path(),
        "Super Nintendo Entertainment System (SFC)/Super Metroid.7z",
    );
    std::env::set_var(slot::handoff::ROMS_VAR, roms.path());
    std::env::set_var(
        slot::handoff::HANDOFF_VAR,
        "/tmp/slot-handoff-consoles-test",
    );

    let (mut s, _) = session_with_platform(root.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.push((
            s.app().shelf_platform_name(),
            s.app().browsed_rom().map(|p| p.to_path_buf()),
        ));
        press(&mut s, &mut now, Btn::Down);
        for _ in 0..30 {
            step(&mut s, &mut now, vec![]);
        }
    }
    assert!(seen.contains(&(Some("NES"), Some(nes))), "{seen:?}");
    assert!(
        seen.contains(&(Some("Mega Drive"), Some(md.clone()))),
        "{seen:?}"
    );
    assert!(
        seen.contains(&(Some("Super Nintendo"), Some(snes))),
        "{seen:?}"
    );

    while s.app().shelf_platform_name() != Some("Mega Drive") {
        press(&mut s, &mut now, Btn::Down);
        for _ in 0..30 {
            step(&mut s, &mut now, vec![]);
        }
    }
    press(&mut s, &mut now, Btn::A);
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    assert_eq!(s.app().handed_off(), Some(md.as_path()));
    assert!(!s.has_core(), "a core ran for a Mega Drive game");
}
