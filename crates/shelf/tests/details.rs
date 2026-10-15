mod common;

use std::path::PathBuf;

use common::{session_with_platform, tmp_root_with_carts};
use shelf::session::Session;
use shelf_gfx::{Draw, TexId};
use shelf_input::{Action, Btn, Millis, RawEvent};

const ALPHA: &str = "name=Alpha Quest\nyear=1999\ngenre=Puzzle, Action\npublisher=Nintendo\n\
                     players=2\nrating=4.4\nvotes=1200\nesrb=E\noverview=A game of alphas.\n";

fn step(s: &mut Session, now: &mut Millis, evs: Vec<RawEvent>) {
    *now += 16;
    s.feed(evs, *now);
    s.update(1.0 / 60.0);
}

fn rom_of(s: &Session, stem: &str) -> PathBuf {
    s.app()
        .carts()
        .find(|c| c.stem == stem)
        .map(|c| c.rom.clone())
        .expect("cart")
}

fn card_with_details() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Alpha", "Bravo"]);
    let dir = d.path().join("Details").join("GBA");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Alpha.txt"), ALPHA).unwrap();
    d
}

fn tex_draws(s: &Session, tex: TexId) -> Vec<[f32; 4]> {
    let mut out = Vec::new();
    s.app().draw(&mut out);
    out.iter()
        .filter_map(|d| match *d {
            Draw::Tex {
                x, y, w, h, tex: t, ..
            } if t == tex => Some([x, y, w, h]),
            _ => None,
        })
        .collect()
}

#[test]
fn details_are_read_only_once_the_shelf_rests_and_are_kept() {
    let d = card_with_details();
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    let alpha = rom_of(&s, "Alpha");
    let bravo = rom_of(&s, "Bravo");
    assert!(
        s.app().wanted_details_line().is_none(),
        "details were read before the shelf rested"
    );

    // Resting on Bravo does not read Alpha's file, which is not the middle cart.
    s.app_mut().load_details_of(&bravo);
    s.app_mut().load_details_of(&alpha);
    assert!(s.app().details_of(&bravo).is_none(), "Bravo has no file");
    let middle = s
        .app()
        .wanted_details_line()
        .expect("the middle cart's details");
    assert_eq!(middle.0, alpha);
    assert_eq!(middle.1.year, Some(1999));

    // It is read once: the file going away does not take the details.
    std::fs::remove_file(d.path().join("Details/GBA/Alpha.txt")).unwrap();
    s.app_mut().load_details_of(&alpha);
    assert!(
        s.app().details_of(&alpha).is_some(),
        "the details were not kept"
    );
}

#[test]
fn the_line_is_printed_under_the_name_and_follows_the_cart() {
    let d = card_with_details();
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    let alpha = rom_of(&s, "Alpha");
    let name = TexId::from_raw(4242);
    let line = TexId::from_raw(4243);
    let want = s.app().wanted_game_name().expect("name");
    s.app_mut().set_game_name_face(want, name);
    s.app_mut().set_details_line_face(alpha.clone(), line);
    assert!(
        tex_draws(&s, line).is_empty(),
        "a line was drawn for details that were not read"
    );

    s.app_mut().load_details_of(&alpha);
    let [_, name_y, _, name_h] = tex_draws(&s, name)[0];
    let [_, y, _, h] = *tex_draws(&s, line).first().expect("no line drawn");
    let top = shelf_ui::rest_y(shelf_ui::CART_H as f32);
    assert!(
        y > name_y && y < name_y + name_h,
        "the line is not under the name"
    );
    assert!(
        y + h <= top,
        "the line at {y} overlaps the cart top at {top}"
    );

    step(&mut s, &mut now, vec![RawEvent::Down(Btn::Right)]);
    step(&mut s, &mut now, vec![RawEvent::Up(Btn::Right)]);
    for _ in 0..30 {
        step(&mut s, &mut now, vec![]);
    }
    assert!(tex_draws(&s, line).is_empty(), "the old line stayed up");
    // Bravo has no details, so there is no line for it.
    let bravo_rom = rom_of(&s, "Bravo");
    s.app_mut().load_details_of(&bravo_rom);
    assert!(s.app().wanted_details_line().is_none());
}

#[test]
fn the_box_back_is_made_only_for_a_game_with_details() {
    let d = card_with_details();
    let media = d.path().join("Games/GBA/.media");
    std::fs::create_dir_all(&media).unwrap();
    for stem in ["Alpha", "Bravo"] {
        std::fs::write(media.join(format!("{stem}.png")), b"x").unwrap();
    }
    let mut s = session_with_platform(d.path());
    let mut now: Millis = 0;
    for _ in 0..60 {
        step(&mut s, &mut now, vec![]);
    }
    let front = TexId::from_raw(777);
    let back = TexId::from_raw(778);

    // B on Alpha reads its details, and box view then wants a back made.
    s.app_mut().apply(Action::GbaDown(Btn::B));
    let art = s.app().wanted_box_art().expect("box art").to_path_buf();
    assert!(
        s.app().back_details_key().is_none(),
        "made before the front is in"
    );
    s.app_mut().set_box_face(art, front, 200, 280);
    let key = s.app().back_details_key().expect("a back to make");
    assert!(!key.strip, "no scan, so a whole back");
    assert_eq!((key.w, key.h), (200, 280));
    s.app_mut().set_back_details_face(key, back, 400, 560);

    for _ in 0..60 {
        s.update(1.0 / 60.0);
    }
    s.app_mut().apply(Action::GbaDown(Btn::B));
    for _ in 0..60 {
        s.update(1.0 / 60.0);
    }
    assert!(
        !tex_draws(&s, back).is_empty(),
        "the made back was not shown when the box turned round"
    );

    // Bravo has no file: nothing to make, and the plain back shows.
    s.app_mut().apply(Action::GbaDown(Btn::B));
    for _ in 0..60 {
        s.update(1.0 / 60.0);
    }
    assert!(s.app().box_view().is_none() || s.app().back_details_key().is_some());
    s.app_mut().apply(Action::GbaDown(Btn::Right));
    for _ in 0..60 {
        s.update(1.0 / 60.0);
    }
    s.app_mut().apply(Action::GbaDown(Btn::B));
    let art = s.app().wanted_box_art().expect("box art").to_path_buf();
    s.app_mut().set_box_face(art, front, 200, 280);
    assert!(
        s.app().back_details_key().is_none(),
        "a back was made for a game with no details"
    );
}
