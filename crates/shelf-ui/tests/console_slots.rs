use shelf_store::Platform;
use shelf_ui::{draw_empty_slot_for, housing, Draw};

fn colours(platform: Platform) -> Vec<[u32; 4]> {
    let mut out = Vec::new();
    draw_empty_slot_for(Some(platform), &mut out);
    let mut seen: Vec<[u32; 4]> = out
        .iter()
        .filter_map(|d| match d {
            Draw::Rect { colour, .. } => Some(colour.map(|c| (c * 255.0).round() as u32)),
            _ => None,
        })
        .collect();
    seen.sort();
    seen.dedup();
    seen
}

#[test]
fn each_console_has_a_slot_of_its_own() {
    for (i, a) in Platform::ALL.iter().enumerate() {
        for b in &Platform::ALL[i + 1..] {
            assert_ne!(colours(*a), colours(*b), "{a:?} and {b:?} share a slot");
        }
    }
}

#[test]
fn the_gba_is_purple() {
    let [housing, ..] = shelf_ui::slot_colours(Some(Platform::Gba));
    assert!(
        housing[2] > housing[0] && housing[0] > housing[1],
        "the GBA's plastic is {housing:?}, not indigo"
    );
}

#[test]
fn a_card_with_no_shelf_keeps_the_cards_theme() {
    let theme = housing().map(|c| (c * 255.0).round() as u32);
    let mut out = Vec::new();
    shelf_ui::draw_empty_slot(&mut out);
    assert!(out.iter().any(|d| matches!(d, Draw::Rect { colour, .. } if colour.map(|c| (c * 255.0).round() as u32) == theme)));
}
