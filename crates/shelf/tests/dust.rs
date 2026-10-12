mod common;

use std::path::PathBuf;

use common::{app_booting_at, insert, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::App;
use shelf_gfx::Draw;
use shelf_input::{Action, Btn};
use shelf_store::{
    dust_amount, played_path, read_played, write_played, Platform, Played, ShelfState,
};
use shelf_ui::{QuickRow, BLOW_MS, FILM_MAX};

const DAY: i64 = 86_400;

fn boot_with(dusty: bool, played: &[(&str, i64)]) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    shelf_store::write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            dusty,
            ..ShelfState::default()
        },
    )
    .unwrap();
    if !played.is_empty() {
        let mut map = Played::new();
        for (stem, secs) in played {
            let rom = d
                .path()
                .join("Games")
                .join(Platform::Gba.dir_name())
                .join(format!("{stem}.{}", Platform::Gba.extensions()[0]));
            map.insert(rom, *secs);
        }
        write_played(d.path(), &map).unwrap();
    }
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    (d, a)
}

fn frame(a: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out
}

/// The film over a cart: a translucent grey-brown rectangle.
fn films(out: &[Draw]) -> Vec<f32> {
    out.iter()
        .filter_map(|d| match d {
            Draw::Rect { colour, w, .. }
                if (colour[0] - 0.62).abs() < 1e-3
                    && (colour[1] - 0.58).abs() < 1e-3
                    && (colour[2] - 0.52).abs() < 1e-3
                    && colour[3] > 0.0
                    && colour[3] <= FILM_MAX
                    && *w > 20.0 =>
            {
                Some(colour[3])
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_cart_nobody_has_touched_starts_a_little_dusty() {
    let (_d, a) = boot_with(true, &[]);
    assert!(
        (a.selected_dust() - 0.35).abs() < 1e-6,
        "{}",
        a.selected_dust()
    );
}

#[test]
fn dust_follows_how_long_ago_the_game_was_played() {
    let ago = |days: i64| CLOCK_IS_SET - days * DAY;
    let (d, a) = boot_with(true, &[("Emerald", ago(3))]);
    assert_eq!(a.selected_dust(), 0.0, "a game from this week is dusty");
    drop(d);
    let (_d, a) = boot_with(true, &[("Emerald", ago(37))]);
    assert!((a.selected_dust() - dust_amount(Some(ago(37)), CLOCK_IS_SET)).abs() < 1e-6);
    assert!((a.selected_dust() - 0.5).abs() < 0.01);
    let (_d, a) = boot_with(true, &[("Emerald", ago(400))]);
    assert_eq!(a.selected_dust(), 1.0);
}

#[test]
fn dust_is_drawn_over_the_carts_only_when_the_setting_is_on() {
    let (_d, on) = boot_with(true, &[]);
    let (_d2, off) = boot_with(false, &[]);
    assert!(!films(&frame(&on)).is_empty(), "no dust on dusty carts");
    assert!(on.selected_dust() > 0.0);
    assert!(films(&frame(&off)).is_empty(), "dust with the setting off");
    assert_eq!(off.selected_dust(), 0.0);
    assert!(
        frame(&on).len() > frame(&off).len(),
        "no specks over the film"
    );
}

#[test]
fn the_side_carts_are_dusted_fainter_than_the_middle_one() {
    let (_d, a) = boot_with(true, &[]);
    let alphas = films(&frame(&a));
    assert!(alphas.len() >= 2, "the side cart has no dust: {alphas:?}");
    let front = alphas.iter().cloned().fold(0.0, f32::max);
    assert!((front - FILM_MAX * 0.35).abs() < 1e-4, "{front}");
    assert!(alphas.iter().any(|a| *a < front));
}

#[test]
fn the_specks_do_not_shimmer_from_frame_to_frame() {
    let (_d, a) = boot_with(true, &[("Emerald", CLOCK_IS_SET - 90 * DAY)]);
    assert_eq!(frame(&a), frame(&a));
}

#[test]
fn the_dusty_carts_row_turns_dust_on_and_off_in_the_menu() {
    let (_d, mut a) = boot_with(true, &[]);
    a.apply(Action::QuickMenu);
    a.apply(Action::GbaDown(Btn::A));
    for _ in 0..4 {
        a.apply(Action::GbaDown(Btn::Down));
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::Dusty));
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(
        a.selected_dust(),
        0.0,
        "the dust stayed after turning it off"
    );
    a.apply(Action::GbaDown(Btn::Left));
    assert!(a.selected_dust() > 0.0, "the dust did not come back");
}

fn blow_through(a: &mut App) {
    a.apply_at(Action::GbaDown(Btn::X), 1000);
    assert!(a.blow().is_some(), "X did not blow on the cart");
    a.tick_ms(1000 + BLOW_MS + 50);
    a.update(1.0 / 60.0);
    assert!(a.blow().is_none());
}

#[test]
fn blowing_on_a_cart_takes_its_dust_off_for_good_and_notes_the_time() {
    let (d, mut a) = boot_with(true, &[("Emerald", CLOCK_IS_SET - 90 * DAY)]);
    assert_eq!(a.selected_dust(), 1.0);
    let rom: PathBuf = a.browsed_rom().expect("a cart").to_path_buf();
    blow_through(&mut a);
    assert_eq!(a.selected_dust(), 0.0, "the cart is still dusty");
    assert_eq!(
        read_played(d.path()).get(&rom),
        Some(&CLOCK_IS_SET),
        "the blow was not written to played.txt"
    );
    let (again, _) = app_booting_at(d.path(), CLOCK_IS_SET + 2 * DAY);
    assert_eq!(again.selected_dust(), 0.0, "the cart got dusty again");
}

#[test]
fn the_front_dust_fades_with_the_breath() {
    let (_d, mut a) = boot_with(true, &[("Emerald", CLOCK_IS_SET - 90 * DAY)]);
    let front = |a: &App| films(&frame(a)).into_iter().fold(0.0, f32::max);
    let before = front(&a);
    assert!((before - FILM_MAX).abs() < 1e-4, "{before}");
    a.apply_at(Action::GbaDown(Btn::X), 1000);
    a.tick_ms(1000 + (BLOW_MS as f32 * 0.45) as u64);
    let mid = front(&a);
    assert!(mid < before, "the dust stayed thick in the breath: {mid}");
    a.tick_ms(1000 + (BLOW_MS as f32 * 0.8) as u64);
    assert!(front(&a) < mid, "the dust came back");
}

#[test]
fn blowing_still_works_and_is_noted_with_dust_off() {
    let (d, mut a) = boot_with(false, &[]);
    let rom: PathBuf = a.browsed_rom().expect("a cart").to_path_buf();
    blow_through(&mut a);
    assert_eq!(read_played(d.path()).get(&rom), Some(&CLOCK_IS_SET));
    assert!(films(&frame(&a)).is_empty());
}

#[test]
fn playing_a_game_notes_the_time_in_played_txt() {
    let (d, mut a) = boot_with(true, &[]);
    let rom: PathBuf = a.browsed_rom().expect("a cart").to_path_buf();
    assert!(!played_path(d.path()).exists());
    insert(&mut a);
    let played = read_played(d.path());
    assert_eq!(played.get(&rom), Some(&CLOCK_IS_SET));
    let text = std::fs::read_to_string(played_path(d.path())).unwrap();
    assert_eq!(
        text,
        format!("{CLOCK_IS_SET}\t{}\n", rom.display()),
        "played.txt is <seconds><tab><rom path>"
    );
}
