mod common;

use common::{session_with_platform, tmp_root_with_carts};
use shelf::session::Session;
use shelf_input::{Btn, Millis, RawEvent};

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn write_in(dir: &std::path::Path, rel: &str) -> std::path::PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("rom has a parent")).expect("create dir");
    std::fs::write(&path, vec![0u8; 0x200]).expect("write rom");
    path
}

#[test]
fn with_a_collection_set_only_the_games_it_lists_are_shelved() {
    let root = tmp_root_with_carts(&[]);
    let card = tempfile::TempDir::new().expect("card");
    let roms = card.path().join("Roms");
    let tetris = write_in(&roms, "Game Boy (GB)/Tetris.zip");
    write_in(&roms, "Game Boy (GB)/Kirby.zip");
    let ristar = write_in(&roms, "Sega Genesis (MD)/Ristar.md");
    write_in(&roms, "Sega Genesis (MD)/Aladdin.md");
    let lists = card.path().join("Collections/Top 100");
    std::fs::create_dir_all(&lists).unwrap();
    std::fs::write(
        lists.join("T100 Game Boy.txt"),
        "/Roms/Game Boy (GB)/Tetris.zip\n",
    )
    .unwrap();
    std::fs::write(
        lists.join("T100 Sega Genesis.txt"),
        "/Roms/Sega Genesis (MD)/Ristar.md\n",
    )
    .unwrap();
    std::env::set_var(shelf::handoff::ROMS_VAR, &roms);
    std::env::set_var(shelf::handoff::ONLY_VAR, &lists);
    std::env::set_var(shelf::handoff::HANDOFF_VAR, "/tmp/slot-handoff-only-test");

    let mut s = session_with_platform(root.path());
    let mut now: Millis = 0;
    step(&mut s, &mut now, vec![]);
    let mut seen = Vec::new();
    for _ in 0..4 {
        for _ in 0..3 {
            seen.extend(s.app().browsed_rom().map(|p| p.to_path_buf()));
            step(&mut s, &mut now, vec![RawEvent::Down(Btn::Right)]);
            step(&mut s, &mut now, vec![RawEvent::Up(Btn::Right)]);
            for _ in 0..30 {
                step(&mut s, &mut now, vec![]);
            }
        }
        step(&mut s, &mut now, vec![RawEvent::Down(Btn::Down)]);
        step(&mut s, &mut now, vec![RawEvent::Up(Btn::Down)]);
        for _ in 0..30 {
            step(&mut s, &mut now, vec![]);
        }
    }
    seen.sort();
    seen.dedup();
    let mut want = vec![tetris, ristar];
    want.sort();
    assert_eq!(seen, want);
}
