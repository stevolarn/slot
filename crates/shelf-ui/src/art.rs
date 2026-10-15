use std::path::Path;

pub fn cover(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    cover_from(path, w, h, false)
}

/// Like `cover`, but keeps the top of the picture, where a box cover has
/// its title, and crops from the bottom.
pub fn cover_top(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    cover_from(path, w, h, true)
}

fn cover_from(path: &Path, w: u32, h: u32, top: bool) -> Option<Vec<u8>> {
    let (src, sw, sh) = decode(path)?;
    if sw == 0 || sh == 0 {
        return None;
    }

    let scale = (w as f32 / sw as f32).max(h as f32 / sh as f32);
    let ox = (sw as f32 - w as f32 / scale) / 2.0;
    let oy = if top {
        0.0
    } else {
        (sh as f32 - h as f32 / scale) / 2.0
    };

    let cols = spans(w, sw, |x| ox + x as f32 / scale);
    let rows = spans(h, sh, |y| oy + y as f32 / scale);
    Some(resample(&src, sw, &cols, &rows))
}

/// The whole picture scaled to fit inside `max_w`x`max_h`, keeping its
/// shape: its pixels, width and height.
pub fn fit(path: &Path, max_w: u32, max_h: u32) -> Option<(Vec<u8>, u32, u32)> {
    let (src, sw, sh) = decode(path)?;
    fit_pixels(&src, sw, sh, max_w, max_h, true)
}

/// RGBA pixels `sw` by `sh` scaled to fit inside `max_w`x`max_h`, keeping
/// their shape; never made bigger unless `grow`.
pub fn fit_pixels(
    src: &[u8],
    sw: u32,
    sh: u32,
    max_w: u32,
    max_h: u32,
    grow: bool,
) -> Option<(Vec<u8>, u32, u32)> {
    if sw == 0 || sh == 0 {
        return None;
    }
    let mut scale = (max_w as f32 / sw as f32).min(max_h as f32 / sh as f32);
    if !grow {
        scale = scale.min(1.0);
    }
    let (w, h) = (
        ((sw as f32 * scale).round() as u32).max(1),
        ((sh as f32 * scale).round() as u32).max(1),
    );
    let (sx, sy) = (sw as f32 / w as f32, sh as f32 / h as f32);
    let cols = spans(w, sw, |x| x as f32 * sx);
    let rows = spans(h, sh, |y| y as f32 * sy);
    Some((resample(src, sw, &cols, &rows), w, h))
}

/// A cut-out photo (transparent around the subject) trimmed to the subject
/// and scaled to fit inside `w`x`h`, centred across and standing on the
/// bottom edge.
pub fn fit_bottom(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    let (src, sw, sh) = decode(path)?;
    let solid = |x: u32, y: u32| src[((y * sw + x) * 4 + 3) as usize] > 24;
    let rows: Vec<u32> = (0..sh).filter(|&y| (0..sw).any(|x| solid(x, y))).collect();
    let cols: Vec<u32> = (0..sw).filter(|&x| (0..sh).any(|y| solid(x, y))).collect();
    let (top, bottom) = (*rows.first()?, *rows.last()? + 1);
    let (left, right) = (*cols.first()?, *cols.last()? + 1);
    let (tw, th) = ((right - left) as f32, (bottom - top) as f32);

    let scale = (w as f32 / tw).min(h as f32 / th);
    let (dw, dh) = (tw * scale, th * scale);
    let (dx, dy) = ((w as f32 - dw) / 2.0, h as f32 - dh);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 - dx, y as f32 - dy);
            if fx + 1.0 <= 0.0 || fy + 1.0 <= 0.0 || fx >= dw || fy >= dh {
                continue;
            }
            let px = weighted_average(
                &src,
                sw,
                sh,
                left as f32 + fx.max(0.0) / scale,
                top as f32 + fy.max(0.0) / scale,
                left as f32 + (fx + 1.0).min(dw) / scale,
                top as f32 + (fy + 1.0).min(dh) / scale,
            );
            out[((y * w + x) * 4) as usize..][..4].copy_from_slice(&px);
        }
    }
    Some(out)
}

/// A box average that weights colour by alpha, so a cut-out's transparent
/// surround does not bleed into its edges.
fn weighted_average(src: &[u8], sw: u32, sh: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> [u8; 4] {
    let xa = (x0.floor().max(0.0) as u32).min(sw - 1);
    let ya = (y0.floor().max(0.0) as u32).min(sh - 1);
    let xb = ((x1.ceil() as u32).max(xa + 1)).min(sw);
    let yb = ((y1.ceil() as u32).max(ya + 1)).min(sh);
    let (mut rgb, mut alpha, mut n) = ([0u64; 3], 0u64, 0u64);
    for y in ya..yb {
        for x in xa..xb {
            let i = ((y * sw + x) * 4) as usize;
            let a = src[i + 3] as u64;
            for c in 0..3 {
                rgb[c] += src[i + c] as u64 * a;
            }
            alpha += a;
            n += 1;
        }
    }
    if alpha == 0 {
        return [0; 4];
    }
    [
        (rgb[0] / alpha) as u8,
        (rgb[1] / alpha) as u8,
        (rgb[2] / alpha) as u8,
        ((alpha + n / 2) / n) as u8,
    ]
}

/// The source pixels, `[start, end)`, each of `n` output pixels along a
/// side of `len` averages: output pixel `i` covers `edge(i)..edge(i + 1)`
/// in the source, widened to whole pixels.
fn spans(n: u32, len: u32, edge: impl Fn(u32) -> f32) -> Vec<(u32, u32)> {
    (0..n)
        .map(|i| {
            let a = (edge(i).floor().max(0.0) as u32).min(len - 1);
            let b = ((edge(i + 1).ceil() as u32).max(a + 1)).min(len);
            (a, b)
        })
        .collect()
}

/// Each output pixel the plain average of the source rectangle its column
/// and row spans cover. The rows of a span are summed once per output row
/// and the columns then read from that sum, rather than every rectangle
/// being walked pixel by pixel.
fn resample(src: &[u8], sw: u32, cols: &[(u32, u32)], rows: &[(u32, u32)]) -> Vec<u8> {
    let stride = sw as usize * 4;
    let mut out = Vec::with_capacity(cols.len() * rows.len() * 4);
    let mut sum = vec![0u32; stride];
    for &(ya, yb) in rows {
        sum.fill(0);
        for y in ya..yb {
            let line = &src[y as usize * stride..][..stride];
            for (s, v) in sum.iter_mut().zip(line) {
                *s += *v as u32;
            }
        }
        let tall = yb - ya;
        for &(xa, xb) in cols {
            let n = (xb - xa) * tall;
            let mut acc = [0u32; 4];
            for px in sum[xa as usize * 4..xb as usize * 4].chunks_exact(4) {
                for c in 0..4 {
                    acc[c] += px[c];
                }
            }
            for a in acc {
                out.push(((a + n / 2) / n) as u8);
            }
        }
    }
    out
}

fn decode(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    decode_bytes(&std::fs::read(path).ok()?)
}

/// A PNG or JPEG picture as RGBA pixels, its width and its height.
pub fn decode_bytes(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    if bytes.starts_with(&[0xff, 0xd8]) {
        return decode_jpeg(bytes);
    }
    let orientation = exif_chunk(bytes).and_then(exif_orientation).unwrap_or(1);
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    dec.set_limits(png::Limits { bytes: 64 << 20 });

    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let n = (info.width * info.height) as usize;
    if info.color_type == png::ColorType::Rgba {
        buf.truncate(n * 4);
        return Some(orient(buf, info.width, info.height, orientation));
    }
    let src = &buf[..info.buffer_size()];
    let mut rgba = vec![255u8; n * 4];
    match info.color_type {
        png::ColorType::Rgba => {}
        png::ColorType::Rgb => {
            for (to, from) in rgba.chunks_exact_mut(4).zip(src.chunks_exact(3)) {
                to[..3].copy_from_slice(from);
            }
        }
        png::ColorType::Grayscale => {
            for i in 0..n {
                rgba[i * 4..i * 4 + 3].fill(src[i]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..n {
                rgba[i * 4..i * 4 + 3].fill(src[i * 2]);
                rgba[i * 4 + 3] = src[i * 2 + 1];
            }
        }
        png::ColorType::Indexed => return None,
    }
    Some(orient(rgba, info.width, info.height, orientation))
}

fn decode_jpeg(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    use zune_jpeg::zune_core::{colorspace::ColorSpace, options::DecoderOptions};
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut dec = zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(bytes), options);
    let rgba = dec.decode().ok()?;
    let (w, h) = dec.dimensions()?;
    let (w, h) = (w as u32, h as u32);
    (rgba.len() == (w * h * 4) as usize).then_some((rgba, w, h))
}

fn exif_chunk(png: &[u8]) -> Option<&[u8]> {
    let mut at = 8;
    loop {
        let len = u32::from_be_bytes(png.get(at..at + 4)?.try_into().ok()?) as usize;
        let kind = png.get(at + 4..at + 8)?;
        match kind {
            b"eXIf" => return png.get(at + 8..at + 8 + len),
            b"IDAT" | b"IEND" => return None,
            _ => at += 12 + len,
        }
    }
}

fn exif_orientation(exif: &[u8]) -> Option<u16> {
    let big = match exif.get(..2)? {
        b"MM" => true,
        b"II" => false,
        _ => return None,
    };
    let u16_at = |i: usize| {
        let b: [u8; 2] = exif.get(i..i + 2)?.try_into().ok()?;
        Some(if big {
            u16::from_be_bytes(b)
        } else {
            u16::from_le_bytes(b)
        })
    };
    let u32_at = |i: usize| {
        let b: [u8; 4] = exif.get(i..i + 4)?.try_into().ok()?;
        Some(if big {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        })
    };
    let ifd = u32_at(4)? as usize;
    let count = u16_at(ifd)? as usize;
    (0..count)
        .map(|n| ifd + 2 + n * 12)
        .find(|&e| u16_at(e) == Some(0x0112))
        .and_then(|e| u16_at(e + 8))
}

fn orient(src: Vec<u8>, w: u32, h: u32, orientation: u16) -> (Vec<u8>, u32, u32) {
    if !(2..=8).contains(&orientation) {
        return (src, w, h);
    }
    let (dw, dh) = if orientation >= 5 { (h, w) } else { (w, h) };
    let mut out = vec![0u8; src.len()];
    for y in 0..dh {
        for x in 0..dw {
            let (sx, sy) = match orientation {
                2 => (w - 1 - x, y),
                3 => (w - 1 - x, h - 1 - y),
                4 => (x, h - 1 - y),
                5 => (y, x),
                6 => (y, h - 1 - x),
                7 => (w - 1 - y, h - 1 - x),
                _ => (w - 1 - y, x),
            };
            let s = ((sy * w + sx) * 4) as usize;
            let d = ((y * dw + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    (out, dw, dh)
}

pub(crate) fn render_svg(svg: &str, w: u32, h: u32) -> Option<Vec<u8>> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let size = tree.size();
    let scale =
        resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(&tree, scale, &mut pixmap.as_mut());
    let mut rgba = pixmap.data().to_vec();
    unpremultiply(&mut rgba);
    Some(rgba)
}

fn unpremultiply(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3];
        if a == 0 || a == 255 {
            continue;
        }
        for c in &mut px[..3] {
            *c = ((*c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
        }
    }
}
