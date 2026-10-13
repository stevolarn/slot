use shelf_store::{Cart, Platform};
use shelf_ui::{
    cart_box, cart_face_with, cart_shape, console_cart_shadow, console_silhouette, foot_y, rest_y,
    seated_box, shell_for, CartShape, Console, MD_CART_H, MD_CART_W, MD_LABEL, MD_SHELL, MOUTH_H,
    NES_CART_H, NES_CART_W, NES_LABEL, NES_SHELL, OUT_H, PLATE_H, SNES_CART_H, SNES_CART_W,
    SNES_LABEL, SNES_SHELL, GG_CART_H, GG_CART_W, GG_LABEL, SMS_CART_H, SMS_CART_W, SMS_LABEL,
};

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.into(),
        rom: "/no/such/rom".into(),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

const BOTH: [(Platform, Console, &str); 5] = [
    (Platform::Nes, Console::Nes, "the NES cart"),
    (Platform::Md, Console::Md, "the Mega Drive cart"),
    (Platform::Snes, Console::Snes, "the SNES cart"),
    (Platform::Gg, Console::Gg, "the Game Gear cart"),
    (Platform::Sms, Console::Sms, "the Master System cart"),
];

#[test]
fn mega_drive_and_snes_games_get_carts_of_their_own_shape() {
    for (platform, console, name) in BOTH {
        assert_eq!(
            cart_shape(&cart(platform, "Game")),
            CartShape::Console(console),
            "{name}"
        );
    }
    assert_eq!(cart_box(Platform::Md), (MD_CART_W, MD_CART_H));
    assert_eq!(cart_box(Platform::Snes), (SNES_CART_W, SNES_CART_H));
    const {
        assert!(
            MD_CART_W > MD_CART_H,
            "a Mega Drive cart is wider than tall"
        )
    };
    const { assert!(SNES_CART_W > SNES_CART_H, "a SNES cart is wider than tall") };
}

#[test]
fn each_seats_at_the_slot_width_with_its_own_aspect() {
    for (platform, _, name) in BOTH {
        let (cw, ch) = cart_box(platform);
        let (sw, sh) = seated_box(platform);
        assert_eq!(sw, 240, "{name} seats wider than the slot");
        let (cart, seated) = (ch as f32 / cw as f32, sh as f32 / sw as f32);
        assert!((cart - seated).abs() < 0.01, "{name} changes shape seating");
    }
}

#[test]
fn each_stands_on_the_row_clear_of_the_plate_and_the_slot() {
    let lip = OUT_H as f32 - MOUTH_H;
    for (platform, _, name) in BOTH {
        let h = cart_box(platform).1 as f32;
        let (top, foot) = (rest_y(h), foot_y(h));
        assert!(
            top > PLATE_H,
            "{name}'s top at {top} is under the HUD plate"
        );
        assert!(
            foot < lip,
            "{name}'s foot at {foot} is standing in the slot"
        );
    }
}

#[test]
fn the_label_sits_inside_the_outline() {
    for (label, console, name) in [
        (NES_LABEL, Console::Nes, "NES"),
        (MD_LABEL, Console::Md, "Mega Drive"),
        (SNES_LABEL, Console::Snes, "SNES"),
        (GG_LABEL, Console::Gg, "Game Gear"),
        (SMS_LABEL, Console::Sms, "Master System"),
    ] {
        let (w, h) = match console {
            Console::Nes => (NES_CART_W, NES_CART_H),
            Console::Md => (MD_CART_W, MD_CART_H),
            Console::Snes => (SNES_CART_W, SNES_CART_H),
            Console::Gg => (GG_CART_W, GG_CART_H),
            Console::Sms => (SMS_CART_W, SMS_CART_H),
        };
        let mask = console_silhouette(console, w, h);
        let (x, y, lw, lh) = label;
        for (cx, cy) in [
            (x, y),
            (x + lw - 1, y),
            (x, y + lh - 1),
            (x + lw - 1, y + lh - 1),
        ] {
            assert!(
                mask[(cy * w + cx) as usize] > 250,
                "the {name} label corner {cx},{cy} hangs off the cart"
            );
        }
    }
}

#[test]
fn a_face_is_drawn_at_the_cart_size_and_clipped_to_its_outline() {
    for (platform, console, name) in BOTH {
        let face = cart_face_with(&cart(platform, "Gunstar Heroes (USA)"), None);
        let (w, h) = cart_box(platform);
        assert_eq!((face.w, face.h), (w, h), "{name}");
        let mask = console_silhouette(console, w, h);
        for (px, cover) in face.rgba.chunks_exact(4).zip(&mask) {
            assert!(px[3] <= *cover, "{name} paints outside its outline");
        }
        let (x, y, lw, lh) = match console {
            Console::Nes => NES_LABEL,
            Console::Md => MD_LABEL,
            Console::Snes => SNES_LABEL,
            Console::Gg => GG_LABEL,
            Console::Sms => SMS_LABEL,
        };
        let mid = (((y + lh / 2) * w + x + lw / 2) * 4) as usize;
        let shell = shell_for(&cart(platform, "x")).colour;
        assert_ne!(&face.rgba[mid..mid + 3], &shell[..], "{name} has no label");
    }
}

#[test]
fn mega_drive_carts_are_black_and_snes_carts_grey() {
    assert_eq!(shell_for(&cart(Platform::Nes, "Metroid")), NES_SHELL);
    assert_eq!(shell_for(&cart(Platform::Md, "Ristar")), MD_SHELL);
    assert_eq!(shell_for(&cart(Platform::Snes, "F-Zero")), SNES_SHELL);
    let luma = |c: [u8; 3]| c.iter().map(|v| *v as u32).sum::<u32>() / 3;
    assert!(luma(MD_SHELL.colour) < 60);
    assert!(luma(SNES_SHELL.colour) > 120);
}

#[test]
fn each_gets_a_black_shadow_of_its_own_outline() {
    for (_, console, name) in BOTH {
        let s = console_cart_shadow(console);
        let mask = console_silhouette(console, s.w, s.h);
        for (px, cover) in s.rgba.chunks_exact(4).zip(&mask) {
            assert_eq!(&px[..3], &[0, 0, 0], "{name}'s shadow is not black");
            assert_eq!(px[3], *cover, "{name}'s shadow is not its shape");
        }
    }
}
