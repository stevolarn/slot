use std::path::Path;

use slot::label_cache::{cart_photo, photo_path};
use slot_store::{Cart, Platform};
use slot_ui::cart_box;
use tempfile::TempDir;

fn cart(platform: Platform) -> Cart {
    Cart {
        platform,
        stem: "Ristar".into(),
        rom: "/no/such/rom.zip".into(),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

/// A red cart, 60x40, in the middle of a transparent 100x100 photo.
fn photo(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut px = vec![0u8; 100 * 100 * 4];
    for y in 30..70 {
        for x in 20..80 {
            px[(y * 100 + x) * 4..][..4].copy_from_slice(&[200, 0, 0, 255]);
        }
    }
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 100, 100);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
}

#[test]
fn a_real_cart_photo_is_the_whole_face_trimmed_and_standing_on_the_bottom() {
    let root = TempDir::new().unwrap();
    let md = cart(Platform::Md);
    photo(&photo_path(root.path(), &md));
    let face = cart_photo(root.path(), &md).expect("no photo face");
    assert_eq!((face.w, face.h), cart_box(Platform::Md));
    let at = |x: u32, y: u32| &face.rgba[((y * face.w + x) * 4) as usize..][..4];
    assert_eq!(
        at(face.w / 2, face.h - 1),
        &[200, 0, 0, 255],
        "not on the bottom"
    );
    assert_eq!(
        at(1, face.h / 2)[3],
        255,
        "the transparent margin was not trimmed"
    );
    let again = cart_photo(root.path(), &md).expect("no cached face");
    assert_eq!(again.rgba, face.rgba, "the cached face differs");
}

#[test]
fn without_a_photo_there_is_no_photo_face() {
    let root = TempDir::new().unwrap();
    assert!(cart_photo(root.path(), &cart(Platform::Snes)).is_none());
}

#[test]
fn game_boy_shelves_take_real_cart_photos_too() {
    let root = TempDir::new().unwrap();
    for platform in [Platform::Gb, Platform::Gbc, Platform::Gba] {
        let c = cart(platform);
        photo(&photo_path(root.path(), &c));
        let face = cart_photo(root.path(), &c).expect("no photo face");
        assert_eq!((face.w, face.h), slot_ui::cart_box(platform));
    }
}
