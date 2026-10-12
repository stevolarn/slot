mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::{App, FaceKind, Phase, SectionFaces, ShelfKey};
use shelf_gfx::Draw;
use shelf_input::{Action, Btn};
use shelf_store::{
    read_shelf_state, write_shelf_state, Cart, DisplayMode, Platform, ShelfState, SortKey,
};
use shelf_ui::{HelpLine, QuickRow, TexId, Toast, SIGN_IN_MS};

const STEMS: [&str; 5] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo"];

/// What the card knows of each game: year, genre, publisher, rating.
const DETAILS: [(&str, &str); 4] = [
    (
        "Alpha",
        "year=1999\ngenre=Racing\npublisher=Sega\nrating=3.2\n",
    ),
    (
        "Bravo",
        "year=1994\ngenre=Action\npublisher=Nintendo\nrating=4.7\n",
    ),
    (
        "Charlie",
        "year=1996\ngenre=Action\npublisher=Capcom\nrating=4.1\n",
    ),
    (
        "Echo",
        "year=2003\ngenre=RPG\npublisher=Atari\nrating=3.9\n",
    ),
];

fn card_with(state: ShelfState) -> tempfile::TempDir {
    let d = tmp_root_with_carts(&STEMS);
    let dir = d.path().join("Details").join("GBA");
    std::fs::create_dir_all(&dir).unwrap();
    for (stem, text) in DETAILS {
        std::fs::write(dir.join(format!("{stem}.txt")), text).unwrap();
    }
    // Boxes for all of them but Delta.
    let media = d.path().join("Games/GBA/.media");
    std::fs::create_dir_all(&media).unwrap();
    for stem in STEMS.iter().filter(|s| **s != "Delta") {
        std::fs::write(media.join(format!("{stem}.png")), b"x").unwrap();
    }
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..state
        },
    )
    .unwrap();
    d
}

fn boot(state: ShelfState) -> (tempfile::TempDir, App) {
    let d = card_with(state);
    let (a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    (d, a)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

/// From the cart shelf to the store: L2 steps on through the box shelf.
fn to_store(a: &mut App) {
    press(a, Btn::L2);
    press(a, Btn::L2);
    assert_eq!(a.display(), DisplayMode::Store);
}

fn run(a: &mut App, frames: usize) {
    for _ in 0..frames {
        a.update(1.0 / 60.0);
    }
}

/// Run until the details have been read and the shelves are sorted by them.
fn until_read(a: &mut App) {
    for _ in 0..1000 {
        a.update(1.0 / 60.0);
        if !a.animating() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!("the details were never read");
}

fn stems(a: &App) -> Vec<String> {
    a.carts().map(|c| c.stem.clone()).collect()
}

fn frame(a: &App) -> Vec<Draw> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out
}

fn tex_rects(out: &[Draw], tex: TexId) -> Vec<(f32, f32, f32, f32)> {
    out.iter()
        .filter_map(|d| match *d {
            Draw::Tex {
                x, y, w, h, tex: t, ..
            } if t == tex => Some((x, y, w, h)),
            _ => None,
        })
        .collect()
}

fn box_faces(a: &mut App) {
    let n = a.carts().count();
    for i in 0..n {
        a.set_tall_face(0, i, TexId::from_raw(300 + i), 140, 180);
    }
}

fn state_on_disk(d: &Path) -> ShelfState {
    read_shelf_state(d)
}

#[test]
fn l2_steps_through_the_displays_keeps_the_game_and_remembers_it() {
    let (d, mut a) = boot(ShelfState::default());
    assert_eq!(a.display(), DisplayMode::Carts);
    press(&mut a, Btn::Right);
    press(&mut a, Btn::Right);
    run(&mut a, 60);
    let game = a.selected_stem().map(str::to_string);
    assert_eq!(game.as_deref(), Some("Charlie"));

    // Carts, then boxes, then the store, then carts again.
    for mode in [DisplayMode::Boxes, DisplayMode::Store, DisplayMode::Carts] {
        press(&mut a, Btn::L2);
        assert_eq!(a.display(), mode);
        assert_eq!(a.selected_stem(), game.as_deref(), "the game moved");
        assert_eq!(state_on_disk(d.path()).display, mode);
        assert_eq!(a.toast(), Some(Toast::Display(mode)));
    }
}

#[test]
fn the_saved_display_and_sort_come_back_at_boot_in_order_at_once() {
    let (_d, a) = boot(ShelfState {
        display: DisplayMode::Store,
        sort: SortKey::Year,
        ..ShelfState::default()
    });
    assert_eq!(a.display(), DisplayMode::Store);
    assert_eq!(a.sort(), SortKey::Year);
    // The shelf being shown is read before it is first seen: Bravo 1994,
    // Charlie 1996, Alpha 1999, Echo 2003, and Delta, with no year, last.
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Alpha", "Echo", "Delta"]);
}

#[test]
fn the_store_draws_box_pictures_and_the_sign_not_carts() {
    let (_d, mut a) = boot(ShelfState::default());
    for i in 0..5 {
        a.set_cart_face(0, i, TexId::from_raw(100 + i));
    }
    to_store(&mut a);
    box_faces(&mut a);
    a.set_section_faces(SectionFaces {
        label: "A".into(),
        sign: (TexId::from_raw(700), 90, 28),
        tab: (TexId::from_raw(701), 60, 16),
    });
    run(&mut a, 120);
    let out = frame(&a);
    assert!(
        !tex_rects(&out, TexId::from_raw(300)).is_empty(),
        "the middle box was not drawn"
    );
    for i in 0..5 {
        assert!(
            tex_rects(&out, TexId::from_raw(100 + i)).is_empty(),
            "cart {i}'s face was drawn in the store"
        );
    }
    assert!(
        !tex_rects(&out, TexId::from_raw(700)).is_empty(),
        "no aisle sign"
    );
    assert!(
        tex_rects(&out, TexId::from_raw(701)).is_empty(),
        "the cart shelf's tab was drawn in the store"
    );
    // Back on the carts, the carts show and the boxes do not.
    press(&mut a, Btn::L2);
    run(&mut a, 120);
    let out = frame(&a);
    assert!(!tex_rects(&out, TexId::from_raw(100)).is_empty());
    assert!(tex_rects(&out, TexId::from_raw(300)).is_empty());
}

#[test]
fn a_box_with_no_picture_stands_as_a_plain_panel() {
    let (_d, mut a) = boot(ShelfState::default());
    to_store(&mut a);
    run(&mut a, 120);
    let out = frame(&a);
    assert!(
        out.iter().any(|d| matches!(d, Draw::Rect { w, h, colour, .. }
            if *h > 200.0 && *w > 100.0 && colour[3] == 1.0)),
        "no panel for the middle box"
    );
}

#[test]
fn the_faces_asked_for_are_boxes_in_the_store_and_carts_on_the_shelf() {
    let (_d, mut a) = boot(ShelfState::default());
    let wants = a.face_wants();
    assert!(!wants.is_empty());
    assert!(wants.iter().all(|w| w.kind == FaceKind::Cart));

    to_store(&mut a);
    assert_eq!(a.face_kind(), FaceKind::Tall);
    let wants = a.face_wants();
    assert!(!wants.is_empty(), "nothing asked for in the store");
    assert!(wants.iter().any(|w| w.kind == FaceKind::Tall));

    // A cart's face does not do for a box.
    a.set_cart_face(0, 0, TexId::from_raw(1));
    assert!(a.face_wants().iter().any(|w| w.index == 0));
    a.set_tall_face(0, 0, TexId::from_raw(2), 100, 140);
    assert!(a.face_wants().iter().all(|w| w.index != 0));

    // With loose carts on the store's shelf, carts are wanted.
    press(&mut a, Btn::X);
    assert_eq!(a.face_kind(), FaceKind::Cart);
    press(&mut a, Btn::X);

    press(&mut a, Btn::L2);
    assert!(a.face_wants().iter().all(|w| w.kind == FaceKind::Cart));
}

#[test]
fn what_is_in_flight_is_known_to_be_stale_when_the_shelf_changes() {
    let (_d, mut a) = boot(ShelfState::default());
    let before = a.face_epoch();
    to_store(&mut a);
    assert_ne!(
        a.face_epoch(),
        before,
        "showing the store left the faces alone"
    );
    let before = a.face_epoch();
    press(&mut a, Btn::R2);
    until_read(&mut a);
    assert_ne!(a.face_epoch(), before, "sorting left the faces alone");
}

#[test]
fn faces_of_the_kind_not_shown_are_shed_down_to_the_screen() {
    let carts: Vec<Cart> = (0..60)
        .map(|i| Cart {
            platform: Platform::Gba,
            stem: format!("G{i:04}"),
            rom: format!("/x/G{i:04}.gba").into(),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
        })
        .collect();
    let mut a = App::new(carts);
    for i in [0usize, 2, 8, 30] {
        a.set_cart_face(0, i, TexId::from_raw(10 + i));
        a.set_tall_face(0, i, TexId::from_raw(100 + i), 100, 140);
    }
    to_store(&mut a);
    let mut shed: Vec<TexId> = a.shed_faces();
    shed.sort_by_key(|t| format!("{t:?}"));
    // The store's large box fronts are kept only near the middle (0, 2);
    // further ones (8, 30) go; of the carts only those on the screen stay
    // (0 and 2).
    let kept = |i: usize| !shed.contains(&TexId::from_raw(100 + i));
    assert!(kept(0) && kept(2) && !kept(8) && !kept(30), "{shed:?}");
    let cart_kept = |i: usize| !shed.contains(&TexId::from_raw(10 + i));
    assert!(
        cart_kept(0) && cart_kept(2) && !cart_kept(8) && !cart_kept(30),
        "{shed:?}"
    );
}

#[test]
fn r2_cycles_the_sort_toasts_it_and_remembers_it() {
    let (d, mut a) = boot(ShelfState::default());
    assert_eq!(a.sort(), SortKey::Name);
    let mut seen = Vec::new();
    for _ in 0..5 {
        press(&mut a, Btn::R2);
        seen.push(a.sort());
        assert_eq!(a.toast(), Some(Toast::Sorted(a.sort())));
        assert_eq!(state_on_disk(d.path()).sort, a.sort());
        until_read(&mut a);
    }
    assert_eq!(
        seen,
        [
            SortKey::Genre,
            SortKey::Year,
            SortKey::Publisher,
            SortKey::Rating,
            SortKey::Name
        ]
    );
    let text = std::fs::read_to_string(d.path().join("Config/shelf.state")).unwrap();
    assert!(text.lines().any(|l| l == "sort=name"), "{text}");
}

#[test]
fn sorting_orders_the_shelf_by_the_details_and_keeps_the_selected_game() {
    let (_d, mut a) = boot(ShelfState::default());
    press(&mut a, Btn::Right);
    press(&mut a, Btn::Right);
    run(&mut a, 60);
    assert_eq!(a.selected_stem(), Some("Charlie"));

    press(&mut a, Btn::R2); // genre
    until_read(&mut a);
    // Action (Bravo, Charlie), Racing, RPG, then Delta with no details.
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Alpha", "Echo", "Delta"]);
    assert_eq!(a.selected_stem(), Some("Charlie"), "the game moved");

    press(&mut a, Btn::R2); // year
    until_read(&mut a);
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Alpha", "Echo", "Delta"]);
    press(&mut a, Btn::R2); // publisher
    until_read(&mut a);
    assert_eq!(stems(&a), ["Echo", "Charlie", "Bravo", "Alpha", "Delta"]);
    assert_eq!(a.selected_stem(), Some("Charlie"));
    press(&mut a, Btn::R2); // rating
    until_read(&mut a);
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Echo", "Alpha", "Delta"]);
    press(&mut a, Btn::R2); // name again
    until_read(&mut a);
    assert_eq!(stems(&a), STEMS);
    assert_eq!(a.selected_stem(), Some("Charlie"));
}

#[test]
fn sections_name_the_aisle_the_middle_game_stands_in() {
    let (_d, mut a) = boot(ShelfState::default());
    // By name, a small shelf is sectioned by letter; the cart shelf shows
    // no tab for it.
    assert_eq!(a.current_section().as_deref(), Some("A"));
    assert_eq!(a.wanted_section(), None);
    // By genre, still on Alpha, which is a Racing game.
    press(&mut a, Btn::R2);
    until_read(&mut a);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    assert_eq!(a.current_section().as_deref(), Some("RACING"));
    assert_eq!(a.wanted_section().as_deref(), Some("RACING"), "no tab");
    press(&mut a, Btn::Left);
    assert_eq!(a.selected_stem(), Some("Charlie"));
    assert_eq!(a.current_section().as_deref(), Some("ACTION"));
    press(&mut a, Btn::Right);
    // By year, Alpha is 1999.
    press(&mut a, Btn::R2);
    until_read(&mut a);
    assert_eq!(a.current_section().as_deref(), Some("1999"));
    // In the store the sign is wanted whatever the sort.
    press(&mut a, Btn::R2);
    press(&mut a, Btn::R2);
    press(&mut a, Btn::R2);
    until_read(&mut a);
    assert_eq!(a.sort(), SortKey::Name);
    assert_eq!(a.wanted_section(), None);
    to_store(&mut a);
    assert_eq!(a.wanted_section().as_deref(), Some("A"));
}

#[test]
fn l_and_r_jump_a_whole_section_when_the_shelf_is_sorted() {
    let (_d, mut a) = boot(ShelfState::default());
    press(&mut a, Btn::R2); // genre: Bravo Charlie | Alpha | Echo | Delta
    until_read(&mut a);
    assert_eq!(stems(&a), ["Bravo", "Charlie", "Alpha", "Echo", "Delta"]);
    // Alpha was the middle game by name; it stays so, in its own section.
    assert_eq!(a.selected_stem(), Some("Alpha"));
    press(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Echo"));
    press(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Delta"));
    press(&mut a, Btn::R1);
    assert_eq!(a.selected_stem(), Some("Bravo"), "the jump did not wrap");
    press(&mut a, Btn::L1);
    assert_eq!(a.selected_stem(), Some("Delta"));
    // The slot says where the jump landed.
    assert_eq!(a.slot_text().as_deref(), Some("OTHER"));
}

#[test]
fn recently_played_keeps_the_order_games_were_played_in() {
    let carts: Vec<Cart> = ["Alpha", "Bravo", "Charlie"]
        .into_iter()
        .map(|stem| Cart {
            platform: Platform::Gba,
            stem: stem.into(),
            rom: format!("/x/{stem}.gba").into(),
            label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
        })
        .collect();
    let mut a = App::new(carts);
    a.set_recent(&["/x/Charlie.gba".into(), "/x/Alpha.gba".into()]);
    while a.shelf_key() != ShelfKey::Recent {
        press(&mut a, Btn::Down);
    }
    let tour = |a: &mut App| -> Vec<String> {
        let mut out = Vec::new();
        for _ in 0..2 {
            out.push(a.selected_stem().unwrap().to_string());
            press(a, Btn::Right);
        }
        out
    };
    let before = tour(&mut a);
    assert_eq!(before, ["Charlie", "Alpha"]);
    for _ in 0..6 {
        press(&mut a, Btn::R2);
    }
    assert_eq!(tour(&mut a), before);
    assert_eq!(a.selected_stem(), Some("Charlie"));
    assert_eq!(a.current_section().as_deref(), Some("Recently Played"));
}

#[test]
fn the_options_menu_has_display_and_sort_rows_that_change_with_the_arrows() {
    let (d, mut a) = boot(ShelfState::default());
    a.apply(Action::QuickMenu);
    press(&mut a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::Display));
    assert_eq!(
        a.quick_value(QuickRow::Display),
        Some(shelf_ui::QuickValue::Carts)
    );
    // Right steps on to boxes, left back through carts to the store.
    press(&mut a, Btn::Right);
    assert_eq!(a.display(), DisplayMode::Boxes);
    assert_eq!(
        a.quick_value(QuickRow::Display),
        Some(shelf_ui::QuickValue::Boxes)
    );
    assert_eq!(state_on_disk(d.path()).display, DisplayMode::Boxes);
    press(&mut a, Btn::Left);
    assert_eq!(a.display(), DisplayMode::Carts);
    press(&mut a, Btn::Left);
    assert_eq!(a.display(), DisplayMode::Store);
    assert_eq!(
        a.quick_value(QuickRow::Display),
        Some(shelf_ui::QuickValue::Store)
    );
    assert_eq!(state_on_disk(d.path()).display, DisplayMode::Store);
    press(&mut a, Btn::Right);
    assert_eq!(a.display(), DisplayMode::Carts);

    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Sort));
    press(&mut a, Btn::Right);
    assert_eq!(a.sort(), SortKey::Genre);
    assert_eq!(
        a.quick_value(QuickRow::Sort),
        Some(shelf_ui::QuickValue::ByGenre)
    );
    press(&mut a, Btn::Right);
    assert_eq!(a.sort(), SortKey::Year);
    press(&mut a, Btn::Left);
    press(&mut a, Btn::Left);
    assert_eq!(a.sort(), SortKey::Name);
    assert_eq!(state_on_disk(d.path()).sort, SortKey::Name);
    // Neither row opens anything.
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::QuickMenu { .. }));
}

#[test]
fn the_guide_lists_l2_for_display_and_r2_for_sort() {
    let (_d, a) = boot(ShelfState::default());
    let lines = a.help_lines();
    assert!(lines.contains(&HelpLine::Display) && lines.contains(&HelpLine::Sort));
    assert_eq!(lines.len(), HelpLine::ALL.len());
}

#[test]
fn a_in_the_store_opens_the_box_and_a_again_slots_the_cart() {
    let (_d, mut a) = boot(ShelfState {
        display: DisplayMode::Store,
        ..ShelfState::default()
    });
    box_faces(&mut a);
    run(&mut a, 60);
    press(&mut a, Btn::A);
    assert!(a.peek().is_some(), "A did not open the box");
    assert!(a.handed_off().is_none(), "handed off without a slot");
    run(&mut a, 90);
    assert!(a.peek().is_some_and(|p| p.taken_out(a.now())));
    // The box is off the shelf while its cart is out.
    let out = frame(&a);
    assert!(tex_rects(&out, TexId::from_raw(300)).is_empty());
    press(&mut a, Btn::A);
    run(&mut a, 2);
    assert!(matches!(a.phase(), Phase::Inserting { .. }), "not slotted");
}

#[test]
fn b_in_the_store_turns_the_box_over_and_left_puts_it_back() {
    let (_d, mut a) = boot(ShelfState {
        display: DisplayMode::Store,
        ..ShelfState::default()
    });
    box_faces(&mut a);
    run(&mut a, 60);
    press(&mut a, Btn::B);
    assert!(a.peek().is_some(), "B did not take the box");
    run(&mut a, 60);
    assert!(a.peek().is_some_and(|p| p.on_back(a.now())));
    press(&mut a, Btn::B);
    run(&mut a, 120);
    assert_eq!(a.display(), DisplayMode::Store, "it left the store");
    assert!(matches!(a.phase(), Phase::Shelf));
}

#[test]
fn a_on_the_cart_shelf_still_takes_the_slot_animation() {
    let (_d, mut a) = boot(ShelfState::default());
    run(&mut a, 10);
    press(&mut a, Btn::A);
    run(&mut a, 2);
    assert!(a.handed_off().is_none(), "handed off with no animation");
    assert!(matches!(a.phase(), Phase::Inserting { .. }));
}

#[test]
fn x_in_the_store_swaps_its_shelf_between_boxes_and_carts_and_remembers() {
    let (d, mut a) = boot(ShelfState {
        display: DisplayMode::Store,
        ..ShelfState::default()
    });
    run(&mut a, 10);
    press(&mut a, Btn::X);
    run(&mut a, 5);
    assert!(a.blow().is_none(), "a box was blown on");
    assert!(!a.refusal_active(a.now()), "X shook the shelf");
    assert_eq!(a.toast(), Some(Toast::StoreShelf(true)));
    assert!(state_on_disk(d.path()).store_carts, "not remembered");
    press(&mut a, Btn::X);
    assert_eq!(a.toast(), Some(Toast::StoreShelf(false)));
}

#[test]
fn the_sign_keeps_frames_coming_while_it_swings_and_then_stops() {
    let (_d, mut a) = boot(ShelfState::default());
    run(&mut a, 10);
    assert!(!a.animating());
    to_store(&mut a);
    assert!(
        a.animating(),
        "the sign is swinging in and nothing asks for frames"
    );
    run(&mut a, (SIGN_IN_MS / 16 + 12) as usize);
    assert!(!a.animating(), "frames are still being asked for");
    // Back on the carts nothing keeps asking either.
    press(&mut a, Btn::L2);
    run(&mut a, 10);
    assert!(!a.animating());
}

#[test]
fn the_cart_shelf_tab_is_wanted_only_for_a_sort_other_than_name() {
    let (_d, mut a) = boot(ShelfState::default());
    assert_eq!(a.wanted_section(), None);
    press(&mut a, Btn::R2);
    until_read(&mut a);
    assert!(a.wanted_section().is_some());
    a.set_section_faces(SectionFaces {
        label: a.wanted_section().unwrap(),
        sign: (TexId::from_raw(710), 80, 28),
        tab: (TexId::from_raw(711), 60, 16),
    });
    run(&mut a, 60);
    let out = frame(&a);
    assert!(!tex_rects(&out, TexId::from_raw(711)).is_empty(), "no tab");
    assert!(
        tex_rects(&out, TexId::from_raw(710)).is_empty(),
        "a sign on the carts"
    );
}
