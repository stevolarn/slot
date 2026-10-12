use shelf_store::{Cart, Platform};
use shelf_ui::{
    accent_of, draw_sign, draw_store_backdrop, plain_box_face, sign_face, stat_tag_face,
    store_backdrop_face, store_box_face, store_ceiling, text, Draw, Shelf, StatTag, StatTier,
    TexId, OUT_H, OUT_W, PLAIN_W, STORE_BOX_H, STORE_BOX_W, STORE_FOOT, STORE_SLIVER, STORE_STAND, SIGN_IN_MS,
};

fn cart(i: usize) -> Cart {
    Cart {
        platform: Platform::Gba,
        stem: format!("Game {i}"),
        rom: format!("Games/GBA/Game {i}.gba").into(),
        label: None,
        code: String::new(),
        shell: None,
        title: format!("GAME {i}"),
    }
}

fn shelf_with(n: usize) -> Shelf {
    Shelf::new((0..n).map(cart).collect())
}

fn settle(s: &mut Shelf) {
    for _ in 0..600 {
        s.update(1.0 / 60.0);
    }
}

fn row(s: &Shelf, tags: &[TexId], hide: bool) -> Vec<Draw> {
    let mut out = Vec::new();
    let tag = |i: usize, _middle: bool| tags.get(i % tags.len().max(1)).map(|t| (*t, 60, 16));
    s.draw_store_row(store_ceiling(), false, &tag, hide, 1.0, 0.0, None, &mut out);
    out
}

/// Box fronts of a usual GBA box's shape on every game.
fn boxed(s: &mut Shelf, n: usize) {
    for i in 0..n {
        s.set_tall_face(i, TexId::from_raw(100 + i), 500, 500);
    }
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

#[test]
fn reordering_moves_each_carts_pictures_with_it_and_keeps_the_selection() {
    let mut s = shelf_with(4);
    for i in 0..4 {
        s.set_face(i, TexId::from_raw(10 + i));
        s.set_box_face(i, TexId::from_raw(20 + i), 100 + i as u32, 150);
    }
    s.select(2);
    // New order: 3, 2, 0, 1.
    s.reorder(&[3, 2, 0, 1]);
    let stems: Vec<&str> = s.carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(stems, ["Game 3", "Game 2", "Game 0", "Game 1"]);
    assert_eq!(s.index, 1, "the selection followed Game 2");
    assert_eq!(s.scroll, 1.0, "the scroll did not come to rest on it");
    assert_eq!(s.face(0), Some(TexId::from_raw(13)));
    assert_eq!(s.face(2), Some(TexId::from_raw(10)));
    assert_eq!(s.box_face(1), Some((TexId::from_raw(22), 102, 150)));
    assert_eq!(s.shapes().len(), 4);
}

#[test]
fn a_reorder_that_is_not_a_permutation_is_ignored() {
    let mut s = shelf_with(3);
    s.reorder(&[0, 0, 1]);
    s.reorder(&[0, 1]);
    s.reorder(&[0, 1, 7]);
    let stems: Vec<&str> = s.carts.iter().map(|c| c.stem.as_str()).collect();
    assert_eq!(stems, ["Game 0", "Game 1", "Game 2"]);
}

#[test]
fn a_reorder_takes_the_dust_with_the_carts() {
    let mut s = shelf_with(3);
    s.set_dust(vec![0.1, 0.2, 0.3]);
    s.reorder(&[2, 0, 1]);
    assert_eq!(s.dust(), [0.3, 0.1, 0.2]);
}

#[test]
fn jumping_goes_by_group_when_the_shelf_is_given_groups() {
    let mut s = shelf_with(6);
    // Three groups of two, whatever the carts' names begin with.
    s.set_groups(vec![1, 1, 2, 2, 3, 3]);
    s.select(0);
    s.jump_next_letter();
    assert_eq!(s.index, 2);
    s.jump_next_letter();
    assert_eq!(s.index, 4);
    s.jump_next_letter();
    assert_eq!(s.index, 0, "the jump did not wrap");
    s.jump_prev_letter();
    assert_eq!(s.index, 4, "back from the first group goes to the last");
    s.select(5);
    s.jump_prev_letter();
    assert_eq!(s.index, 4, "back from inside a group goes to its start");
}

#[test]
fn groups_of_the_wrong_length_are_dropped_and_names_are_used_again() {
    let mut s = Shelf::new(vec![
        cart_named("Alpha"),
        cart_named("Apple"),
        cart_named("Bravo"),
    ]);
    s.set_groups(vec![1, 2]);
    s.jump_next_letter();
    assert_eq!(s.index, 2, "went by letter");
}

fn cart_named(stem: &str) -> Cart {
    Cart {
        stem: stem.to_string(),
        ..cart(0)
    }
}

#[test]
fn the_store_row_draws_the_boxes_pictures_on_the_shelf() {
    let mut s = shelf_with(9);
    boxed(&mut s, 9);
    settle(&mut s);
    let out = row(&s, &[], false);
    let middle = tex_rects(&out, TexId::from_raw(100));
    assert_eq!(middle.len(), 1, "the middle box is not drawn once");
    let (x, y, w, h) = middle[0];
    assert!(
        (x + w / 2.0 - OUT_W as f32 / 2.0).abs() < 0.5,
        "not centred"
    );
    assert!(
        (y + h - STORE_STAND).abs() < 0.5,
        "not standing on the shelf"
    );
    assert!(y >= store_ceiling() - 0.5, "above the name: {y}");
    assert!(h > 250.0, "the middle box is small: {h}");
    let side = tex_rects(&out, TexId::from_raw(101));
    let (sx, _, sw, sh) = side[0];
    assert!(sw < w && sh < h, "a side box is as big as the middle one");
    assert!(
        (OUT_W as f32 - sx - STORE_SLIVER).abs() < 0.5,
        "only an edge of the side box shows: {sx}"
    );
    assert!(sx > x + w, "the side box overlaps the middle one");
    assert!(
        tex_rects(&out, TexId::from_raw(102)).is_empty(),
        "a box two along is drawn"
    );
    assert!(
        out.iter().all(|d| !matches!(d, Draw::Turned { .. })),
        "a cart was drawn"
    );
}

#[test]
fn the_middle_box_is_drawn_last_so_it_is_in_front() {
    let mut s = shelf_with(9);
    boxed(&mut s, 9);
    settle(&mut s);
    let out = row(&s, &[], false);
    let at = |t: usize| {
        out.iter()
            .position(|d| matches!(d, Draw::Tex { tex, .. } if *tex == TexId::from_raw(t)))
            .unwrap()
    };
    assert!(at(100) > at(101) && at(100) > at(108));
    let _ = (STORE_BOX_W, OUT_H);
}

#[test]
fn side_boxes_are_darkened_and_the_middle_one_is_not() {
    let mut s = shelf_with(5);
    boxed(&mut s, 5);
    settle(&mut s);
    let out = row(&s, &[], false);
    let shades = |tex: usize| {
        let rects = tex_rects(&out, TexId::from_raw(tex));
        let (x, y, w, h) = rects[0];
        out.iter()
            .filter(|d| {
                matches!(d, Draw::Rect { x: rx, y: ry, w: rw, h: rh, colour }
                    if (*rx, *ry, *rw, *rh) == (x, y, w, h) && colour[3] > 0.0 && colour[0] < 0.1)
            })
            .count()
    };
    assert_eq!(shades(100), 0, "the middle box is darkened");
    assert_eq!(shades(101), 1, "a side box is not darkened");
}

#[test]
fn a_hidden_middle_box_is_left_out_while_the_rest_stand() {
    let mut s = shelf_with(5);
    boxed(&mut s, 5);
    settle(&mut s);
    let out = row(&s, &[], true);
    assert!(tex_rects(&out, TexId::from_raw(100)).is_empty());
    assert!(!tex_rects(&out, TexId::from_raw(101)).is_empty());
}

#[test]
fn a_box_with_no_picture_yet_stands_as_a_panel_in_the_consoles_colour() {
    let s = shelf_with(3);
    let out = row(&s, &[], false);
    let panels: Vec<&Draw> = out
        .iter()
        .filter(|d| matches!(d, Draw::Rect { w, h, colour, .. } if *h > 250.0 && *w > 100.0 && colour[3] == 1.0))
        .collect();
    assert_eq!(panels.len(), 1, "the middle panel: {out:?}");
}

#[test]
fn price_tags_hang_under_each_box_on_screen() {
    let mut s = shelf_with(9);
    boxed(&mut s, 9);
    settle(&mut s);
    let tags = [
        TexId::from_raw(900),
        TexId::from_raw(901),
        TexId::from_raw(902),
    ];
    let out = row(&s, &tags, false);
    let tagged: Vec<(f32, f32, f32, f32)> = tags.iter().flat_map(|t| tex_rects(&out, *t)).collect();
    let boxes = (100..109)
        .map(|t| tex_rects(&out, TexId::from_raw(t)).len())
        .sum::<usize>();
    assert_eq!(tagged.len(), boxes, "a tag for every box drawn");
    assert!(
        tagged.iter().all(|(_, y, _, _)| *y > STORE_FOOT),
        "a tag is not under the shelf's board"
    );
}

#[test]
fn the_box_scrolls_with_the_spring_like_the_carts_do() {
    let mut s = shelf_with(9);
    boxed(&mut s, 9);
    settle(&mut s);
    s.right();
    s.update(1.0 / 60.0);
    let (x, _, w, _) = s.store_selected_rect(store_ceiling(), false).unwrap();
    assert!(x + w / 2.0 > OUT_W as f32 / 2.0, "the box is still at rest");
    settle(&mut s);
    let (x, _, w, _) = s.store_selected_rect(store_ceiling(), false).unwrap();
    assert!((x + w / 2.0 - OUT_W as f32 / 2.0).abs() < 0.5);
    assert_eq!(s.index, 1);
}

#[test]
fn loose_carts_stand_on_the_shelf_with_an_edge_of_each_neighbour() {
    let mut s = shelf_with(9);
    for i in 0..9 {
        s.set_face(i, TexId::from_raw(300 + i));
    }
    settle(&mut s);
    let mut out = Vec::new();
    s.draw_store_row(store_ceiling(), true, &|_, _| None, false, 1.0, 0.0, None, &mut out);
    let (x, y, w, h) = tex_rects(&out, TexId::from_raw(300))[0];
    assert!((x + w / 2.0 - OUT_W as f32 / 2.0).abs() < 0.5);
    assert!((y + h - STORE_STAND).abs() < 0.5);
    let (sx, _, _, _) = tex_rects(&out, TexId::from_raw(301))[0];
    assert!((OUT_W as f32 - sx - STORE_SLIVER).abs() < 0.5, "{sx}");
}

#[test]
fn stat_tags_say_new_or_how_often_and_when_a_game_was_played() {
    let new = StatTag::new(0, 0, true);
    assert_eq!(new.text, "NEW");
    assert_eq!(new.tier, StatTier::New);
    // 12 Oct 2026, midday.
    let day = 1_791_806_400;
    let side = StatTag::new(3, day, false);
    assert!(side.text.starts_with("RENTED 3"), "{}", side.text);
    assert!(!side.text.contains("LAST"));
    assert_eq!(side.tier, StatTier::Played);
    let middle = StatTag::new(12, day, true);
    assert!(middle.text.contains("LAST 12 OCT"), "{}", middle.text);
    assert_eq!(middle.tier, StatTier::Favourite);
    let face = stat_tag_face(&middle);
    let short = stat_tag_face(&side);
    assert!(face.w > short.w, "the middle tag is not wider");
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
}

#[test]
fn the_plain_box_has_the_console_colour_and_the_title_printed_on_it() {
    let face = plain_box_face(&cart_named("Zelda, The - Minish Cap (USA)"));
    assert_eq!((face.w, face.h), (PLAIN_W, STORE_BOX_H));
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
    assert!(
        face.rgba.chunks(4).all(|p| p[3] == 255),
        "a hole in the box"
    );
    // The title is lettering in white over the middle.
    let white = face
        .rgba
        .chunks(4)
        .enumerate()
        .filter(|(i, p)| {
            let y = *i as u32 / face.w;
            y > 40 && y < face.h - 30 && p[0] > 235 && p[1] > 235 && p[2] > 235
        })
        .count();
    assert!(white > 200, "no title on the box: {white} white pixels");
    let other = Cart {
        platform: Platform::Snes,
        ..cart_named("Zelda, The - Minish Cap (USA)")
    };
    assert_ne!(
        plain_box_face(&other).rgba,
        face.rgba,
        "the colour does not follow the console"
    );
    assert_ne!(
        accent_of(Some(Platform::Gba)),
        accent_of(Some(Platform::Nes))
    );
}

#[test]
fn the_nes_rail_accent_is_a_strong_red_not_its_grey_housing() {
    let [r, g, b, a] = accent_of(Some(Platform::Nes));
    assert_eq!(a, 1.0);
    assert!(r > 0.7 && g < 0.3 && b < 0.3, "{r} {g} {b}");
}

#[test]
fn a_box_picture_is_shrunk_to_the_store_size_keeping_its_shape() {
    let d = tempfile::TempDir::new().unwrap();
    let path = d.path().join("Big.png");
    let (w, h) = (600u32, 800u32);
    let px: Vec<u8> = (0..w * h).flat_map(|_| [200u8, 30, 30, 255]).collect();
    let file = std::fs::File::create(&path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
    let face = store_box_face(&path).expect("face");
    assert!(face.w <= STORE_BOX_W && face.h <= STORE_BOX_H);
    assert!(face.w == STORE_BOX_W || face.h == STORE_BOX_H);
    assert!((face.w as f32 / face.h as f32 - 0.75).abs() < 0.02);
    assert!(store_box_face(&d.path().join("missing.png")).is_none());
}

#[test]
fn the_backdrop_is_a_whole_opaque_screen_and_there_are_price_tags_to_hang() {
    let back = store_backdrop_face();
    assert_eq!((back.w, back.h), (OUT_W, OUT_H));
    assert!(back.rgba.chunks(4).all(|p| p[3] == 255));
    // Dark blue overall: blue is the strongest channel on the wall.
    let at = |x: u32, y: u32| {
        let i = ((y * back.w + x) * 4) as usize;
        [back.rgba[i], back.rgba[i + 1], back.rgba[i + 2]]
    };
    let wall = at(360, 60);
    assert!(wall[2] > wall[0] && wall[2] > wall[1], "{wall:?}");
    // The carpet has a pattern, not one flat colour.
    let carpet: std::collections::HashSet<[u8; 3]> = (0..64).map(|x| at(100 + x, 450)).collect();
    assert!(carpet.len() > 4, "a flat carpet: {}", carpet.len());
    let mut out = Vec::new();
    draw_store_backdrop(None, [1.0; 4], &mut out);
    assert!(
        matches!(out[0], Draw::Rect { .. }),
        "no plain wall before it is made"
    );
    out.clear();
    draw_store_backdrop(Some(TexId::from_raw(3)), [1.0; 4], &mut out);
    assert!(matches!(out[0], Draw::Tex { tex, .. } if tex == TexId::from_raw(3)));
}

#[test]
fn the_label_font_has_the_dash_the_sections_use() {
    let font = text::label_font().expect("font");
    assert_ne!(font.lookup_glyph_index('\u{2013}'), 0, "no en dash");
    assert_ne!(font.lookup_glyph_index('&'), 0);
}

#[test]
fn the_sign_is_lettered_in_yellow_and_wider_for_longer_words() {
    let short = sign_face("RPG");
    let long = sign_face("Sierra Entertainment");
    assert!(long.w > short.w + 60);
    let yellow = short
        .rgba
        .chunks(4)
        .filter(|p| p[3] > 200 && p[0] > 200 && p[1] > 150 && p[2] < 80)
        .count();
    assert!(yellow > 60, "no yellow lettering: {yellow}");
    // A star is drawn, not a missing-glyph box: it has points, so it is
    // narrower at its top than at its middle.
    let stars = sign_face("\u{2605}\u{2605}\u{2605}");
    assert!(stars.rgba.chunks(4).any(|p| p[3] > 200));
}

#[test]
fn the_sign_swings_down_into_place_when_it_changes() {
    let face = Some((TexId::from_raw(5), 120u32, 28u32));
    let at = |age| {
        let mut out = Vec::new();
        draw_sign(face, age, &mut out);
        let (_, y, _, _) = tex_rects(&out, TexId::from_raw(5))[0];
        (y, out)
    };
    let (start, begun) = at(0);
    let (rest, done) = at(SIGN_IN_MS);
    let (later, _) = at(SIGN_IN_MS * 10);
    assert!(
        start < rest,
        "the sign did not start higher: {start} {rest}"
    );
    assert_eq!(rest, later, "it is still moving once it has settled");
    // Chains hold it up, plate and lettering are all drawn.
    assert!(done.len() >= 6 && begun.len() == done.len());
    let mut none = Vec::new();
    draw_sign(None, 0, &mut none);
    assert!(none.is_empty(), "a sign with nothing on it was drawn");
}

#[test]
fn a_long_name_is_set_smaller_so_the_sign_stays_on_the_screen() {
    let name = "Konami Computer Entertainment Studios Japan Limited Company";
    let face = sign_face(name);
    assert!(face.w <= 530, "the sign's lettering is {} wide", face.w);
    assert!(face.rgba.chunks(4).any(|p| p[3] > 200), "no lettering");
    assert!(shelf_ui::tab_face(name).w <= 430);
}
