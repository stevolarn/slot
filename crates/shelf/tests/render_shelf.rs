#![cfg(target_os = "macos")]

mod common;

use std::sync::{Mutex, MutexGuard, PoisonError};

use shelf::app::App;
use shelf_gfx::{Compositor, HeadlessSurface, TexId, OUT_H, OUT_W};
use shelf_store::{write_slot_state, Platform, SlotState};
use shelf_ui::{cart_face, cart_shadow, gb_cart_shadow, GbShell};
use tempfile::TempDir;

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn tex(c: &mut Compositor, w: u32, h: u32, rgba: &[u8]) -> TexId {
    c.create_texture(w, h, rgba)
}

fn upload_faces(app: &mut App, c: &mut Compositor) {
    let row: Vec<TexId> = app
        .carts()
        .map(|cart| {
            let f = cart_face(cart);
            tex(c, f.w, f.h, &f.rgba)
        })
        .collect();
    app.set_faces(row);
    let shadow = cart_shadow();
    let shadow = tex(c, shadow.w, shadow.h, &shadow.rgba);
    app.set_cart_shadow(shadow);
    let notched = gb_cart_shadow(GbShell::Notched);
    let notched = tex(c, notched.w, notched.h, &notched.rgba);
    let rounded = gb_cart_shadow(GbShell::Rounded);
    let rounded = tex(c, rounded.w, rounded.h, &rounded.rgba);
    app.set_gb_cart_shadows(notched, rounded);
}

fn shot(app: &App, c: &mut Compositor, name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    app.draw(&mut out);
    c.begin_frame();
    c.draw_list(&out);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/shelf-{name}.png");
        let file = std::fs::File::create(&path).expect("create png");
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .expect("png header")
            .write_image_data(&px)
            .expect("png data");
        println!("wrote {path}");
    }
    px
}

#[test]
fn the_card_says_which_tetris_the_shelf_rests_on() {
    let Some((_g, _s, mut c)) = compositor() else {
        eprintln!("no GL on this host, skipping");
        return;
    };
    let d = common::tmp_root_with_carts(&["Tetris", "Emerald"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");

    let gba = resumed_shot(&d, &mut c, Some(Platform::Gba), "resume-gba");
    let gb = resumed_shot(&d, &mut c, Some(Platform::Gb), "resume-gb");
    let unstated = resumed_shot(&d, &mut c, None, "resume-unstated");

    assert!(
        gba != gb,
        "the card named the Game Boy shelf and the shelf drew the same as the Game Boy Advance's"
    );
    assert!(
        unstated == gba,
        "a card with no cart_platform line did not rest on the Game Boy Advance cartridge"
    );
}

fn resumed_shot(
    d: &TempDir,
    c: &mut Compositor,
    platform: Option<Platform>,
    name: &str,
) -> Vec<u8> {
    write_slot_state(
        d.path(),
        &SlotState {
            cart: Some("Tetris".into()),
            cart_platform: platform,
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut app = App::boot(d.path());
    upload_faces(&mut app, c);
    shot(&app, c, name)
}
