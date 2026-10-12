mod common;

use std::path::Path;

use shelf::app::{App, Phase, ShelfKey};
use shelf::audio::Sfx;
use shelf_input::{Action, Btn};
use shelf_store::{Cart, Platform};
use shelf_ui::{Draw, TexId, CART_H, CART_W};

fn play(a: &mut App) {
    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaUp(Btn::A));
}

fn app_with_carts(stems: &[&str]) -> App {
    App::new(
        stems
            .iter()
            .map(|stem| Cart {
                platform: Platform::Gba,
                stem: (*stem).to_string(),
                rom: format!("Games/GBA/{stem}.gba").into(),
                label: None,
                code: String::new(),
                shell: None,
                title: stem.to_uppercase(),
            })
            .collect(),
    )
}

fn is_mouth(d: &Draw) -> bool {
    match *d {
        Draw::Rect { colour, .. } => std::iter::once(None)
            .chain(Platform::ALL.into_iter().map(Some))
            .map(|p| shelf_ui::slot_colours(p)[2])
            .any(|o| (0..3).all(|i| (colour[i] - o[i]).abs() < 0.001)),
        _ => false,
    }
}

#[test]
fn the_cart_sounds_when_it_reaches_the_slot_and_not_when_it_starts_moving() {
    let mut a = app_with_carts(&["Emerald", "Zzz"]);
    play(&mut a);
    a.update(1.0 / 60.0);
    assert_eq!(a.take_sfx(), None, "it sounded before it touched anything");
    let mut heard = None;
    while a.seat() < 1.0 && heard.is_none() {
        a.update(1.0 / 60.0);
        heard = a.take_sfx();
    }
    assert_eq!(heard, Some(Sfx::Insert));
}

#[test]
fn an_empty_shelf_has_nothing_to_insert() {
    let mut a = app_with_carts(&[]);
    play(&mut a);
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn a_held_direction_walks_the_shelf_and_a_release_stops_it() {
    let mut a = app_with_carts(&["A", "B", "C", "D", "E", "F", "G"]);
    a.apply(Action::GbaDown(Btn::Right));
    for _ in 0..30 {
        a.update(1.0 / 60.0);
    }
    a.apply(Action::GbaUp(Btn::Right));
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    play(&mut a);
    let Phase::Inserting { cart, .. } = a.phase() else {
        panic!("A on the shelf did not insert: {:?}", a.phase())
    };
    assert_eq!(cart, "C", "one press and one repeat, then nothing");
}

fn app_with_platforms(carts: &[(Platform, &str)]) -> App {
    App::new(
        carts
            .iter()
            .map(|(platform, stem)| Cart {
                platform: *platform,
                stem: (*stem).to_string(),
                rom: format!(
                    "Games/{}/{stem}.{}",
                    platform.dir_name(),
                    platform.extensions()[0]
                )
                .into(),
                label: None,
                code: String::new(),
                shell: None,
                title: stem.to_uppercase(),
            })
            .collect(),
    )
}

fn tap(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

#[test]
fn up_and_down_ring_over_the_shelves() {
    let mut a = app_with_platforms(&[(Platform::Gba, "Emerald"), (Platform::Gb, "Tetris")]);
    assert_eq!(a.selected_stem(), Some("Emerald"));
    tap(&mut a, Btn::Down);
    assert_eq!(
        a.selected_stem(),
        Some("Tetris"),
        "Down did not reach the Game Boy shelf"
    );
    tap(&mut a, Btn::Down);
    assert_eq!(
        a.selected_stem(),
        Some("Emerald"),
        "the ring did not come back round"
    );
    tap(&mut a, Btn::Up);
    assert_eq!(
        a.selected_stem(),
        Some("Tetris"),
        "Up did not reach the Game Boy shelf"
    );
}

#[test]
fn a_colour_cart_stands_on_a_shelf_of_its_own() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Emerald"),
        (Platform::Gb, "Tetris"),
        (Platform::Gbc, "Chromatic"),
    ]);
    tap(&mut a, Btn::Down);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    tap(&mut a, Btn::Down);
    assert_eq!(
        a.selected_stem(),
        Some("Chromatic"),
        "the Colour cart shares the Game Boy shelf"
    );
    tap(&mut a, Btn::Down);
    assert_eq!(
        a.selected_stem(),
        Some("Emerald"),
        "three shelves did not ring back round to the first"
    );
    tap(&mut a, Btn::Up);
    assert_eq!(a.selected_stem(), Some("Chromatic"));
}

#[test]
fn every_shelf_keeps_the_cart_it_was_left_on() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Emerald"),
        (Platform::Gba, "Fusion"),
        (Platform::Gb, "Tetris"),
        (Platform::Gb, "Zelda"),
    ]);
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Fusion"));
    tap(&mut a, Btn::Down);
    assert_eq!(a.selected_stem(), Some("Tetris"));
    tap(&mut a, Btn::Right);
    assert_eq!(a.selected_stem(), Some("Zelda"));

    tap(&mut a, Btn::Up);
    assert_eq!(
        a.selected_stem(),
        Some("Fusion"),
        "the Game Boy Advance shelf forgot where it was"
    );
    tap(&mut a, Btn::Down);
    assert_eq!(
        a.selected_stem(),
        Some("Zelda"),
        "the Game Boy shelf forgot where it was"
    );
}

#[test]
fn a_shelf_comes_back_settled_where_it_was_left() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Aaa"),
        (Platform::Gba, "Bbb"),
        (Platform::Gba, "Ccc"),
        (Platform::Gb, "Tetris"),
    ]);
    let faces: Vec<TexId> = (0..4).map(|i| TexId::from_raw(700 + i)).collect();
    a.set_faces(faces.clone());
    tap(&mut a, Btn::Right);
    tap(&mut a, Btn::Right);
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    let settled = tex_at(&frame(&a), faces[2])
        .expect("no face for the selected cart")
        .1;

    tap(&mut a, Btn::Down);
    assert!(
        tex_at(&frame(&a), faces[3]).is_some(),
        "the Game Boy cart has no face of its own"
    );
    tap(&mut a, Btn::Up);
    a.update(1.0 / 60.0);
    let back = tex_at(&frame(&a), faces[2])
        .expect("no face on the way back")
        .1;
    assert!(
        (back[0] - settled[0]).abs() < 1.0,
        "the row came back at {} rather than settled at {}",
        back[0],
        settled[0]
    );
}

#[test]
fn up_and_down_do_nothing_when_there_is_one_shelf_to_be_on() {
    let mut a = app_with_platforms(&[(Platform::Gba, "Emerald"), (Platform::Gba, "Fusion")]);
    for btn in [Btn::Up, Btn::Down] {
        tap(&mut a, btn);
        assert_eq!(
            a.selected_stem(),
            Some("Emerald"),
            "{btn:?} moved a shelf with no neighbour"
        );
        assert_eq!(
            a.toast(),
            None,
            "{btn:?} said something about doing nothing"
        );
        assert_eq!(
            a.shelf_shake(),
            0.0,
            "{btn:?} refused where it should have done nothing"
        );
    }
}

#[test]
fn the_switch_changes_the_name_on_the_band_and_says_nothing() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Emerald"),
        (Platform::Gb, "Tetris"),
        (Platform::Gbc, "Chromatic"),
    ]);
    assert_eq!(
        a.shelf_platform_name(),
        Some("Game Boy Advance"),
        "the card did not open on the GBA"
    );
    for (btn, want) in [
        (Btn::Down, "Game Boy"),
        (Btn::Down, "Game Boy Color"),
        (Btn::Down, "Game Boy Advance"),
        (Btn::Up, "Game Boy Color"),
    ] {
        a.apply_at(Action::GbaDown(btn), 1_000);
        assert_eq!(
            a.shelf_platform_name(),
            Some(want),
            "{btn:?} left the wrong name on the case"
        );
        assert_eq!(a.toast(), None, "{btn:?} put a banner up over the carts");
    }
    a.update(5.0);
    assert_eq!(
        a.shelf_platform_name(),
        Some("Game Boy Color"),
        "the name faded"
    );
}

#[test]
fn a_card_on_one_shelf_names_nothing_because_there_is_nowhere_to_switch_to() {
    let mut alone = app_with_platforms(&[(Platform::Gba, "Emerald"), (Platform::Gba, "Fusion")]);
    assert_eq!(
        alone.shelf_platform_name(),
        None,
        "a card with only Game Boy Advance carts named its platform anyway"
    );
    for btn in [Btn::Up, Btn::Down] {
        alone.apply_at(Action::GbaDown(btn), 1_000);
        assert_eq!(
            alone.shelf_platform_name(),
            None,
            "{btn:?} named a shelf on a card that has one"
        );
    }

    let two = app_with_platforms(&[(Platform::Gba, "Emerald"), (Platform::Gb, "Tetris")]);
    assert_eq!(
        two.shelf_platform_name(),
        Some("Game Boy Advance"),
        "a card with two shelves did not say which one it was on"
    );
}

#[test]
fn the_carousel_opens_on_a_shelf_that_has_carts() {
    let a = app_with_platforms(&[(Platform::Gbc, "Chromatic")]);
    assert_eq!(a.selected_stem(), Some("Chromatic"));
}

#[test]
fn a_held_direction_does_not_follow_the_shelf_it_was_pressed_on() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Aaa"),
        (Platform::Gba, "Bbb"),
        (Platform::Gba, "Ccc"),
        (Platform::Gb, "Tetris"),
    ]);
    a.apply_at(Action::GbaDown(Btn::Right), 0);
    assert_eq!(a.selected_stem(), Some("Bbb"));
    a.apply_at(Action::GbaDown(Btn::Down), 10);
    a.apply_at(Action::GbaDown(Btn::Up), 20);
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert_eq!(
        a.selected_stem(),
        Some("Bbb"),
        "the row walked on by itself once the shelf came back"
    );
}

#[test]
fn a_cart_in_flight_is_not_also_left_standing_on_the_shelf() {
    let mut a = app_with_carts(&["Emerald"]);
    play(&mut a);
    a.update(0.2);
    let mut out = Vec::new();
    a.draw(&mut out);
    let carts = out
        .iter()
        .filter(|d| match **d {
            Draw::Rect { w, h, .. } | Draw::Tex { w, h, .. } | Draw::Turned { w, h, .. } => {
                w > 200.0
                    && w <= CART_W as f32 + 0.01
                    && (w / h - CART_W as f32 / CART_H as f32).abs() < 0.05
            }
        })
        .count();
    assert_eq!(
        carts, 1,
        "the shelf still holds the cart the slot is taking"
    );
}

#[test]
fn the_shelf_shows_the_empty_slot() {
    let a = app_with_carts(&["Emerald", "Zzz"]);
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        out.iter().any(is_mouth),
        "the shelf has no slot, so the cart has nowhere visible to go"
    );
}

#[test]
fn inserting_still_has_a_mouth_to_go_into() {
    let mut a = app_with_carts(&["Emerald"]);
    play(&mut a);
    a.update(1.0 / 60.0);
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        out.iter().any(is_mouth),
        "the cart has nothing to slide into"
    );
}

#[test]
fn both_b_and_menu_take_the_about_screen_back_to_the_quick_menu() {
    let d = common::tmp_root_with_carts(&["Emerald", "Fusion"]);
    for out in [
        shelf_input::Action::GbaDown(shelf_input::Btn::B),
        shelf_input::Action::QuickMenu,
    ] {
        let mut s = common::session_with_platform(d.path());
        s.app_mut().apply(shelf_input::Action::QuickMenu);
        for _ in 0..shelf_ui::QuickRow::About.index() {
            s.app_mut()
                .apply(shelf_input::Action::GbaDown(shelf_input::Btn::Down));
        }
        s.app_mut()
            .apply(shelf_input::Action::GbaDown(shelf_input::Btn::A));
        assert!(matches!(s.app().phase(), shelf::app::Phase::About));
        s.app_mut().apply(out);
        assert_eq!(
            s.app().quick_menu(),
            Some(shelf_ui::QuickRow::About),
            "{out:?} did not take the label back to the menu"
        );
    }
}

fn frame(app: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    app.draw(&mut out);
    out
}

fn tex_at(out: &[Draw], want: TexId) -> Option<(usize, [f32; 4])> {
    out.iter().enumerate().find_map(|(i, d)| match *d {
        Draw::Tex {
            x, y, w, h, tex, ..
        } if tex == want => Some((i, [x, y, w, h])),
        _ => None,
    })
}

fn slot_text_alpha(a: &App, tex: shelf_ui::TexId) -> f32 {
    let mut out = Vec::new();
    a.draw(&mut out);
    out.iter()
        .find_map(|d| match *d {
            Draw::Tex { tex: t, alpha, .. } if t == tex => Some(alpha),
            _ => None,
        })
        .unwrap_or(0.0)
}

#[test]
fn the_shelf_name_stays_in_the_slot_and_comes_back_after_a_letter() {
    let mut a = app_with_platforms(&[
        (Platform::Gba, "Alpha"),
        (Platform::Gba, "Bravo"),
        (Platform::Gb, "Tetris"),
    ]);
    let name = shelf_ui::TexId::from_raw(78);
    assert_eq!(a.slot_text().as_deref(), Some("Game Boy Advance"));
    a.set_shelf_platform_face(name, 20);
    a.update(1.0 / 60.0);
    a.update(30.0);
    assert!(slot_text_alpha(&a, name) > 0.0, "the shelf name faded away");

    a.apply(Action::GbaDown(Btn::R1));
    assert_eq!(a.slot_text().as_deref(), Some("B"));
    a.set_shelf_platform_face(name, 20);
    a.update(1.0 / 60.0);
    a.update(5.0);
    a.update(1.0 / 60.0);
    assert_eq!(
        a.slot_text().as_deref(),
        Some("Game Boy Advance"),
        "the shelf name did not come back after the letter"
    );
    a.set_shelf_platform_face(name, 20);
    a.update(1.0 / 60.0);
    a.update(1.0);
    assert!(
        slot_text_alpha(&a, name) > 0.0,
        "the shelf name was not shown again"
    );
}

#[test]
fn a_letter_jump_shows_the_letter_in_the_slot_and_fades_like_the_shelf_name() {
    let mut a = app_with_carts(&["Alpha", "Bravo"]);
    let face = shelf_ui::TexId::from_raw(77);
    assert_eq!(
        a.slot_text(),
        None,
        "a one shelf card printed something in the slot"
    );

    a.apply(Action::GbaDown(Btn::R1));
    assert_eq!(a.slot_text().as_deref(), Some("B"));
    a.set_shelf_platform_face(face, 20);
    a.update(1.0 / 60.0);
    a.update(0.5);
    assert!(slot_text_alpha(&a, face) > 0.0, "the letter never showed");
    a.update(5.0);
    assert_eq!(slot_text_alpha(&a, face), 0.0, "the letter never faded");

    a.apply(Action::GbaDown(Btn::Left));
    a.apply(Action::GbaDown(Btn::R1));
    assert_eq!(a.slot_text().as_deref(), Some("B"));
    a.set_shelf_platform_face(face, 20);
    a.update(1.0 / 60.0);
    a.update(0.5);
    assert!(
        slot_text_alpha(&a, face) > 0.0,
        "jumping to the letter already printed did not show it again"
    );
}

#[test]
fn x_blows_on_the_middle_cart_and_the_dust_settles() {
    let mut a = app_with_carts(&["Alpha", "Bravo", "Charlie"]);
    let faces: Vec<TexId> = (0..3).map(|i| TexId::from_raw(900 + i)).collect();
    a.set_faces(faces.clone());
    for _ in 0..60 {
        a.update(1.0 / 60.0);
    }
    let still = tex_at(&frame(&a), faces[0]).expect("no middle cart").1;
    let side = tex_at(&frame(&a), faces[1]).expect("no side cart").1;
    let specks = |out: &[Draw]| {
        out.iter()
            .filter(|d| matches!(d, Draw::Rect { w, h, .. } if *w < 16.0 && *h < 16.0))
            .count()
    };
    let before = specks(&frame(&a));

    a.apply(Action::GbaDown(Btn::X));
    assert!(a.blow().is_some(), "X did not blow on the cart");
    let gold = |out: &[Draw]| {
        out.iter()
            .filter(|d| {
                matches!(*d, Draw::Rect { colour, .. }
                    if colour[3] > 0.9 && colour[0] > 0.8 && colour[1] > 0.6
                        && colour[1] < 0.75 && colour[2] < 0.4)
            })
            .count()
    };
    let gold_before = gold(&frame(&a));
    let puffed = |out: &[Draw]| {
        out.iter().any(|d| {
            matches!(*d, Draw::Rect { w, colour, .. }
                if w > 40.0 && colour[3] > 0.05 && colour[3] < 0.4
                    && colour[0] > 0.9 && colour[2] > 0.95)
        })
    };
    let mut moved = false;
    let mut dusty = false;
    let mut tipped = false;
    let mut breathed = false;
    for _ in 0..60 {
        a.update(1.0 / 60.0);
        let out = frame(&a);
        let now = tex_at(&out, faces[0]).unwrap().1;
        moved |= now != still;
        dusty |= specks(&out) > before + 8;
        let (x, y, w, h) = now.into();
        let bottom = out.iter().any(|d| {
            matches!(*d, Draw::Rect { x: bx, y: by, w: bw, h: bh, colour }
                if colour[3] > 0.9 && (bx - x).abs() < 1.0 && (by - (y + h)).abs() < 1.0
                    && (bw - w).abs() < 1.0 && bh > 10.0)
        });
        tipped |= h < still[3] * 0.7 && bottom && gold(&out) >= gold_before + 16;
        breathed |= puffed(&out);
        assert_eq!(
            tex_at(&out, faces[1]).unwrap().1,
            side,
            "a cart beside it moved"
        );
    }
    assert!(tipped, "the cart never tipped up to show its contacts");
    assert!(breathed, "no breath went over it");
    assert!(moved, "the cart did not budge");
    assert!(dusty, "no dust came off it");
    assert_eq!(a.selected_stem(), Some("Alpha"), "blowing moved the shelf");
    assert!(matches!(a.phase(), Phase::Shelf), "blowing put the cart in");

    for _ in 0..60 {
        a.update(1.0 / 60.0);
    }
    assert!(a.blow().is_none(), "the puff never ended");
    assert_eq!(
        gold(&frame(&a)),
        gold_before,
        "the cart's bottom is still showing"
    );
    assert_eq!(
        tex_at(&frame(&a), faces[0]).unwrap().1,
        still,
        "the cart did not settle back where it was"
    );
}

#[test]
fn y_shows_the_button_guide_and_any_button_puts_it_away() {
    let mut a = app_with_carts(&["Alpha", "Bravo", "Charlie"]);
    let lines: Vec<TexId> = (0..shelf_ui::HelpLine::ALL.len())
        .map(|i| TexId::from_raw(950 + i))
        .collect();
    a.set_help_faces(lines.iter().map(|&t| (t, 160)).collect());
    a.update(1.0 / 60.0);
    let shown = |a: &App| {
        let out = frame(a);
        lines.iter().filter(|&&t| tex_at(&out, t).is_some()).count()
    };
    assert_eq!(shown(&a), 0, "the guide is up before Y");

    a.apply(Action::GbaDown(Btn::Y));
    assert!(a.help_open(), "Y did not open the guide");
    let out = frame(&a);
    assert_eq!(shown(&a), 10, "the guide does not list every button");
    let start = a.help_lines()[7];
    assert_eq!(start, shelf_ui::HelpLine::PowerMenu, "Start's line is wrong");
    let dim = out
        .iter()
        .position(|d| matches!(d, Draw::Rect { colour, .. } if *colour == shelf_ui::HELP_DIM))
        .expect("the shelf is not dimmed behind the guide");
    assert!(
        lines
            .iter()
            .filter_map(|&t| tex_at(&out, t))
            .all(|(i, _)| i > dim),
        "the guide is drawn under the dimming"
    );

    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaUp(Btn::A));
    a.update(1.0 / 60.0);
    assert!(!a.help_open(), "A did not close the guide");
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "closing the guide put a cart in"
    );
    assert_eq!(shown(&a), 0, "the guide is still drawn");

    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::Down));
    assert!(!a.help_open());
    assert_eq!(
        a.selected_stem(),
        Some("Alpha"),
        "closing the guide moved the shelf"
    );
}

#[test]
fn crt_mode_lays_scanlines_over_the_shelf() {
    let mut a = app_with_carts(&["Alpha", "Bravo"]);
    a.update(1.0 / 60.0);
    let scanlines = |a: &App| {
        frame(a)
            .iter()
            .filter(|d| matches!(d, Draw::Rect { h, colour, .. } if *h == 1.0 && *colour == shelf_ui::SCANLINE))
            .count()
    };
    assert_eq!(scanlines(&a), 0, "scanlines with CRT mode off");
    a.apply(Action::QuickMenu);
    for _ in 0..shelf_ui::QuickRow::Crt.index() {
        a.apply(Action::GbaDown(Btn::Down));
    }
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(
        a.quick_value(shelf_ui::QuickRow::Crt),
        Some(shelf_ui::QuickValue::flag(true)),
        "CRT Mode did not turn on"
    );
    a.apply(Action::QuickMenu);
    a.update(1.0 / 60.0);
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(
        scanlines(&a),
        (shelf_gfx::OUT_H / shelf_ui::SCANLINE_PITCH) as usize,
        "no scanlines over the shelf"
    );
}

#[test]
fn b_turns_the_cart_round_to_its_box_and_back() {
    let carts = ["Alpha", "Bravo"]
        .iter()
        .map(|stem| Cart {
            platform: Platform::Gba,
            stem: (*stem).to_string(),
            rom: format!("Roms/GBA/{stem}.gba").into(),
            label: Some(format!("Roms/GBA/.media/{stem}.png").into()),
            code: String::new(),
            shell: None,
            title: stem.to_uppercase(),
        })
        .collect();
    let mut a = App::new(carts);
    let face = TexId::from_raw(990);
    a.update(1.0 / 60.0);

    a.apply(Action::GbaDown(Btn::B));
    let want = a.wanted_box_art().map(|p| p.to_path_buf());
    assert_eq!(
        want.as_deref(),
        Some(Path::new("Roms/GBA/.media/Alpha.png")),
        "B did not ask for the box art"
    );
    a.set_box_face(want.unwrap(), face, 200, 280);
    let mut halfway = false;
    for _ in 0..40 {
        a.update(1.0 / 60.0);
        let out = frame(&a);
        if let Some((_, [_, _, w, _])) = tex_at(&out, face) {
            halfway |= w < 150.0;
        }
    }
    assert!(halfway, "the box did not turn in");
    let (_, [_, _, w, h]) = tex_at(&frame(&a), face).expect("no box after the turn");
    assert!(
        (w - 200.0).abs() < 1.0 && (h - 280.0).abs() < 1.0,
        "box is {w}x{h}"
    );

    a.apply(Action::GbaDown(Btn::B));
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
    assert!(
        a.box_view().is_some(),
        "B on the front did not turn to the back"
    );
    assert!(
        a.wanted_box_back().is_none(),
        "asked for a back scan the card does not have"
    );
    assert!(
        tex_at(&frame(&a), face).is_none(),
        "the front still shows on the back"
    );

    a.apply(Action::GbaDown(Btn::B));
    for _ in 0..40 {
        a.update(1.0 / 60.0);
    }
    assert!(a.box_view().is_none(), "the cart did not turn back");
    assert!(tex_at(&frame(&a), face).is_none(), "the box is still drawn");
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn b_with_no_box_art_shakes_its_head() {
    let mut a = app_with_carts(&["Alpha", "Bravo"]);
    a.update(1.0 / 60.0);
    a.apply(Action::GbaDown(Btn::B));
    assert!(a.box_view().is_none(), "box view opened with no box art");
    assert!(
        a.refusal_active(a.now()),
        "no shake for a cart without a box"
    );
}

#[test]
fn select_spins_the_shelf_and_stops_on_a_random_cart() {
    let stems: Vec<String> = (0..20).map(|i| format!("Game {i:02}")).collect();
    let refs: Vec<&str> = stems.iter().map(String::as_str).collect();
    let mut a = app_with_carts(&refs);
    a.update(1.0 / 60.0);
    let start = a.selected_stem().unwrap().to_string();

    a.apply(Action::GbaDown(Btn::Select));
    let spin = a.spin().expect("Select did not spin the shelf");
    let target = stems[spin.target()].clone();
    assert_ne!(target, start, "it picked the cart already in the middle");

    let mut seen = std::collections::HashSet::new();
    for _ in 0..30 {
        a.update(1.0 / 60.0);
        seen.insert(a.selected_stem().unwrap().to_string());
    }
    a.apply(Action::GbaDown(Btn::Left));
    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaUp(Btn::A));
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "A put a cart in mid-spin"
    );
    for _ in 0..(shelf_ui::SPIN_MS as usize * 60 / 1000 + 30) {
        a.update(1.0 / 60.0);
        seen.insert(a.selected_stem().unwrap().to_string());
    }
    assert!(seen.len() > 15, "the shelf hardly moved: {seen:?}");
    assert!(a.spin().is_none(), "the shelf never stopped");
    assert_eq!(
        a.selected_stem(),
        Some(target.as_str()),
        "it stopped on the wrong cart"
    );
}

#[test]
fn the_recently_played_shelf_sits_above_the_first_console() {
    let mut a = app_with_carts(&["Alpha", "Bravo", "Charlie"]);
    a.set_recent(&[
        "Games/GBA/Charlie.gba".into(),
        "Games/GBA/Gone.gba".into(),
        "Games/GBA/Alpha.gba".into(),
    ]);
    a.update(1.0 / 60.0);
    assert_eq!(a.shelf_key(), ShelfKey::Platform(Platform::Gba));
    assert_eq!(a.selected_stem(), Some("Alpha"));

    a.apply(Action::GbaDown(Btn::Up));
    assert_eq!(
        a.shelf_key(),
        ShelfKey::Recent,
        "Up did not reach the recent shelf"
    );
    assert_eq!(a.shelf_platform_name(), Some("Recently Played"));
    assert_eq!(a.selected_stem(), Some("Charlie"), "newest is not first");
    a.apply(Action::GbaDown(Btn::Right));
    a.update(1.0 / 60.0);
    assert_eq!(
        a.selected_stem(),
        Some("Alpha"),
        "a missing rom was shelved"
    );

    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(a.shelf_key(), ShelfKey::Platform(Platform::Gba));
    assert_eq!(a.carts().count(), 3, "recent games counted twice");
}

#[test]
fn with_nothing_played_there_is_no_recent_shelf() {
    let mut a = app_with_carts(&["Alpha", "Bravo"]);
    a.update(1.0 / 60.0);
    a.apply(Action::GbaDown(Btn::Up));
    assert_eq!(a.shelf_key(), ShelfKey::Platform(Platform::Gba));
}
