use shelf_store::{read_shelf_state, write_shelf_state, ShelfState};

fn card() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    d
}

#[test]
fn the_levels_and_the_crt_setting_round_trip() {
    let d = card();
    let s = ShelfState {
        volume: 70,
        muted: true,
        brightness: 3,
        crt: true,
        ..Default::default()
    };
    write_shelf_state(d.path(), &s).unwrap();
    assert_eq!(read_shelf_state(d.path()), s);
}

#[test]
fn a_state_written_by_an_older_slot_keeps_what_the_shelf_still_uses() {
    let d = card();
    std::fs::write(
        d.path().join("Config/slot.state"),
        "cart=Tetris\ncart_platform=gb\nbrightness=4\nblue_light=3\nvolume=35\nvolume_hp=20\n\
         muted=1\nmuted_hp=0\nclock_set=1\nutc_offset_min=-300\nrumble=0\nff_speed=4\n\
         ff_sound=1\ncolour_correction=1\nshader_gba=dot\nshader_gb=grid\neject_save=0\n\
         turbo=0\nrewind=0\ngb_palettes=1\ngb_palette=pocket\ncrt=1\n",
    )
    .unwrap();
    let s = read_shelf_state(d.path());
    assert_eq!(s.cart.as_deref(), Some("Tetris"));
    assert_eq!(
        (s.brightness, s.volume, s.muted, s.utc_offset_min, s.crt),
        (4, 35, true, -300, true)
    );
}

#[test]
fn a_state_missing_a_level_reads_as_a_fresh_one() {
    let d = card();
    std::fs::write(
        d.path().join("Config/slot.state"),
        "cart=\ncart_platform=\nbrightness=5\nclock_set=1\n",
    )
    .unwrap();
    assert_eq!(read_shelf_state(d.path()), ShelfState::default());
}
