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

#[test]
fn each_console_bar_hides_the_cart_below_its_slot_and_shows_it_above() {
    let mut seen = Vec::new();
    for p in Platform::ALL {
        let bar = slot_ui::bar_faces(p).expect("the bar renders");
        let alpha = |rgba: &[u8], x: u32, y: u32| rgba[((y * bar.back.w + x) * 4 + 3) as usize];
        let mid = bar.back.w / 2;
        assert_eq!(alpha(&bar.back.rgba, mid, 2), 255, "{p:?} back has a hole");
        assert_eq!(
            alpha(&bar.front.rgba, mid, 2),
            0,
            "{p:?} covers the cart above its slot"
        );
        assert_eq!(
            alpha(&bar.front.rgba, mid, 40),
            255,
            "{p:?} shows the cart below its slot"
        );
        assert_eq!(
            alpha(&bar.front.rgba, 10, 2),
            255,
            "{p:?} cut away its sides"
        );
        seen.push(bar.back.rgba);
    }
    for (i, a) in seen.iter().enumerate() {
        assert!(!seen[i + 1..].contains(a), "two consoles share a bar");
    }
}
