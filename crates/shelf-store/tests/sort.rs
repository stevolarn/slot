mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::tmp_root;
use shelf_store::{
    read_shelf_state, sections, sorted_order, write_shelf_state, Cart, Details, DisplayMode,
    Platform, ShelfState, SortKey,
};

fn cart(stem: &str) -> Cart {
    Cart {
        platform: Platform::Gba,
        stem: stem.to_string(),
        rom: PathBuf::from(format!("{stem}.gba")),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

fn details(year: Option<u16>, genre: &str, publisher: &str, rating: Option<f32>) -> Details {
    Details {
        year,
        genres: if genre.is_empty() {
            Vec::new()
        } else {
            vec![genre.to_string(), "Other".to_string()]
        },
        publisher: (!publisher.is_empty()).then(|| publisher.to_string()),
        rating,
        ..Details::default()
    }
}

fn shelf() -> (Vec<Cart>, HashMap<String, Details>) {
    let carts: Vec<Cart> = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"]
        .into_iter()
        .map(cart)
        .collect();
    let mut d = HashMap::new();
    d.insert(
        "Alpha".into(),
        details(Some(1999), "Racing", "SEGA", Some(3.2)),
    );
    d.insert(
        "Bravo".into(),
        details(Some(1994), "Action", "NINTENDO", Some(4.7)),
    );
    d.insert(
        "Charlie".into(),
        details(Some(1994), "action", "CAPCOM", Some(4.1)),
    );
    // Delta has no details at all.
    d.insert("Echo".into(), details(None, "RPG", "", None));
    d.insert(
        "Foxtrot".into(),
        details(Some(2003), "", "Atari", Some(4.7)),
    );
    (carts, d)
}

fn order(key: SortKey) -> Vec<String> {
    let (carts, d) = shelf();
    sorted_order(&carts, |c| d.get(&c.stem), |_| None, key)
        .into_iter()
        .map(|i| carts[i].stem.clone())
        .collect()
}

#[test]
fn name_is_the_shelf_order_whatever_the_details() {
    assert_eq!(
        order(SortKey::Name),
        ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"]
    );
}

#[test]
fn genre_goes_by_the_first_genre_ignoring_case_with_name_breaking_ties() {
    assert_eq!(
        order(SortKey::Genre),
        ["Bravo", "Charlie", "Alpha", "Echo", "Delta", "Foxtrot"]
    );
}

#[test]
fn year_goes_oldest_first_and_games_without_one_go_last_in_name_order() {
    assert_eq!(
        order(SortKey::Year),
        ["Bravo", "Charlie", "Alpha", "Foxtrot", "Delta", "Echo"]
    );
}

#[test]
fn publisher_goes_alphabetically_without_one_last() {
    assert_eq!(
        order(SortKey::Publisher),
        ["Foxtrot", "Charlie", "Bravo", "Alpha", "Delta", "Echo"]
    );
}

#[test]
fn rating_goes_best_first_with_equal_ratings_in_name_order() {
    assert_eq!(
        order(SortKey::Rating),
        ["Bravo", "Foxtrot", "Charlie", "Alpha", "Delta", "Echo"]
    );
}

#[test]
fn sorting_is_stable_and_repeatable() {
    for key in SortKey::ALL {
        assert_eq!(order(key), order(key));
    }
    // With no details at all every order is the name order.
    let carts: Vec<Cart> = ["Zed", "Ant", "Mole"].into_iter().map(cart).collect();
    for key in SortKey::ALL {
        let order = sorted_order(&carts, |_| None, |_| None, key);
        let stems: Vec<&str> = order.iter().map(|i| carts[*i].stem.as_str()).collect();
        assert_eq!(stems, ["Ant", "Mole", "Zed"], "{key:?}");
    }
}

fn labels(key: SortKey) -> Vec<String> {
    let (carts, d) = shelf();
    sections(&carts, |c| d.get(&c.stem), |_| None, key)
}

#[test]
fn sections_follow_the_order_they_are_for() {
    assert_eq!(
        labels(SortKey::Genre),
        ["RACING", "ACTION", "ACTION", "OTHER", "RPG", "OTHER"]
    );
    assert_eq!(
        labels(SortKey::Year),
        [
            "1999",
            "1994",
            "1994",
            "YEAR UNKNOWN",
            "YEAR UNKNOWN",
            "2003"
        ]
    );
    assert_eq!(
        labels(SortKey::Publisher),
        [
            "SEGA",
            "NINTENDO",
            "CAPCOM",
            "PUBLISHER UNKNOWN",
            "PUBLISHER UNKNOWN",
            "ATARI"
        ]
    );
}

#[test]
fn ratings_fall_into_staff_picks_and_star_bands() {
    assert_eq!(
        labels(SortKey::Rating),
        [
            "\u{2605}\u{2605}\u{2605}",
            "STAFF PICKS \u{2605}",
            "\u{2605}\u{2605}\u{2605}\u{2605}",
            "UNRATED",
            "UNRATED",
            "STAFF PICKS \u{2605}"
        ]
    );
}

#[test]
fn every_shelf_is_sectioned_by_letter() {
    assert_eq!(labels(SortKey::Name), ["A", "B", "C", "D", "E", "F"]);
    let big: Vec<Cart> = (0..40)
        .map(|i| cart(&format!("{}{i}", (b'A' + (i % 26) as u8) as char)))
        .collect();
    let got = sections(&big, |_| None, |_| None, SortKey::Name);
    assert_eq!(got[0], "A");
    assert_eq!(got[3], "D");
    assert_eq!(got[25], "Z");
    let digits = sections(&[cart("007 Nightfire")], |_| None, |_| None, SortKey::Name);
    assert_eq!(digits, ["0\u{2013}9"]);
}

#[test]
fn r2_cycles_every_order_and_comes_back_to_name() {
    let mut key = SortKey::Name;
    let mut seen = Vec::new();
    for _ in 0..6 {
        key = key.next();
        seen.push(key.word());
    }
    assert_eq!(seen, ["genre", "year", "publisher", "rating", "plays", "name"]);
    assert_eq!(SortKey::Genre.prev(), SortKey::Name);
    assert_eq!(SortKey::Name.prev(), SortKey::Plays);
}

#[test]
fn display_and_sort_are_saved_and_default_to_carts_by_name() {
    let fresh = ShelfState::default();
    assert_eq!(
        (fresh.display, fresh.sort),
        (DisplayMode::Carts, SortKey::Name)
    );
    let d = tmp_root();
    let s = ShelfState {
        clock_set: true,
        display: DisplayMode::Store,
        sort: SortKey::Publisher,
        ..ShelfState::default()
    };
    write_shelf_state(d.path(), &s).unwrap();
    assert_eq!(read_shelf_state(d.path()), s);
    let text = std::fs::read_to_string(d.path().join("Config/shelf.state")).unwrap();
    for line in ["display=store", "sort=publisher"] {
        assert!(text.lines().any(|l| l == line), "no {line} in {text:?}");
    }
}

#[test]
fn a_state_file_from_before_the_store_loads_as_carts_by_name() {
    let d = tmp_root();
    std::fs::write(
        d.path().join("Config/shelf.state"),
        "cart=Emerald\nbrightness=3\nvolume=40\nmuted=0\nclock_set=1\nutc_offset_min=0\ncrt=1\n\
         display=nonsense\nsort=\n",
    )
    .unwrap();
    let s = read_shelf_state(d.path());
    assert_eq!((s.display, s.sort), (DisplayMode::Carts, SortKey::Name));
}

#[test]
fn display_steps_store_carts_boxes_and_round() {
    use shelf_store::DisplayMode::{Boxes, Carts, Store};
    assert_eq!([Store.next(), Carts.next(), Boxes.next()], [Carts, Boxes, Store]);
    assert_eq!([Store.prev(), Carts.prev(), Boxes.prev()], [Boxes, Store, Carts]);
    for mode in [Store, Carts, Boxes] {
        assert_eq!(shelf_store::DisplayMode::parse(mode.word()), Some(mode));
    }
    assert_eq!(Boxes.word(), "boxes");
}

#[test]
fn by_plays_the_most_rented_come_first_in_their_own_sections() {
    use shelf_store::Rental;
    let carts: Vec<Cart> = ["Ant", "Bee", "Cat", "Dog"].into_iter().map(cart).collect();
    let plays = |c: &Cart| match c.stem.as_str() {
        "Bee" => Some(Rental { times: 12, last: 50 }),
        "Cat" => Some(Rental { times: 4, last: 10 }),
        "Dog" => Some(Rental { times: 4, last: 90 }),
        _ => None,
    };
    let order = sorted_order(&carts, |_| None, plays, SortKey::Plays);
    let stems: Vec<&str> = order.iter().map(|i| carts[*i].stem.as_str()).collect();
    assert_eq!(stems, ["Bee", "Dog", "Cat", "Ant"]);
    let sorted: Vec<Cart> = order.iter().map(|i| carts[*i].clone()).collect();
    let labels = sections(&sorted, |_| None, plays, SortKey::Plays);
    assert_eq!(
        labels,
        ["FAVOURITES \u{2605}", "REGULARS", "REGULARS", "NEW RELEASES"]
    );
}
