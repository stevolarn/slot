mod common;

use common::{app_with_volume, boot, tmp_root_with_carts};
use shelf::app::Phase;
use shelf_input::Action;
use shelf_store::read_shelf_state;
use shelf_ui::Icon;

#[test]
fn levels_clamp_and_persist() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    for _ in 0..20 {
        a.apply(Action::BrightnessUp);
    }
    a.flush_state();
    assert_eq!(read_shelf_state(d.path()).brightness, 9);
    for _ in 0..20 {
        a.apply(Action::BrightnessDown);
    }
    a.flush_state();
    assert_eq!(read_shelf_state(d.path()).brightness, 0);
}

#[test]
fn volume_moves_five_per_press() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());
    a.apply(Action::VolumeUp);
    a.flush_state();
    assert_eq!(read_shelf_state(d.path()).volume, 65);
    for _ in 0..20 {
        a.apply(Action::VolumeDown);
    }
    a.flush_state();
    assert_eq!(read_shelf_state(d.path()).volume, 0);
}

#[test]
fn adjusting_a_level_leaves_the_shelf_where_it_is() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    assert!(matches!(a.phase(), Phase::Shelf));
    a.apply(Action::VolumeDown);
    assert!(matches!(a.phase(), Phase::Shelf));
    a.flush_state();
    assert_eq!(read_shelf_state(d.path()).volume, 55);
}

#[test]
fn muting_leaves_the_level_alone_even_at_the_ends() {
    for start in [0u8, 5, 60, 100] {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_with_volume(d.path(), start);
        a.apply_at(Action::VolumeUp, 0);
        a.apply_at(Action::VolumeDown, 40);
        a.apply_at(Action::MuteToggle, 60);
        assert!(a.muted(), "not muted from {start}");
        a.apply_at(Action::MuteToggle, 2_000);
        assert_eq!(a.volume(), start, "unmuted to the wrong level from {start}");
    }
}

#[test]
fn muted_is_silent_but_remembers() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    assert_eq!(a.output_volume(), 0, "muted but still audible");
    assert_eq!(a.volume(), 70, "muting forgot the level");
    assert_eq!(a.hud_icon(), Icon::VolumeMuted);
}

#[test]
fn adjusting_the_volume_unmutes() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    a.apply_at(Action::VolumeUp, 1_000);
    assert!(!a.muted(), "still muted after turning it up");
    assert_eq!(a.volume(), 75);
}

#[test]
fn unmuting_with_the_chord_does_not_mute_again() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    a.apply_at(Action::VolumeDown, 1_000);
    a.apply_at(Action::VolumeUp, 1_060);
    a.apply_at(Action::MuteToggle, 1_060);
    assert!(!a.muted(), "the unmute chord muted it again");
    assert_eq!(a.volume(), 70);
}

#[test]
fn the_mute_survives_a_reboot() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_with_volume(d.path(), 70);
    a.apply_at(Action::MuteToggle, 0);
    drop(a);
    let b = boot(d.path());
    assert!(b.muted());
    assert_eq!(b.volume(), 70);
}
