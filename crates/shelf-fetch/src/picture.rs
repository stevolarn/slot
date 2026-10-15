//! Saving a downloaded picture at the size Shelf draws it: box scans as
//! JPEG, which is a seventh the size of PNG on the card, and cart photos,
//! which are cut out, as PNG trimmed to the cart.

use std::io;
use std::path::Path;

/// Box scans are drawn at most this big, on the box shelf.
pub const BOX_MAX: u32 = 600;
/// Cart photos are drawn at most this big.
pub const CART_MAX: u32 = 360;
const JPEG_QUALITY: u8 = 80;

fn bad(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

/// Write `bytes` to `dest` through a temporary file beside it, so a stop
/// half way leaves no broken picture.
fn write_new(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dest)
}

/// A box front or back from `bytes` (PNG or JPEG), as a JPEG at `dest`.
pub fn save_box(bytes: &[u8], dest: &Path) -> io::Result<()> {
    let (rgba, w, h) = shelf_ui::decode_bytes(bytes).ok_or_else(|| bad("not a picture"))?;
    let (rgba, w, h) =
        shelf_ui::fit_pixels(&rgba, w, h, BOX_MAX, BOX_MAX, false).ok_or_else(|| bad("empty picture"))?;
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, JPEG_QUALITY)
        .encode(&rgba, w as u16, h as u16, jpeg_encoder::ColorType::Rgba)
        .map_err(|e| bad(&e.to_string()))?;
    write_new(dest, &out)
}

/// The smallest rectangle holding every pixel that is not see-through.
fn opaque_bounds(rgba: &[u8], w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if rgba[((y * w + x) * 4 + 3) as usize] > 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// A cart photo from `bytes`, trimmed to the cart and fitted to the size
/// Shelf draws carts, as a PNG at `dest`.
pub fn save_cart(bytes: &[u8], dest: &Path) -> io::Result<()> {
    let (rgba, w, h) = shelf_ui::decode_bytes(bytes).ok_or_else(|| bad("not a picture"))?;
    let (x0, y0, x1, y1) = opaque_bounds(&rgba, w, h).unwrap_or((0, 0, w, h));
    let (cw, ch) = (x1 - x0, y1 - y0);
    let mut cut = Vec::with_capacity((cw * ch * 4) as usize);
    for y in y0..y1 {
        let from = ((y * w + x0) * 4) as usize;
        cut.extend_from_slice(&rgba[from..from + (cw * 4) as usize]);
    }
    let (rgba, w, h) =
        shelf_ui::fit_pixels(&cut, cw, ch, CART_MAX, CART_MAX, false).ok_or_else(|| bad("empty picture"))?;
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Best);
        let mut writer = enc.write_header().map_err(|e| bad(&e.to_string()))?;
        writer.write_image_data(&rgba).map_err(|e| bad(&e.to_string()))?;
    }
    write_new(dest, &out)
}
