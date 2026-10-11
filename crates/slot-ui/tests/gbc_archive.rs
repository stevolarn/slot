use slot_store::{Cart, Platform};
use slot_ui::shell_for;

#[test]
fn a_colour_game_whose_header_cannot_be_read_is_not_drawn_as_a_grey_cart() {
    let d = tempfile::TempDir::new().unwrap();
    let rom = d.path().join("Shantae.7z");
    std::fs::write(&rom, b"7z\xbc\xaf\x27\x1c not a rom").unwrap();
    let cart = |platform| Cart {
        platform,
        stem: "Shantae".into(),
        rom: rom.clone(),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    };
    assert_ne!(shell_for(&cart(Platform::Gbc)), shell_for(&cart(Platform::Gb)));
    let c = shell_for(&cart(Platform::Gbc)).colour;
    assert!(c.iter().map(|v| *v as u32).sum::<u32>() / 3 < 60, "{c:?}");
}
