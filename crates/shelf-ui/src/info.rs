//! The info card: a box back Shelf makes for a game, big enough to read
//! on the Brick, as a rental shop's own back would be. It has the game's
//! name, its title screen and a screenshot of it being played, the blurb,
//! and a shop's index card of its facts, with the rental sticker on it.

use shelf_store::{Details, Platform};

use crate::cart::CartFace;
use crate::details::{credit_line, draw_stars, fill_rect, star_halves, stars_width, BACK_SCALE};
use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::hud::Millis;
use crate::plate::game_name;
use crate::rental::{blit_turned, draw_rental, rental_size, RentalSticker};
use crate::text;

/// The card's size on the canvas; its face is made `BACK_SCALE` times
/// bigger so it is sharp on the Brick's screen.
pub const INFO_W: u32 = 470;
pub const INFO_H: u32 = 524;
/// How long the card takes to rise in front of the box, or drop away.
pub const INFO_MS: Millis = 260;
/// Screenshots are kept at most this big for the card.
pub const SHOT_MAX: (u32, u32) = (440, 330);

const PANEL: [u8; 4] = [0x0b, 0x0d, 0x1e, 255];
const CREAM: [u8; 4] = [0xfb, 0xf6, 0xe4, 255];
const CARD_BLUE: [u8; 3] = [0x0b, 0x3a, 0x8c];
const INK: [u8; 3] = [0xf2, 0xf2, 0xf2];
const DARK: [u8; 3] = [0x14, 0x14, 0x14];
const TITLE_RED: [u8; 3] = [0xff, 0x3b, 0x30];
const TITLE_GOLD: [u8; 3] = [0xff, 0xd2, 0x00];

/// A screenshot as decoded: its pixels, width and height.
pub type Shot = (Vec<u8>, u32, u32);

/// The screenshot at `path`, fitted to the size the card draws it.
pub fn info_shot(path: &std::path::Path) -> Option<Shot> {
    crate::art::fit(path, SHOT_MAX.0, SHOT_MAX.1)
}

fn byte(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The card for a game: `d` its details, `stem` its ROM's name for when
/// the details have no name, on `platform`'s colours, with whichever of
/// its title screen and play screenshot there are, and the rental
/// sticker for a game that has been played.
pub fn info_face(
    d: &Details,
    stem: &str,
    platform: Platform,
    shots: &[Shot],
    rental: Option<&RentalSticker>,
) -> CartFace {
    let s = BACK_SCALE as f32;
    let (w, h) = (INFO_W * BACK_SCALE, INFO_H * BACK_SCALE);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        rgba.extend_from_slice(&PANEL);
    }
    let Some(font) = text::label_font() else {
        return CartFace { rgba, w, h };
    };
    let accent = crate::accent_of(Some(platform));
    let band = [byte(accent[0]), byte(accent[1]), byte(accent[2]), 255];
    let band_ink = if crate::details::light(accent) {
        DARK
    } else {
        INK
    };
    let (fw, fh) = (w as f32, h as f32);

    // The spine down the right, with the console's name along it.
    let spine = 40.0 * s;
    let left = fw - spine;
    fill_rect(&mut rgba, w, h, left, 0.0, spine, fh, band);
    spine_name(&mut rgba, w, h, left, spine, platform.name(), band_ink);

    // The strip along the top: how many play, and who made it.
    let strip = 30.0 * s;
    let margin = 13.0 * s;
    fill_rect(&mut rgba, w, h, 0.0, 0.0, left, strip, band);
    let players = d.players.map(|n| match n {
        0 | 1 => "1 PLAYER".to_string(),
        2 => "1 OR 2 PLAYERS".to_string(),
        n => format!("1 TO {n} PLAYERS"),
    });
    let maker = d.publisher.clone().or_else(|| d.developer.clone());
    let strip_line = |rgba: &mut Vec<u8>, words: &str, right: bool, room: f32| {
        let lay = text::fit(font, words, room, 1, 14.0 * s, 9.0 * s);
        if let Some(line) = lay.lines.first() {
            let lw = text::line_width(font, line, lay.px, lay.tracking);
            let x = if right { left - margin - lw } else { margin };
            let base = strip / 2.0 + text::cap_height(font, lay.px) / 2.0;
            text::draw_line(
                rgba,
                w,
                h,
                x,
                base,
                line,
                lay.px,
                lay.tracking,
                band_ink,
                255,
            );
        }
    };
    let half = (left - 3.0 * margin) / 2.0;
    if let Some(p) = &players {
        strip_line(&mut rgba, p, false, half);
    }
    if let Some(m) = &maker {
        strip_line(&mut rgba, m, true, half);
    }

    // The name, as loud as a box's.
    let cw = left - 2.0 * margin;
    let mut y = strip + 9.0 * s;
    let title = d.name.clone().unwrap_or_else(|| game_name(stem));
    let lay = text::fit(font, &title, cw, 2, 30.0 * s, 16.0 * s);
    let (ascent, pitch) = text::line_metrics(font, lay.px);
    for line in &lay.lines {
        let base = y + ascent;
        for (off, ink) in [(3.0, DARK), (1.5, TITLE_GOLD), (0.0, TITLE_RED)] {
            let o = off * s;
            text::draw_line(
                &mut rgba,
                w,
                h,
                margin + o,
                base + o,
                line,
                lay.px,
                lay.tracking,
                ink,
                255,
            );
        }
        y += pitch * 0.92;
    }
    y += 6.0 * s;

    // The screenshots, side by side in white frames, a little askew.
    if !shots.is_empty() {
        let gap = 10.0 * s;
        let sw = (cw - gap) / 2.0;
        let sh = sw * 0.7;
        let n = shots.len().min(2);
        let row_w = sw * n as f32 + gap * (n as f32 - 1.0);
        let mut x = margin + (cw - row_w) / 2.0;
        for (i, shot) in shots.iter().take(2).enumerate() {
            let deg = if i == 0 { -2.0 } else { 1.5 };
            let framed = framed_shot(shot, sw.round() as u32, sh.round() as u32, (3.0 * s) as u32);
            let (fw, fh) = (framed.1, framed.2);
            let centre = (x + sw / 2.0, y + sh / 2.0 + 3.0 * s);
            let drop = 4.0 * s;
            blit_turned(
                &mut rgba,
                w,
                h,
                &framed.0,
                (fw, fh),
                (centre.0 + drop, centre.1 + drop),
                deg,
                Some(([0, 0, 0], 200)),
            );
            blit_turned(&mut rgba, w, h, &framed.0, (fw, fh), centre, deg, None);
            x += sw + gap;
        }
        y += sh + 16.0 * s;
    }

    // The shop's card of facts at the foot, the sticker on its corner.
    let card_h = 96.0 * s;
    let card_top = fh - margin - card_h;
    let card = facts_card(d, cw.round() as u32, card_h.round() as u32);
    let card_centre = (margin + cw / 2.0, card_top + card_h / 2.0);
    blit_turned(
        &mut rgba,
        w,
        h,
        &card,
        (cw.round() as u32, card_h.round() as u32),
        (card_centre.0 + 4.0 * s, card_centre.1 + 4.0 * s),
        -1.0,
        Some(([0, 0, 0], 130)),
    );
    blit_turned(
        &mut rgba,
        w,
        h,
        &card,
        (cw.round() as u32, card_h.round() as u32),
        card_centre,
        -1.0,
        None,
    );
    if let Some(r) = rental {
        let sticker_h = 30.0 * s;
        let (sw, _) = rental_size(sticker_h);
        let at = (margin + cw - sw / 2.0 + 4.0 * s, card_top - 2.0 * s);
        draw_rental(&mut rgba, w, h, r, at, sticker_h, 5.0);
    }

    // The blurb between the pictures and the card.
    if let Some(overview) = &d.overview {
        let room = card_top - 10.0 * s - y;
        if room > 14.0 * s {
            let lay = text::fit_para(font, overview, cw, room, 15.0 * s, 10.0 * s);
            let (ascent, pitch) = text::line_metrics(font, lay.px);
            for line in &lay.lines {
                text::draw_line(
                    &mut rgba,
                    w,
                    h,
                    margin,
                    y + ascent,
                    line,
                    lay.px,
                    0.0,
                    INK,
                    255,
                );
                y += pitch;
            }
        }
    }
    CartFace { rgba, w, h }
}

/// The console's name, reading down the spine `width` wide at `left`.
fn spine_name(rgba: &mut [u8], w: u32, h: u32, left: f32, width: f32, name: &str, ink: [u8; 3]) {
    let Some(font) = text::label_font() else {
        return;
    };
    let s = BACK_SCALE as f32;
    let long = h as f32 - 40.0 * s;
    let lay = text::fit(font, name, long, 1, width * 0.62, 10.0 * s);
    let Some(line) = lay.lines.first() else {
        return;
    };
    let tracking = lay.tracking + 4.0 * s;
    let lw = text::line_width(font, line, lay.px, tracking).min(long);
    let (bw, bh) = (lw.ceil() as u32 + 2, width.ceil() as u32);
    let mut buf = vec![0u8; (bw * bh * 4) as usize];
    let base = bh as f32 / 2.0 + text::cap_height(font, lay.px) / 2.0;
    text::draw_line(
        &mut buf, bw, bh, 1.0, base, line, lay.px, tracking, ink, 255,
    );
    let centre = (left + width / 2.0, h as f32 / 2.0);
    blit_turned(rgba, w, h, &buf, (bw, bh), centre, 90.0, None);
}

/// A screenshot cropped to fill `w` by `h` and framed in white `edge` wide.
fn framed_shot((src, sw, sh): &Shot, w: u32, h: u32, edge: u32) -> (Vec<u8>, u32, u32) {
    let (fw, fh) = (w.max(2 * edge + 1), h.max(2 * edge + 1));
    let mut out = vec![255u8; (fw * fh * 4) as usize];
    let (iw, ih) = (fw - 2 * edge, fh - 2 * edge);
    // Fill the frame, cutting off whichever way the shot is too long.
    let scale = (iw as f32 / *sw as f32).max(ih as f32 / *sh as f32);
    let (ox, oy) = (
        (*sw as f32 - iw as f32 / scale) / 2.0,
        (*sh as f32 - ih as f32 / scale) / 2.0,
    );
    for y in 0..ih {
        let sy = ((oy + (y as f32 + 0.5) / scale) as u32).min(sh - 1);
        for x in 0..iw {
            let sx = ((ox + (x as f32 + 0.5) / scale) as u32).min(sw - 1);
            let from = ((sy * sw + sx) * 4) as usize;
            let to = (((y + edge) * fw + x + edge) * 4) as usize;
            out[to..to + 3].copy_from_slice(&src[from..from + 3]);
            out[to + 3] = 255;
        }
    }
    (out, fw, fh)
}

/// The shop's index card: genre, players and year along the top, who
/// made it and the stars below, under a blue band.
fn facts_card(d: &Details, w: u32, h: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        buf.extend_from_slice(&CREAM);
    }
    let Some(font) = text::label_font() else {
        return buf;
    };
    let s = BACK_SCALE as f32;
    let (fw, fh) = (w as f32, h as f32);
    let band = 7.0 * s;
    fill_rect(
        &mut buf,
        w,
        h,
        0.0,
        0.0,
        fw,
        band,
        [CARD_BLUE[0], CARD_BLUE[1], CARD_BLUE[2], 255],
    );
    let pad = 10.0 * s;
    let col = (fw - 2.0 * pad) / 3.0;
    let row_h = (fh - band - 6.0 * s) / 2.0;
    let fact = |buf: &mut Vec<u8>, at: (f32, f32), room: f32, key: &str, value: &str| {
        let k = text::fit(font, key, room, 1, 9.0 * s, 7.0 * s);
        let (ka, kp) = text::line_metrics(font, k.px);
        if let Some(line) = k.lines.first() {
            text::draw_line(
                buf,
                w,
                h,
                at.0,
                at.1 + ka,
                line,
                k.px,
                k.tracking,
                CARD_BLUE,
                255,
            );
        }
        let v = text::fit(font, value, room, 1, 16.0 * s, 9.0 * s);
        let (va, _) = text::line_metrics(font, v.px);
        if let Some(line) = v.lines.first() {
            text::draw_line(
                buf,
                w,
                h,
                at.0,
                at.1 + kp + va * 0.95,
                line,
                v.px,
                v.tracking,
                DARK,
                255,
            );
        }
    };
    let top = band + 5.0 * s;
    if let Some(genre) = d.genres.first() {
        fact(&mut buf, (pad, top), col * 1.3 - 6.0 * s, "GENRE", genre);
    }
    let players = match (d.players, d.coop) {
        (Some(0 | 1), _) => Some("1".to_string()),
        (Some(n), true) => Some(format!("1-{n} co-op")),
        (Some(n), false) => Some(format!("1-{n}")),
        (None, _) => None,
    };
    if let Some(players) = players {
        fact(
            &mut buf,
            (pad + col * 1.3, top),
            col * 1.1 - 6.0 * s,
            "PLAYERS",
            &players,
        );
    }
    if let Some(year) = d.year {
        fact(
            &mut buf,
            (pad + col * 2.45, top),
            col * 0.55,
            "YEAR",
            &year.to_string(),
        );
    }
    let second = top + row_h;
    let made = d.developer.clone().or_else(|| credit_line(d));
    if let Some(made) = made {
        fact(&mut buf, (pad, second), col * 1.9, "MADE BY", &made);
    }
    if let Some(halves) = d.rating.map(star_halves) {
        let size = (13.0 * s) as u32;
        let x = fw - pad - stars_width(size) as f32;
        let k = text::fit(font, "MEMBERS RATE IT", fw - x, 1, 9.0 * s, 7.0 * s);
        let (ka, kp) = text::line_metrics(font, k.px);
        if let Some(line) = k.lines.first() {
            text::draw_line(
                &mut buf,
                w,
                h,
                x,
                second + ka,
                line,
                k.px,
                k.tracking,
                CARD_BLUE,
                255,
            );
        }
        draw_stars(
            &mut buf,
            w,
            h,
            x,
            second + kp + 2.0 * s,
            size,
            halves,
            DARK,
            255,
        );
    }
    buf
}

/// The card in front of the box, `t` of the way up from below the screen
/// (0 gone, 1 in place), over the dimmed shelf.
pub fn draw_info(tex: TexId, t: f32, out: &mut Vec<Draw>) {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        return;
    }
    let band = crate::shelf::band();
    let ease = 1.0 - (1.0 - t) * (1.0 - t);
    out.push(Draw::Rect {
        x: 0.0,
        y: -band,
        w: OUT_W as f32,
        h: OUT_H as f32 + band,
        colour: [0.0, 0.0, 0.0, 0.6 * ease],
    });
    let (w, h) = (INFO_W as f32, INFO_H as f32);
    let rest = -band + (OUT_H as f32 + band - h) / 2.0;
    let y = rest + (1.0 - ease) * (OUT_H as f32 + band - rest);
    let x = (OUT_W as f32 - w) / 2.0;
    out.push(Draw::Rect {
        x: x + 8.0,
        y: y + 8.0,
        w,
        h,
        colour: [0.0, 0.0, 0.0, 0.5],
    });
    out.push(Draw::Tex {
        x,
        y,
        w,
        h,
        tex,
        alpha: 1.0,
    });
}
