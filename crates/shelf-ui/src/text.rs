use std::sync::OnceLock;

use fontdue::{Font, FontSettings};

const LABEL_TTF: &[u8] = include_bytes!("../assets/label.ttf");

pub struct Layout {
    pub lines: Vec<String>,
    pub px: f32,
    pub tracking: f32,
}

pub fn label_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(LABEL_TTF, FontSettings::default()).ok())
        .as_ref()
}

pub fn fit(
    font: &Font,
    text: &str,
    max_w: f32,
    max_lines: usize,
    max_px: f32,
    min_px: f32,
) -> Layout {
    fit_box(font, text, max_w, f32::INFINITY, max_lines, max_px, min_px)
}

#[allow(clippy::too_many_arguments)]
pub fn fit_box(
    font: &Font,
    text: &str,
    max_w: f32,
    max_h: f32,
    max_lines: usize,
    max_px: f32,
    min_px: f32,
) -> Layout {
    let upper = text.to_uppercase();
    let mut px = max_px;
    while px > min_px {
        let tracking = tracking_for(px);
        let lines = wrap(font, &upper, px, tracking, max_w, false);
        if lines.len() <= max_lines
            && lines
                .iter()
                .all(|l| line_width(font, l, px, tracking) <= max_w)
            && block_height(font, px, lines.len()) <= max_h
        {
            return Layout {
                lines,
                px,
                tracking,
            };
        }
        px -= 1.0;
    }
    let tracking = tracking_for(min_px);
    Layout {
        lines: wrap(font, &upper, min_px, tracking, max_w, true)
            .into_iter()
            .take(max_lines)
            .collect(),
        px: min_px,
        tracking,
    }
}

pub fn draw_centred(dst: &mut [u8], dst_w: u32, dst_h: u32, layout: &Layout, colour: [u8; 3]) {
    for (i, a) in coverage(dst_w, dst_h, layout).into_iter().enumerate() {
        if a > 0 {
            blend(&mut dst[i * 4..i * 4 + 4], a as u32, colour);
        }
    }
}

pub fn coverage(dst_w: u32, dst_h: u32, layout: &Layout) -> Vec<u8> {
    let mut out = vec![0u8; (dst_w * dst_h) as usize];
    let Some(font) = label_font() else { return out };
    let Some(vm) = font.horizontal_line_metrics(layout.px) else {
        return out;
    };
    let line_h = vm.new_line_size;
    let block_h = line_h * layout.lines.len() as f32;
    let mut baseline = (dst_h as f32 - block_h) / 2.0 + vm.ascent;

    for line in &layout.lines {
        let mut pen = (dst_w as f32 - line_width(font, line, layout.px, layout.tracking)) / 2.0;
        for ch in line.chars() {
            let (m, cov) = font.rasterize(ch, layout.px);
            stamp(
                &mut out,
                dst_w,
                dst_h,
                (pen + m.xmin as f32).round() as i32,
                (baseline - (m.height as f32 + m.ymin as f32)).round() as i32,
                &cov,
                m.width as u32,
                m.height as u32,
            );
            pen += m.advance_width + layout.tracking;
        }
        baseline += line_h;
    }
    out
}

fn block_height(font: &Font, px: f32, lines: usize) -> f32 {
    match font.horizontal_line_metrics(px) {
        Some(vm) => vm.new_line_size * lines as f32,
        None => 0.0,
    }
}

pub fn line_width(font: &Font, line: &str, px: f32, tracking: f32) -> f32 {
    let mut w = 0.0;
    let mut n = 0;
    for ch in line.chars() {
        w += font.metrics(ch, px).advance_width;
        n += 1;
    }
    w + tracking * (n as f32 - 1.0).max(0.0)
}

fn tracking_for(px: f32) -> f32 {
    (px * 0.10).round()
}

fn wrap(
    font: &Font,
    text: &str,
    px: f32,
    tracking: f32,
    max_w: f32,
    break_words: bool,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let parts = if break_words {
            break_word(font, word, px, tracking, max_w)
        } else {
            vec![word.to_string()]
        };
        for part in parts {
            let joined = if line.is_empty() {
                part.clone()
            } else {
                format!("{line} {part}")
            };
            if line.is_empty() || line_width(font, &joined, px, tracking) <= max_w {
                line = joined;
            } else {
                lines.push(std::mem::take(&mut line));
                line = part;
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn break_word(font: &Font, word: &str, px: f32, tracking: f32, max_w: f32) -> Vec<String> {
    if line_width(font, word, px, tracking) <= max_w {
        return vec![word.to_string()];
    }
    let mut parts = Vec::new();
    let mut part = String::new();
    for ch in word.chars() {
        part.push(ch);
        if line_width(font, &part, px, tracking) > max_w && part.chars().count() > 1 {
            part.pop();
            parts.push(std::mem::take(&mut part));
            part.push(ch);
        }
    }
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

#[allow(clippy::too_many_arguments)]
fn stamp(dst: &mut [u8], dst_w: u32, dst_h: u32, x: i32, y: i32, cov: &[u8], gw: u32, gh: u32) {
    for gy in 0..gh {
        let dy = y + gy as i32;
        if dy < 0 || dy >= dst_h as i32 {
            continue;
        }
        for gx in 0..gw {
            let dx = x + gx as i32;
            if dx < 0 || dx >= dst_w as i32 {
                continue;
            }
            let i = (dy as u32 * dst_w + dx as u32) as usize;
            dst[i] = dst[i].max(cov[(gy * gw + gx) as usize]);
        }
    }
}

fn blend(px: &mut [u8], a: u32, colour: [u8; 3]) {
    let under = px[3] as u32 * (255 - a) / 255;
    let out = a + under;
    for c in 0..3 {
        px[c] = ((colour[c] as u32 * a + px[c] as u32 * under + out / 2) / out) as u8;
    }
    px[3] = out as u8;
}

/// How tall a capital letter is at `px`.
pub fn cap_height(font: &Font, px: f32) -> f32 {
    font.metrics('H', px).height as f32
}

/// How far below the top of a line its baseline is, and the line's pitch.
pub fn line_metrics(font: &Font, px: f32) -> (f32, f32) {
    match font.horizontal_line_metrics(px) {
        Some(vm) => (vm.ascent, vm.new_line_size),
        None => (px, px * 1.2),
    }
}

/// `line` as it is, in whatever case, with its left end at `x` and its
/// baseline at `baseline`, blended onto `dst` in `colour` at `alpha`/255.
#[allow(clippy::too_many_arguments)]
pub fn draw_line(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    x: f32,
    baseline: f32,
    line: &str,
    px: f32,
    tracking: f32,
    colour: [u8; 3],
    alpha: u8,
) {
    let Some(font) = label_font() else { return };
    let mut pen = x;
    for ch in line.chars() {
        let (m, cov) = font.rasterize(ch, px);
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = (baseline - (m.height as f32 + m.ymin as f32)).round() as i32;
        for row in 0..m.height as i32 {
            let dy = gy + row;
            if dy < 0 || dy >= dst_h as i32 {
                continue;
            }
            for col in 0..m.width as i32 {
                let dx = gx + col;
                if dx < 0 || dx >= dst_w as i32 {
                    continue;
                }
                let a = cov[row as usize * m.width + col as usize] as u32 * alpha as u32 / 255;
                if a > 0 {
                    let at = ((dy as u32 * dst_w + dx as u32) * 4) as usize;
                    blend(&mut dst[at..at + 4], a, colour);
                }
            }
        }
        pen += m.advance_width + tracking;
    }
}

/// Blend `colour` over one pixel of `dst`, `alpha`/255 strong.
pub fn blend_pixel(dst: &mut [u8], dst_w: u32, x: u32, y: u32, alpha: u32, colour: [u8; 3]) {
    if alpha > 0 {
        let at = ((y * dst_w + x) * 4) as usize;
        blend(&mut dst[at..at + 4], alpha, colour);
    }
}

/// `text` as a paragraph in its own case, wrapped to `max_w` and as large
/// as will fit in `max_w` by `max_h`, down to `min_px`; at that size what
/// still does not fit is cut off with an ellipsis.
pub fn fit_para(
    font: &Font,
    text: &str,
    max_w: f32,
    max_h: f32,
    max_px: f32,
    min_px: f32,
) -> Layout {
    let mut px = max_px;
    loop {
        let lines = wrap(font, text, px, 0.0, max_w, true);
        let room = (max_h / block_height(font, px, 1).max(1.0)).floor() as usize;
        if lines.len() <= room || px - 1.0 < min_px {
            let mut lines = lines;
            if lines.len() > room {
                lines.truncate(room);
                if let Some(last) = lines.last_mut() {
                    let mut cut = false;
                    while !last.is_empty()
                        && line_width(font, &format!("{last}\u{2026}"), px, 0.0) > max_w
                    {
                        last.pop();
                        cut = true;
                    }
                    // Cut at a word, not in the middle of one.
                    if let (true, Some(at)) = (cut, last.rfind(' ')) {
                        last.truncate(at);
                    }
                    *last = format!("{}\u{2026}", last.trim_end());
                }
            }
            return Layout {
                lines,
                px,
                tracking: 0.0,
            };
        }
        px -= 1.0;
    }
}
