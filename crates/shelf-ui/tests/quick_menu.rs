use shelf_store::parse_stamp;
use shelf_ui::{
    date_time_text, quick_caret_face, quick_label_face, quick_value_face, ClockPicker, QuickRow,
    QuickValue, UndoFace, MENU_PAD,
};

fn inked(f: &UndoFace, x: u32, y: u32) -> bool {
    f.rgba[((y * f.w + x) * 4 + 3) as usize] > 128
}

fn ink_columns(f: &UndoFace) -> (u32, u32) {
    let cols: Vec<u32> = (0..f.w)
        .filter(|&x| (0..f.h).any(|y| inked(f, x, y)))
        .collect();
    (
        *cols.first().expect("an empty face"),
        *cols.last().expect("an empty face"),
    )
}

#[test]
fn the_type_sits_exactly_menu_pad_in_from_both_sides_of_its_face() {
    for f in [
        quick_label_face(QuickRow::DateTime),
        quick_label_face(QuickRow::Crt),
        quick_value_face("Off", false),
        quick_value_face(QuickValue::On.text(), true),
        quick_value_face("SEP 15 16:35", true),
    ] {
        let (first, last) = ink_columns(&f);
        let right = f.w - 1 - MENU_PAD;
        assert!(
            (MENU_PAD..MENU_PAD + 4).contains(&first),
            "ink starts at {first} of {}",
            f.w
        );
        assert!(
            (right - 4..=right).contains(&last),
            "ink ends at {last} of {}",
            f.w
        );
    }
}

#[test]
fn a_long_label_is_set_as_large_as_a_short_one() {
    let tall = |f: &UndoFace| {
        (0..f.h)
            .filter(|&y| (0..f.w).any(|x| inked(f, x, y)))
            .count()
    };
    let long = tall(&quick_label_face(QuickRow::DateTime));
    let short = tall(&quick_label_face(QuickRow::About));
    assert!(long + 1 >= short, "{long} rows of ink against {short}");
}

#[test]
fn the_date_and_time_read_as_a_month_a_day_and_the_carousels_24_hour_clock() {
    let at = |stamp: &str| date_time_text(parse_stamp(stamp).expect("a stamp"));
    assert_eq!(at("2026-09-15_16-35-00"), "SEP 15 16:35");
    assert_eq!(at("2027-01-05_04-07-59"), "JAN 5 04:07");
}

#[test]
fn a_picker_for_a_set_clock_starts_at_the_local_time_and_its_offset() {
    let utc = parse_stamp("2026-09-15_21-35-42").expect("a stamp");
    let p = ClockPicker::local(utc, -300);
    assert_eq!(p.offset_min(), -300);
    assert_eq!(
        p.secs(),
        utc - 42,
        "the picker does not show the minute it was opened in"
    );
    assert!(p.text().starts_with("2026-09-15 16:35"), "{}", p.text());
}

#[test]
fn a_value_is_grey_until_its_row_is_in_hand() {
    let inkiest = |lit| {
        let f = quick_value_face("On", lit);
        f.rgba
            .chunks(4)
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("an empty face")
    };
    assert_eq!(inkiest(true), [0xf6, 0xf4, 0xef]);
    assert_eq!(inkiest(false), [0x9a, 0x9a, 0xa4]);
}

#[test]
fn the_arrows_are_faces_the_height_of_a_value() {
    for right in [false, true] {
        let caret = quick_caret_face(right);
        assert!(
            caret.w > 0 && caret.rgba.chunks(4).any(|p| p[3] > 0),
            "an empty arrow"
        );
        assert_eq!(caret.h, quick_value_face("On", true).h);
    }
}

#[test]
fn the_two_carets_point_different_ways() {
    assert_ne!(
        shelf_ui::quick_caret_face(false).rgba,
        shelf_ui::quick_caret_face(true).rgba,
        "a caret glyph is missing from the symbols font"
    );
}

#[test]
fn the_rows_run_in_the_order_the_menu_lists_them() {
    assert_eq!(
        QuickRow::TOP.map(QuickRow::label),
        [
            "Options",
            "Search",
            "Hide Game",
            "Get Artwork",
            "Button Guide",
            "About",
            "Receipts",
            "Restart",
            "Power Off",
            "Exit"
        ]
    );
    assert_eq!(
        QuickRow::SETTINGS.map(QuickRow::label),
        [
            "Theme",
            "View",
            "Sort",
            "CRT Mode",
            "Dusty Carts",
            "Resume Saves",
            "Show Hidden",
            "Print Receipts",
            "Date & Time"
        ]
    );
    let opens: Vec<QuickRow> = QuickRow::ALL.into_iter().filter(|r| r.opens()).collect();
    assert_eq!(opens.len(), QuickRow::ALL.len() - 8);
    for row in [
        QuickRow::Display,
        QuickRow::View,
        QuickRow::Sort,
        QuickRow::Crt,
        QuickRow::Dusty,
        QuickRow::Resume,
        QuickRow::ShowHidden,
        QuickRow::PrintReceipts,
    ] {
        assert!(!row.opens(), "{row:?} is a setting the arrows change");
    }
    assert_eq!(
        [QuickRow::TOP.to_vec(), QuickRow::SETTINGS.to_vec()].concat(),
        QuickRow::ALL,
        "ALL is the top rows then the Options rows"
    );
    for (i, row) in QuickRow::ALL.into_iter().enumerate() {
        assert_eq!(row.index(), i, "{row:?} is out of order in ALL");
    }
}

#[test]
fn up_and_down_walk_the_rows_and_wrap() {
    for level in [&QuickRow::TOP[..], &QuickRow::SETTINGS[..]] {
        let mut row = level[0];
        for _ in 0..level.len() {
            let next = row.down();
            assert_eq!(next.up(), row);
            assert!(level.contains(&next), "{next:?} left its screen");
            row = next;
        }
        assert_eq!(row, level[0], "down did not come back round");
    }
    assert_eq!(QuickRow::Options.up(), QuickRow::Exit);
    assert_eq!(QuickRow::Display.up(), QuickRow::DateTime);
}

#[test]
fn the_values_read_as_the_menu_prints_them() {
    assert_eq!(
        QuickValue::ALL.map(QuickValue::text),
        [
            "On",
            "Off",
            "Original",
            "Video Store",
            "Carts",
            "Boxes",
            "Name",
            "Genre",
            "Year",
            "Publisher",
            "Rating",
            "Plays"
        ]
    );
    assert_eq!(QuickValue::flag(true), QuickValue::On);
    assert_eq!(QuickValue::flag(false), QuickValue::Off);
    for (i, v) in QuickValue::ALL.into_iter().enumerate() {
        assert_eq!(v.index(), i);
    }
}
