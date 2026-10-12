use std::path::Path;

use shelf_gfx::{Draw, TexId, OUT_H, OUT_W};

use crate::art;

const SCRIM: f32 = 0.62;

/// The wallpaper cut to the canvas, already dimmed by the scrim, so it is
/// drawn in one pass instead of a picture and a veil over it.
pub fn wallpaper_face(path: &Path) -> Option<Vec<u8>> {
    let mut rgba = art::cover(path, OUT_W, OUT_H)?;
    dim(&mut rgba);
    Some(rgba)
}

/// What black at the scrim's strength leaves of each colour.
fn dim(rgba: &mut [u8]) {
    let table: Vec<u8> = (0..=255u8)
        .map(|c| (c as f32 * (1.0 - SCRIM)).round() as u8)
        .collect();
    for px in rgba.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            *c = table[*c as usize];
        }
    }
}

pub fn draw_backdrop(face: Option<TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = face else {
        return;
    };
    out.push(Draw::Tex {
        x: 0.0,
        y: -crate::shelf::band(),
        w: OUT_W as f32,
        h: OUT_H as f32 + crate::shelf::band(),
        tex,
        alpha: 1.0,
    });
}
