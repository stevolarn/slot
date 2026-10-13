mod common;

use std::io::{self, Read};
use std::sync::Arc;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use shelf::app::{App, Phase};
use shelf_fetch::zip::Source;
use shelf_fetch::{Net, NetError};
use shelf_input::{Action, Btn};
use shelf_store::{write_shelf_state, ShelfState};
use shelf_ui::QuickRow;

/// A LaunchBox that cannot be reached.
struct Offline;

struct NoZip;

impl Source for NoZip {
    fn len(&self) -> io::Result<u64> {
        Err(io::Error::other("no route to host"))
    }

    fn range(&self, _: u64, _: u64) -> io::Result<Box<dyn Read + Send>> {
        Err(io::Error::other("no route to host"))
    }
}

impl Net for Offline {
    fn metadata(&self) -> Box<dyn Source + Send> {
        Box::new(NoZip)
    }

    fn picture(&self, _: &str) -> Result<Vec<u8>, NetError> {
        Err(NetError::Down("no route to host".into()))
    }
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn on_artwork() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_shelf_state(
        d.path(),
        &ShelfState {
            clock_set: true,
            ..ShelfState::default()
        },
    )
    .unwrap();
    let (mut a, _) = app_booting_at(d.path(), CLOCK_IS_SET);
    a.set_artwork_net(Arc::new(Offline));
    a.apply(Action::QuickMenu);
    while a.quick_menu() != Some(QuickRow::GetArtwork) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::Artwork), "{:?}", a.phase());
    (d, a)
}

fn lines(a: &App) -> Vec<String> {
    a.artwork_text().expect("the screen's words").lines
}

#[test]
fn get_artwork_says_what_it_will_fetch_and_how_much_room_it_needs() {
    let (_d, a) = on_artwork();
    let text = a.artwork_text().unwrap();
    assert!(text.lines[0].contains("for 2 games"), "{:?}", text.lines);
    assert!(
        text.lines[1].contains("6 pictures missing"),
        "{:?}",
        text.lines
    );
    assert!(text.lines.iter().any(|l| l.contains("MB free")));
    assert_eq!(
        text.lines.last().map(String::as_str),
        Some(shelf_ui::ARTWORK_CREDIT)
    );
    assert_eq!(text.hints, [("A", "Start"), ("B", "Back")]);
    // Nothing runs until A.
    assert_eq!(a.artwork_progress(), None);
}

#[test]
fn with_no_connection_it_says_so_and_a_tries_again() {
    let (_d, mut a) = on_artwork();
    press(&mut a, Btn::A);
    for _ in 0..200 {
        if lines(&a)[0] == "Could not get the artwork" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(lines(&a)[0], "Could not get the artwork");
    assert!(lines(&a)[1].contains("no route to host"), "{:?}", lines(&a));
    assert_eq!(
        a.artwork_text().unwrap().hints,
        [("A", "Try again"), ("B", "Back")]
    );
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::Artwork));
}

#[test]
fn b_goes_back_to_the_menu_row() {
    let (_d, mut a) = on_artwork();
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), Some(QuickRow::GetArtwork));
    assert!(a.artwork_text().is_none());
}

#[test]
fn the_screen_dims_everything_and_never_lets_the_brick_sleep() {
    let (_d, mut a) = on_artwork();
    a.set_screen_timeout(Some(1000));
    for _ in 0..200 {
        a.update(1.0 / 60.0);
    }
    assert!(matches!(a.phase(), Phase::Artwork));
    assert!(!a.take_sleep(), "it fell asleep");
    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(out
        .iter()
        .any(|d| matches!(d, shelf_ui::Draw::Rect { w, .. } if *w == shelf_ui::OUT_W as f32)));
}
