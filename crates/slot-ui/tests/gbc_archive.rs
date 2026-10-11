use slot_store::{Cart, Platform};
use slot_ui::{cart_shape, shell_for, CartShape, GbShell, GB_CLEAR_SHELL};

#[test]
fn every_game_on_the_colour_shelf_gets_the_clear_colour_cart() {
    let d = tempfile::TempDir::new().unwrap();
    let rom = d.path().join("Zelda DX.gbc");
    let mut bytes = vec![0u8; 0x8000];
    bytes[0x143] = 0x80;
    std::fs::write(&rom, bytes).unwrap();
    let cart = |platform| Cart {
        platform,
        stem: "Zelda DX".into(),
        rom: rom.clone(),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    };
    assert_eq!(shell_for(&cart(Platform::Gbc)), GB_CLEAR_SHELL);
    assert_eq!(
        cart_shape(&cart(Platform::Gbc)),
        CartShape::Gb(GbShell::Rounded)
    );
    assert_ne!(shell_for(&cart(Platform::Gb)), GB_CLEAR_SHELL);
}
