use shelf_store::Details;
use shelf_ui::{
    back_face, back_strip_face, barcode_text, details_line, details_line_face, players_long,
    players_text, rating_text, star_halves, strip_text, thousands,
    BACK_SCALE, DETAILS_H, DETAILS_W,
};

fn metroid() -> Details {
    Details::parse(include_str!(
        "../../shelf-store/tests/fixtures/Details/SFC/Super Metroid.txt"
    ))
}

fn tetris() -> Details {
    Details::parse(include_str!(
        "../../shelf-store/tests/fixtures/Details/GB/Tetris.txt"
    ))
}

#[test]
fn ratings_round_to_half_stars() {
    assert_eq!(star_halves(4.69), 9);
    assert_eq!(star_halves(4.74), 9);
    assert_eq!(star_halves(4.76), 10);
    assert_eq!(star_halves(2.2), 4);
    assert_eq!(star_halves(0.0), 0);
    assert_eq!(star_halves(5.0), 10);
    assert_eq!(star_halves(7.0), 10);
    assert_eq!(star_halves(f32::NAN), 0);
}

#[test]
fn the_line_is_year_genre_and_players() {
    assert_eq!(details_line(&metroid()), "1994 \u{b7} Adventure \u{b7} 1P");
    assert_eq!(details_line(&tetris()), "1989 \u{b7} Puzzle \u{b7} 1-2P");
    let sparse = Details {
        year: Some(2001),
        ..Details::default()
    };
    assert_eq!(details_line(&sparse), "2001");
    assert_eq!(details_line(&Details::default()), "");
}

#[test]
fn players_read_short_and_long() {
    assert_eq!(players_text(1), "1P");
    assert_eq!(players_text(4), "1-4P");
    let mut d = Details {
        players: Some(2),
        coop: true,
        ..Details::default()
    };
    assert_eq!(players_long(&d).unwrap(), "1-2 Players \u{b7} Co-op");
    d.coop = false;
    d.players = Some(1);
    assert_eq!(players_long(&d).unwrap(), "1 Player");
    d.players = None;
    assert_eq!(players_long(&d), None);
}

#[test]
fn votes_and_ratings_read_like_a_box() {
    assert_eq!(thousands(7), "7");
    assert_eq!(thousands(1018), "1,018");
    assert_eq!(thousands(1_234_567), "1,234,567");
    assert_eq!(rating_text(&metroid()).unwrap(), "4.7 (1,018)");
    let d = Details {
        rating: Some(3.0),
        ..Details::default()
    };
    assert_eq!(rating_text(&d).unwrap(), "3.0");
    assert_eq!(rating_text(&Details::default()), None);
}

#[test]
fn the_strip_names_the_publisher_or_failing_that_the_developer() {
    assert_eq!(
        strip_text(&metroid()),
        "1994 \u{b7} Nintendo \u{b7} Adventure \u{b7} 1P"
    );
    let d = Details {
        developer: Some("Game Freak".into()),
        ..Details::default()
    };
    assert_eq!(strip_text(&d), "Game Freak");
}

#[test]
fn the_bar_code_is_ten_letters_and_digits_of_the_name() {
    assert_eq!(barcode_text("Super Metroid (USA)"), "SUPERMETRO");
    assert_eq!(barcode_text("A-1"), "A1");
    assert_eq!(barcode_text("???"), "SHELF");
    assert!(shelf_ui::code39(&format!("*{}*", barcode_text("Pokémon Emerald"))).is_some());
}

fn lit(rgba: &[u8], w: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> usize {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if rgba[((y * w + x) * 4 + 3) as usize] > 0 {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn the_shelf_line_has_words_and_stars_and_nothing_without_details() {
    let face = details_line_face(&metroid()).expect("a line");
    assert_eq!((face.w, face.h), (DETAILS_W, DETAILS_H));
    assert_eq!(face.rgba.len(), (DETAILS_W * DETAILS_H * 4) as usize);
    let (w, h) = (face.w, face.h);
    assert!(lit(&face.rgba, w, 0, w / 2, 0, h) > 50, "no words");
    assert!(lit(&face.rgba, w, w / 2, w, 0, h) > 50, "no stars");
    // It is all within the face's middle: nothing at the very edges.
    assert_eq!(lit(&face.rgba, w, 0, 80, 0, h), 0);
    assert!(details_line_face(&Details::default()).is_none());
    // Unrated, it is only words.
    let mut d = metroid();
    d.rating = None;
    assert!(details_line_face(&d).is_some());
    d.year = None;
    d.genres.clear();
    d.players = None;
    assert!(details_line_face(&d).is_none());
}

fn white(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|p| p[0] == 255 && p[1] == 255 && p[2] == 255 && p[3] == 255)
        .count()
}

#[test]
fn a_box_back_with_details_has_text_and_a_bar_code_on_the_panel() {
    let panel = [0.15, 0.2, 0.55, 1.0];
    let face = back_face(&metroid(), "Super Metroid", 300, 210, panel);
    assert_eq!((face.w, face.h), (300 * BACK_SCALE, 210 * BACK_SCALE));
    let fill = [38u8, 51, 140, 255];
    assert_eq!(&face.rgba[0..4], &fill, "not the panel colour");
    let off_panel = face.rgba.chunks_exact(4).filter(|p| **p != fill).count();
    assert!(off_panel > 2_000, "only {off_panel} pixels of text");
    assert!(white(&face.rgba) > 1_500, "no bar code label");

    // No bar code label where there is no room for one: a narrow, short box
    // still gets its text.
    let tiny = back_face(&Details::default(), "X", 60, 40, panel);
    assert_eq!(
        tiny.rgba.len(),
        (60 * 40 * 4 * BACK_SCALE * BACK_SCALE) as usize
    );
}

#[test]
fn an_age_rating_makes_a_badge() {
    let panel = [0.15, 0.2, 0.55, 1.0];
    let plain = back_face(&metroid(), "Super Metroid", 300, 210, panel);
    let mut rated = metroid();
    rated.esrb = Some("T".into());
    let with = back_face(&rated, "Super Metroid", 300, 210, panel);
    let dark = |f: &shelf_ui::CartFace| {
        f.rgba
            .chunks_exact(4)
            .filter(|p| p[0] < 45 && p[1] < 45 && p[2] < 45)
            .count()
    };
    assert!(dark(&with) > dark(&plain) + 1_000, "no badge was drawn");
}

#[test]
fn a_long_overview_is_cut_to_fit_and_a_light_panel_gets_dark_ink() {
    let mut d = metroid();
    d.overview = Some("word ".repeat(2_000));
    let face = back_face(&d, "Super Metroid", 300, 210, [0.9, 0.9, 0.85, 1.0]);
    let dark = face
        .rgba
        .chunks_exact(4)
        .filter(|p| p[0] < 80 && p[1] < 80)
        .count();
    assert!(dark > 1_000, "no dark ink on a light panel");
}

#[test]
fn the_strip_over_a_scan_is_a_translucent_band_a_sixth_as_tall() {
    let strip = back_strip_face(&metroid(), 240, 300);
    assert_eq!((strip.w, strip.h), (240 * BACK_SCALE, 50 * BACK_SCALE));
    assert_eq!(strip.rgba[3], 165, "the band is not translucent");
    assert!(
        strip.rgba.chunks_exact(4).any(|p| p[3] == 255),
        "no words on the band"
    );
}

