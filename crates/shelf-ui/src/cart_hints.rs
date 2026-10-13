//! The strip of button hints under a cart taken out of its box, where the
//! buttons mean something else: a blue shelf tag with yellow key caps,
//! as a rental shop would label a shelf.

use crate::cart::CartFace;
use crate::details::{fill_rect, BACK_SCALE};
use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::text;

/// What each button does with the cart out, in the order shown.
pub const CART_HINTS: [(&str, &str); 4] = [
    ("A", "Play"),
    ("X", "From the start"),
    ("Y", "Blow"),
    ("B", "Put back"),
];
/// How far above the foot of the picture the strip's foot sits.
const LIFT: f32 = 78.0;

const BLUE: [u8; 4] = [0x0b, 0x3a, 0x8c, 235];
const GOLD: [u8; 4] = [0xff, 0xd2, 0x00, 255];
const WHITE: [u8; 3] = [0xf6, 0xf4, 0xef];

/// The strip, made `BACK_SCALE` times the size it is drawn at.
pub fn cart_hints_face() -> CartFace {
    let s = BACK_SCALE as f32;
    let h = (30.0 * s) as u32;
    let Some(font) = text::label_font() else {
        return CartFace {
            rgba: vec![0; (4 * h * 4) as usize],
            w: 4,
            h,
        };
    };
    let (cap, key_px, label_px) = (20.0 * s, 14.0 * s, 13.0 * s);
    let (pad, gap, between) = (10.0 * s, 5.0 * s, 16.0 * s);
    let labels: Vec<_> = CART_HINTS
        .iter()
        .map(|(_, l)| text::fit(font, l, 400.0 * s, 1, label_px, label_px))
        .collect();
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| text::line_width(font, &l.lines[0], l.px, l.tracking))
        .collect();
    let w = (2.0 * pad
        + widths.iter().map(|lw| cap + gap + lw).sum::<f32>()
        + between * (CART_HINTS.len() as f32 - 1.0))
        .ceil() as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let fh = h as f32;
    fill_rect(&mut rgba, w, h, 0.0, 0.0, w as f32, fh, BLUE);
    fill_rect(&mut rgba, w, h, 0.0, 0.0, w as f32, 2.0 * s, GOLD);
    let mut x = pad;
    let mid = fh / 2.0 + s;
    for (((key, _), label), lw) in CART_HINTS.iter().zip(&labels).zip(&widths) {
        let top = mid - cap / 2.0;
        fill_rect(&mut rgba, w, h, x, top, cap, cap, GOLD);
        let k = text::fit(font, key, cap, 1, key_px, key_px);
        let kw = text::line_width(font, &k.lines[0], k.px, k.tracking);
        let base = mid + text::cap_height(font, k.px) / 2.0;
        let ink = [BLUE[0], BLUE[1], BLUE[2]];
        text::draw_line(&mut rgba, w, h, x + (cap - kw) / 2.0, base, &k.lines[0], k.px, k.tracking, ink, 255);
        x += cap + gap;
        let base = mid + text::cap_height(font, label.px) / 2.0;
        text::draw_line(&mut rgba, w, h, x, base, &label.lines[0], label.px, label.tracking, WHITE, 255);
        x += lw + between;
    }
    CartFace { rgba, w, h }
}

/// The strip, `w` by `h` as made, centred near the foot of the picture,
/// `alpha` strong.
pub fn draw_cart_hints(tex: TexId, w: u32, h: u32, alpha: f32, out: &mut Vec<Draw>) {
    if alpha <= 0.0 {
        return;
    }
    let s = BACK_SCALE as f32;
    let (dw, dh) = (w as f32 / s, h as f32 / s);
    out.push(Draw::Tex {
        x: ((OUT_W as f32 - dw) / 2.0).round(),
        y: (OUT_H as f32 - LIFT - dh).round(),
        w: dw,
        h: dh,
        tex,
        alpha: alpha.min(1.0),
    });
}
