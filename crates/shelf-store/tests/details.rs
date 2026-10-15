use std::path::PathBuf;

use shelf_store::{details_for, Cart, Details, Platform};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.to_string(),
        rom: PathBuf::from(format!("{stem}.rom")),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

#[test]
fn a_full_file_is_read_in_every_field() {
    let d = details_for(&fixtures(), &cart(Platform::Snes, "Super Metroid")).expect("details");
    assert_eq!(d.name.as_deref(), Some("Super Metroid"));
    assert_eq!(d.year, Some(1994));
    assert_eq!(d.genres, ["Adventure", "Platform", "Shooter"]);
    assert_eq!(d.developer.as_deref(), Some("Intelligent Systems"));
    assert_eq!(d.publisher.as_deref(), Some("Nintendo"));
    assert_eq!(d.players, Some(1));
    assert!(!d.coop);
    assert_eq!(d.rating, Some(4.69));
    assert_eq!(d.votes, Some(1018));
    assert_eq!(d.esrb, None, "Not Rated is no rating");
    assert!(d.overview.unwrap().contains("Zebes\u{2019} twisted"));
}

#[test]
fn unicode_and_ratings_come_through() {
    let d = details_for(
        &fixtures(),
        &cart(Platform::Gba, "Pokemon - Emerald Version"),
    )
    .expect("details");
    assert_eq!(d.name.as_deref(), Some("Pokémon Emerald Version"));
    assert_eq!(d.esrb.as_deref(), Some("E"));
    assert_eq!(d.players, Some(4));
    let t = details_for(&fixtures(), &cart(Platform::Gb, "Tetris")).expect("details");
    assert_eq!(t.players, Some(2));
    assert_eq!(t.genres, ["Puzzle"]);
}

#[test]
fn a_game_with_no_file_has_no_details() {
    assert_eq!(details_for(&fixtures(), &cart(Platform::Gb, "Nope")), None);
    assert_eq!(
        details_for(&fixtures(), &cart(Platform::Md, "Super Metroid")),
        None,
        "details are kept per console"
    );
}

#[test]
fn missing_unknown_and_bad_values_are_tolerated() {
    let d = Details::parse(
        "\u{feff}year=soon\nplayers=many\nrating=9.5\nvotes=-3\nfuture=1\nnonsense\ncoop=Yes\n\
         genre= , Puzzle ,,\npublisher=\nesrb=Not Rated\n",
    );
    assert_eq!(d.year, None);
    assert_eq!(d.players, None);
    assert_eq!(d.rating, Some(5.0), "a rating is held to 0..5");
    assert_eq!(d.votes, None);
    assert!(d.coop);
    assert_eq!(d.genres, ["Puzzle"]);
    assert_eq!(d.publisher, None);
    assert_eq!(d.esrb, None);
}

#[test]
fn a_file_with_nothing_usable_is_no_details() {
    assert!(Details::parse("").is_empty());
    assert!(Details::parse("junk\nfoo=bar\n").is_empty());
    assert!(Details::parse("rating=nan\n").is_empty());
}
