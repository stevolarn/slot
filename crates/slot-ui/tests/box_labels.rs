use slot_store::{Cart, Platform};
use slot_ui::label_art;

/// A tall box cover: a red title band over the top quarter, blue below.
fn cover(dir: &std::path::Path) -> std::path::PathBuf {
    let (w, h) = (100u32, 140u32);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for _ in 0..w {
            px.extend_from_slice(if y < h / 4 {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 255, 255]
            });
        }
    }
    let path = dir.join("Ristar.png");
    let file = std::fs::File::create(&path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
    path
}

fn cart(platform: Platform, label: std::path::PathBuf) -> Cart {
    Cart {
        platform,
        stem: "Ristar".into(),
        rom: "/no/such/rom".into(),
        label: Some(label),
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

#[test]
fn a_box_cover_on_a_console_cart_keeps_its_title_at_the_top() {
    let d = tempfile::TempDir::new().unwrap();
    let path = cover(d.path());
    for platform in [Platform::Md, Platform::Snes, Platform::Nes] {
        let art = label_art(&cart(platform, path.clone())).expect("no art");
        assert_eq!(&art[..3], &[255, 0, 0], "{platform:?} lost the title band");
    }
    let gba = label_art(&cart(Platform::Gba, path)).expect("no art");
    assert_eq!(
        &gba[..3],
        &[0, 0, 255],
        "a GBA label scan is still cut from the middle"
    );
}
