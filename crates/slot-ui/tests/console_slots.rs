use slot_store::Platform;
use slot_ui::{draw_empty_slot_for, housing, Draw};

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
fn each_home_console_has_a_slot_of_its_own() {
    let consoles = [Platform::Nes, Platform::Md, Platform::Snes];
    for (i, a) in consoles.iter().enumerate() {
        assert_ne!(
            colours(*a),
            colours(Platform::Gba),
            "{a:?} wears the handheld's slot"
        );
        for b in &consoles[i + 1..] {
            assert_ne!(colours(*a), colours(*b), "{a:?} and {b:?} share a slot");
        }
    }
}

#[test]
fn the_handhelds_keep_the_cards_theme() {
    let theme = housing().map(|c| (c * 255.0).round() as u32);
    for p in [Platform::Gb, Platform::Gbc, Platform::Gba] {
        assert!(colours(p).contains(&theme), "{p:?} lost the theme");
    }
}
