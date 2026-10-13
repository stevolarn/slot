mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_gfx::Draw;
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::TexId;

const TAG: usize = 905;

fn write_in(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create dir");
    std::fs::write(&path, body).expect("write file");
}

fn tag_drawn(a: &App) -> bool {
    let mut out = Vec::new();
    a.draw(&mut out);
    out.iter()
        .any(|d| matches!(*d, Draw::Tex { tex, .. } if tex == TexId::from_raw(TAG)))
}

fn shelf_with(resume: bool, shared: &Path) -> (tempfile::TempDir, App) {
    let root = tmp_root_with_carts(&[]);
    write_shelf_state(
        root.path(),
        &ShelfState {
            clock_set: true,
            resume,
            ..ShelfState::default()
        },
    )
    .unwrap();
    std::env::set_var(shelf::handoff::SHARED_USERDATA_VAR, shared);
    let (mut a, _) = app_booting_at(root.path(), CLOCK_IS_SET);
    a.set_note_face((TexId::from_raw(TAG), 50, 20));
    a.update(1.0 / 60.0);
    (root, a)
}

#[test]
fn a_game_with_a_nextui_save_slot_wears_a_continue_note_while_resume_is_on() {
    let roms = tempfile::tempdir().unwrap();
    write_in(roms.path(), "Game Boy Advance (GBA)/Minish Cap.gba", "rom");
    std::env::set_var(shelf::handoff::ROMS_VAR, roms.path());
    let shared = tempfile::tempdir().unwrap();

    let (_r, a) = shelf_with(true, shared.path());
    assert!(a.browsed_rom().is_some(), "no game on the shelf");
    assert!(!a.selected_saved(), "saved with no slot file");
    assert!(!tag_drawn(&a));

    write_in(shared.path(), ".minui/GBA/Minish Cap.gba.txt", "");
    let (_r, a) = shelf_with(true, shared.path());
    assert!(!a.selected_saved(), "an empty slot file counts as a save");

    write_in(shared.path(), ".minui/GBA/Minish Cap.gba.txt", "0");
    let (_r, a) = shelf_with(true, shared.path());
    assert!(a.selected_saved(), "a slot file was not found");
    assert!(tag_drawn(&a), "no CONTINUE? note on the cart");

    let (_r, a) = shelf_with(false, shared.path());
    assert!(!a.selected_saved());
    assert!(!tag_drawn(&a), "a CONTINUE? note with Resume Saves off");

    write_in(shared.path(), ".minui/GB/Minish Cap.gba.txt", "");
    std::fs::remove_file(shared.path().join(".minui/GBA/Minish Cap.gba.txt")).unwrap();
    let (_r, a) = shelf_with(true, shared.path());
    assert!(!a.selected_saved(), "another system's slot was used");
}
