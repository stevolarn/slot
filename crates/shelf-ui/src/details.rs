//! A game's details as they are printed: a line under its name on the
//! shelf, and on the back of its box.

use shelf_store::Details;

use crate::barcode::{code39, CODE39_NARROW, CODE39_WIDE};
use crate::cart::CartFace;
use crate::plate::{game_name, UndoFace, NAME_W};
use crate::text;

pub const DETAILS_W: u32 = NAME_W;
pub const DETAILS_H: u32 = 20;
/// How far below the top of the name's face the details line's face
/// starts: the line sits snug under the name's letters.
pub const DETAILS_DROP: f32 = 32.0;
const LINE_PX: f32 = 16.0;
const LINE_MIN_PX: f32 = 10.0;
const LINE_INK: [u8; 3] = [0xb4, 0xb2, 0xac];
const LINE_ALPHA: u8 = 235;

const INK: [u8; 3] = [0xf6, 0xf4, 0xef];
const DARK_INK: [u8; 3] = [0x1a, 0x19, 0x17];

/// A box back is made at twice the size it is first drawn at, so that it
/// is sharp when the panel scales it up.
pub const BACK_SCALE: u32 = 2;
/// What fraction of a back scan's height the details strip takes.
pub const STRIP_FRAC: f32 = 1.0 / 6.0;

/// The rating as half stars out of ten: 4.69 is nine.
pub fn star_halves(rating: f32) -> u8 {
    (rating.clamp(0.0, 5.0) * 2.0).round() as u8
}

/// "1P" for one player, "1-4P" for up to four.
pub fn players_text(players: u8) -> String {
    match players {
        0 | 1 => "1P".to_string(),
        n => format!("1-{n}P"),
    }
}

/// "1 Player", "1-2 Players", with ` · Co-op` after for a game that has it.
pub fn players_long(d: &Details) -> Option<String> {
    let base = d.players.map(|n| match n {
        0 | 1 => "1 Player".to_string(),
        n => format!("1-{n} Players"),
    });
    match (base, d.coop) {
        (Some(b), true) => Some(format!("{b} \u{b7} Co-op")),
        (Some(b), false) => Some(b),
        (None, true) => Some("Co-op".to_string()),
        (None, false) => None,
    }
}

/// 1018 as "1,018".
pub fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "4.7 (1,018)": the rating, and how many votes it is of.
pub fn rating_text(d: &Details) -> Option<String> {
    let r = d.rating?;
    Some(match d.votes {
        Some(v) => format!("{r:.1} ({})", thousands(v)),
        None => format!("{r:.1}"),
    })
}

fn joined(parts: Vec<String>) -> String {
    parts.join(" \u{b7} ")
}

/// The words of the line under a game's name, year, genre, players:
/// "1994 · Adventure · 1P". The rating follows it as stars, which the
/// font has no glyphs for, drawn as shapes.
pub fn details_line(d: &Details) -> String {
    let mut parts = Vec::new();
    parts.extend(d.year.map(|y| y.to_string()));
    parts.extend(d.genres.first().cloned());
    parts.extend(d.players.map(players_text));
    joined(parts)
}

/// Year, publisher, genre and players, for the strip over a back scan.
pub fn strip_text(d: &Details) -> String {
    let mut parts = Vec::new();
    parts.extend(d.year.map(|y| y.to_string()));
    parts.extend(d.publisher.clone().or_else(|| d.developer.clone()));
    parts.extend(d.genres.first().cloned());
    parts.extend(d.players.map(players_text));
    joined(parts)
}

/// "1994 · Nintendo": when and who.
pub fn credit_line(d: &Details) -> Option<String> {
    let mut parts = Vec::new();
    parts.extend(d.year.map(|y| y.to_string()));
    parts.extend(d.publisher.clone().or_else(|| d.developer.clone()));
    Some(joined(parts)).filter(|s| !s.is_empty())
}

/// What the box back's bar code says: the game's file name boiled down
/// to ten letters and digits.
pub fn barcode_text(stem: &str) -> String {
    let code: String = stem
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(10)
        .collect::<String>()
        .to_uppercase();
    if code.is_empty() {
        "SHELF".to_string()
    } else {
        code
    }
}

// ---- drawing -------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn fill_rect(buf: &mut [u8], w: u32, h: u32, x: f32, y: f32, rw: f32, rh: f32, rgba: [u8; 4]) {
    let (x0, y0) = (x.round().max(0.0) as u32, y.round().max(0.0) as u32);
    let (x1, y1) = (
        ((x + rw).round().max(0.0) as u32).min(w),
        ((y + rh).round().max(0.0) as u32).min(h),
    );
    let colour = [rgba[0], rgba[1], rgba[2]];
    for py in y0..y1 {
        for px in x0..x1 {
            text::blend_pixel(buf, w, px, py, rgba[3] as u32, colour);
        }
    }
}

/// A star `size` pixels across, as coverage.
pub(crate) fn star_mask(size: u32) -> Vec<u8> {
    let s = size as f32;
    let (cx, cy) = (s / 2.0, s * 0.55);
    let (outer, inner) = (s * 0.52, s * 0.52 * 0.45);
    let points: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let a = (-90.0 + 36.0 * i as f32).to_radians();
            let r = if i % 2 == 0 { outer } else { inner };
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    let inside = |x: f32, y: f32| {
        let mut yes = false;
        let mut j = points.len() - 1;
        for i in 0..points.len() {
            let ((xi, yi), (xj, yj)) = (points[i], points[j]);
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                yes = !yes;
            }
            j = i;
        }
        yes
    };
    let mut out = vec![0u8; (size * size) as usize];
    for py in 0..size {
        for px in 0..size {
            let mut hit = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let (fx, fy) = (
                        px as f32 + (sx as f32 + 0.5) / 4.0,
                        py as f32 + (sy as f32 + 0.5) / 4.0,
                    );
                    hit += inside(fx, fy) as u32;
                }
            }
            out[(py * size + px) as usize] = (hit * 255 / 16) as u8;
        }
    }
    out
}

fn star_gap(size: u32) -> u32 {
    (size / 6).max(1)
}

/// How wide a row of five stars `size` across is.
pub fn stars_width(size: u32) -> u32 {
    5 * size + 4 * star_gap(size)
}

/// Five stars with `halves` halves of them lit, the rest faint, with
/// the top left corner of the first at (x, y).
#[allow(clippy::too_many_arguments)]
fn draw_stars(
    buf: &mut [u8],
    w: u32,
    h: u32,
    x: f32,
    y: f32,
    size: u32,
    halves: u8,
    colour: [u8; 3],
    alpha: u8,
) {
    const FAINT: u32 = 70;
    let mask = star_mask(size);
    for star in 0..5u32 {
        let x0 = x.round() as i32 + (star * (size + star_gap(size))) as i32;
        let y0 = y.round() as i32;
        let lit = |col: u32| {
            let whole = star as u8 * 2 + 2;
            halves >= whole || (halves + 1 == whole && col < size.div_ceil(2))
        };
        for py in 0..size {
            for px in 0..size {
                let (dx, dy) = (x0 + px as i32, y0 + py as i32);
                if dx < 0 || dy < 0 || dx >= w as i32 || dy >= h as i32 {
                    continue;
                }
                let cover = mask[(py * size + px) as usize] as u32;
                let strength = if lit(px) { 255 } else { FAINT };
                let a = cover * strength / 255 * alpha as u32 / 255;
                text::blend_pixel(buf, w, dx as u32, dy as u32, a, colour);
            }
        }
    }
}

/// One centred line of `words` and, after them, the stars, with their
/// middle at height `cy`, shrunk to fit `avail`.
#[allow(clippy::too_many_arguments)]
fn draw_summary(
    buf: &mut [u8],
    w: u32,
    h: u32,
    words: &str,
    halves: Option<u8>,
    (px, min_px): (f32, f32),
    cy: f32,
    avail: f32,
    colour: [u8; 3],
    alpha: u8,
) {
    let Some(font) = text::label_font() else {
        return;
    };
    let size = (px * 0.9).round() as u32;
    let gap = (px * 0.5).round();
    let stars = halves.map(|_| stars_width(size) as f32);
    let shown = match (words.is_empty(), stars) {
        (true, _) => String::new(),
        (false, Some(_)) => format!("{words} \u{b7}"),
        (false, None) => words.to_string(),
    };
    let room = avail - stars.map_or(0.0, |s| s + gap);
    let layout = text::fit(font, &shown, room, 1, px, min_px);
    let line = layout.lines.first().map(String::as_str).unwrap_or("");
    let tw = if line.is_empty() {
        0.0
    } else {
        text::line_width(font, line, layout.px, layout.tracking)
    };
    let total = tw
        + match stars {
            Some(s) if tw > 0.0 => gap + s,
            Some(s) => s,
            None => 0.0,
        };
    let mut x = ((w as f32 - total) / 2.0).max(0.0);
    if tw > 0.0 {
        let base = cy + text::cap_height(font, layout.px) / 2.0;
        text::draw_line(
            buf,
            w,
            h,
            x,
            base,
            line,
            layout.px,
            layout.tracking,
            colour,
            alpha,
        );
        x += tw + gap;
    }
    if let Some(halves) = halves {
        draw_stars(
            buf,
            w,
            h,
            x,
            cy - size as f32 / 2.0,
            size,
            halves,
            colour,
            alpha,
        );
    }
}

/// The line printed under a game's name on the shelf, or nothing when
/// there is nothing to say.
pub fn details_line_face(d: &Details) -> Option<UndoFace> {
    let words = details_line(d);
    let halves = d.rating.map(star_halves);
    if words.is_empty() && halves.is_none() {
        return None;
    }
    let mut rgba = vec![0u8; (DETAILS_W * DETAILS_H * 4) as usize];
    draw_summary(
        &mut rgba,
        DETAILS_W,
        DETAILS_H,
        &words,
        halves,
        (LINE_PX, LINE_MIN_PX),
        DETAILS_H as f32 / 2.0,
        DETAILS_W as f32,
        LINE_INK,
        LINE_ALPHA,
    );
    Some(UndoFace {
        rgba,
        w: DETAILS_W,
        h: DETAILS_H,
    })
}

/// The strip laid over the foot of a back scan that is `scan_w` by
/// `scan_h` where it is drawn, as a face twice that size.
pub fn back_strip_face(d: &Details, scan_w: u32, scan_h: u32) -> CartFace {
    let w = scan_w * BACK_SCALE;
    let h = ((scan_h as f32 * STRIP_FRAC).round() as u32).max(1) * BACK_SCALE;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    fill_rect(
        &mut rgba,
        w,
        h,
        0.0,
        0.0,
        w as f32,
        h as f32,
        [0, 0, 0, 165],
    );
    let s = BACK_SCALE as f32;
    let px = (h as f32 * 0.42).min(13.0 * s);
    draw_summary(
        &mut rgba,
        w,
        h,
        &strip_text(d),
        d.rating.map(star_halves),
        (px, px * 0.6),
        h as f32 / 2.0,
        w as f32 - 16.0 * s,
        INK,
        255,
    );
    CartFace { rgba, w, h }
}

fn light(panel: [f32; 4]) -> bool {
    0.2126 * panel[0] + 0.7152 * panel[1] + 0.0722 * panel[2] > 0.6
}

/// The back of a box that has no scan, `box_w` by `box_h` where it is
/// drawn, made twice that size: the game's title, who made it and when,
/// its genres, players, rating and age rating, a paragraph about it,
/// and a bar code, on a panel of `panel`.
pub fn back_face(d: &Details, stem: &str, box_w: u32, box_h: u32, panel: [f32; 4]) -> CartFace {
    let (w, h) = (box_w * BACK_SCALE, box_h * BACK_SCALE);
    let s = BACK_SCALE as f32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let fill = [panel[0], panel[1], panel[2]].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    for _ in 0..w * h {
        rgba.extend_from_slice(&[fill[0], fill[1], fill[2], 255]);
    }
    let Some(font) = text::label_font() else {
        return CartFace { rgba, w, h };
    };
    let ink = if light(panel) { DARK_INK } else { INK };
    let margin = 12.0 * s;
    let cw = w as f32 - 2.0 * margin;
    let mut y = margin;

    let title = d.name.clone().unwrap_or_else(|| game_name(stem));
    let layout = text::fit(font, &title, cw, 2, 20.0 * s, 12.0 * s);
    let (ascent, pitch) = text::line_metrics(font, layout.px);
    for line in &layout.lines {
        text::draw_line(
            &mut rgba,
            w,
            h,
            margin,
            y + ascent,
            line,
            layout.px,
            layout.tracking,
            ink,
            255,
        );
        y += pitch;
    }
    y += 3.0 * s;
    fill_rect(
        &mut rgba,
        w,
        h,
        margin,
        y,
        cw,
        s,
        [ink[0], ink[1], ink[2], 110],
    );
    y += 7.0 * s;

    let row = 17.0 * s;
    let say = |rgba: &mut Vec<u8>, y: &mut f32, words: &str| {
        let lay = text::fit(font, words, cw, 1, 11.0 * s, 8.0 * s);
        if let Some(line) = lay.lines.first() {
            let base = *y + row / 2.0 + text::cap_height(font, lay.px) / 2.0;
            text::draw_line(
                rgba,
                w,
                h,
                margin,
                base,
                line,
                lay.px,
                lay.tracking,
                ink,
                255,
            );
        }
        *y += row;
    };
    if let Some(credit) = credit_line(d) {
        say(&mut rgba, &mut y, &credit);
    }
    if !d.genres.is_empty() {
        let genres = d.genres.iter().take(3).cloned().collect::<Vec<_>>();
        say(&mut rgba, &mut y, &genres.join(" / "));
    }
    if let Some(players) = players_long(d) {
        say(&mut rgba, &mut y, &players);
    }
    if let Some(halves) = d.rating.map(star_halves) {
        let size = (12.0 * s) as u32;
        let top = y + (row - size as f32) / 2.0;
        draw_stars(&mut rgba, w, h, margin, top, size, halves, ink, 255);
        if let Some(rating) = rating_text(d) {
            let at = margin + stars_width(size) as f32 + 6.0 * s;
            let lay = text::fit(font, &rating, w as f32 - margin - at, 1, 11.0 * s, 8.0 * s);
            if let Some(line) = lay.lines.first() {
                let base = y + row / 2.0 + text::cap_height(font, lay.px) / 2.0;
                text::draw_line(
                    &mut rgba,
                    w,
                    h,
                    at,
                    base,
                    line,
                    lay.px,
                    lay.tracking,
                    ink,
                    255,
                );
            }
        }
        y += row;
    }

    // The foot: age rating on the left, bar code on the right.
    let foot_h = 40.0 * s;
    let foot = h as f32 - margin - foot_h;
    let mut code_left = margin;
    if let Some(esrb) = &d.esrb {
        let badge_w = 34.0 * s;
        fill_rect(
            &mut rgba,
            w,
            h,
            margin,
            foot,
            badge_w,
            foot_h,
            [0, 0, 0, 215],
        );
        let edge = 2.0 * s;
        for (bx, by, bw, bh) in [
            (margin, foot, badge_w, edge),
            (margin, foot + foot_h - edge, badge_w, edge),
            (margin, foot, edge, foot_h),
            (margin + badge_w - edge, foot, edge, foot_h),
        ] {
            fill_rect(&mut rgba, w, h, bx, by, bw, bh, [255, 255, 255, 235]);
        }
        let lay = text::fit(
            font,
            esrb,
            badge_w - 2.0 * edge - 2.0 * s,
            1,
            22.0 * s,
            8.0 * s,
        );
        if let Some(line) = lay.lines.first() {
            let tw = text::line_width(font, line, lay.px, lay.tracking);
            let base = foot + foot_h / 2.0 + text::cap_height(font, lay.px) / 2.0;
            let x = margin + (badge_w - tw) / 2.0;
            text::draw_line(
                &mut rgba,
                w,
                h,
                x,
                base,
                line,
                lay.px,
                lay.tracking,
                [255, 255, 255],
                255,
            );
        }
        code_left = margin + badge_w + 8.0 * s;
    }
    let code_w = w as f32 - margin - code_left;
    draw_barcode(&mut rgba, w, h, (code_left, foot, code_w, foot_h), stem);

    if let Some(text_h) = overview_room(foot, y, s) {
        if let Some(overview) = &d.overview {
            let top = y + 5.0 * s;
            let lay = text::fit_para(font, overview, cw, text_h, 10.0 * s, 8.0 * s);
            let (ascent, pitch) = text::line_metrics(font, lay.px);
            for (i, line) in lay.lines.iter().enumerate() {
                let base = top + i as f32 * pitch + ascent;
                text::draw_line(&mut rgba, w, h, margin, base, line, lay.px, 0.0, ink, 225);
            }
        }
    }
    CartFace { rgba, w, h }
}

/// The height left for the overview between the credits and the foot, if
/// there is room for a line of it.
fn overview_room(foot: f32, y: f32, s: f32) -> Option<f32> {
    let room = foot - 8.0 * s - (y + 5.0 * s);
    (room >= 11.0 * s).then_some(room)
}

/// The bar code of `stem` on a white label in `area` (x, y, w, h), its
/// bars as wide as the label allows, or none if it will not fit.
fn draw_barcode(buf: &mut [u8], w: u32, h: u32, area: (f32, f32, f32, f32), stem: &str) {
    let (ax, ay, aw, ah) = area;
    let code = barcode_text(stem);
    let Some(run) = code39(&format!("*{code}*")) else {
        return;
    };
    let quiet = 6.0 * BACK_SCALE as f32;
    let widths = |n: u32| {
        let wide = ((n as f32 * CODE39_WIDE / CODE39_NARROW).round() as u32).max(n * 2);
        let total: u32 =
            run.iter().map(|b| if *b { wide } else { n }).sum::<u32>() + (run.len() as u32 / 9) * n;
        (wide, total)
    };
    let Some((n, wide, total)) = (1..=2u32).rev().find_map(|n| {
        let (wide, total) = widths(n);
        (total as f32 + 2.0 * quiet <= aw).then_some((n, wide, total))
    }) else {
        return;
    };
    let label_w = total as f32 + 2.0 * quiet;
    let lx = ax + aw - label_w;
    fill_rect(buf, w, h, lx, ay, label_w, ah, [255, 255, 255, 255]);
    let bars_h = ah * 0.68;
    let mut x = lx + quiet;
    for chunk in run.chunks_exact(9) {
        for (i, is_wide) in chunk.iter().enumerate() {
            let ew = if *is_wide { wide } else { n };
            if i % 2 == 0 {
                fill_rect(
                    buf,
                    w,
                    h,
                    x,
                    ay + ah * 0.08,
                    ew as f32,
                    bars_h,
                    [0, 0, 0, 255],
                );
            }
            x += ew as f32;
        }
        x += n as f32;
    }
    if let Some(font) = text::label_font() {
        let lay = text::fit(font, &code, label_w - 2.0 * quiet, 1, ah * 0.17, 6.0);
        if let Some(line) = lay.lines.first() {
            let tw = text::line_width(font, line, lay.px, lay.tracking);
            let base = ay + ah * 0.92;
            let tx = lx + (label_w - tw) / 2.0;
            text::draw_line(
                buf,
                w,
                h,
                tx,
                base,
                line,
                lay.px,
                lay.tracking,
                [0, 0, 0],
                255,
            );
        }
    }
}
