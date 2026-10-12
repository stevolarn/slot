//! The dust on carts left on the shelf: a thin grey-brown film over the
//! cart and a scatter of specks, thicker the longer the cart has sat.
//! Blowing on a cart takes it off.

use shelf_gfx::Draw;

use crate::blow::{rgba, unit, DUST, SPECK};

/// How opaque the film is on a cart that is fully dusty.
pub const FILM_MAX: f32 = 0.30;
/// The most specks drawn on the cart in front, and on one at the side.
pub const FRONT_SPECKS: u32 = 40;
pub const SIDE_SPECKS: u32 = 16;

/// A number to scatter one cart's specks by, from its name, so they sit
/// in the same places every frame.
pub fn seed_of(name: &str) -> u32 {
    name.bytes().fold(0x811c_9dc5u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

/// Dust of `amount` (0 to 1) over the cart at `x`, `y`, `w` by `h`, shown
/// at `alpha`: the film, then up to `max_specks` specks, all inside the
/// cart's rectangle.
/// How far in from a cart's edges, as a fraction of it, the specks keep.
const SPECK_INSET: f32 = 0.07;

pub fn draw_dust(
    (x, y, w, h): (f32, f32, f32, f32),
    amount: f32,
    alpha: f32,
    seed: u32,
    max_specks: u32,
    out: &mut Vec<Draw>,
) {
    let amount = amount.clamp(0.0, 1.0);
    if amount <= 0.0 || alpha <= 0.0 || w < 1.0 || h < 1.0 {
        return;
    }
    // The film lies over the label, which is inside every cart's outline:
    // over the cart's whole box it showed as a pale box round a rounded or
    // clear cart. The specks keep a little in from the edges for the same
    // reason.
    let (lx, ly, lw, lh) = crate::shelf::PLACEHOLDER_LABEL;
    out.push(Draw::Rect {
        x: x + w * lx,
        y: y + h * ly,
        w: w * lw,
        h: h * lh,
        colour: rgba(DUST, FILM_MAX * amount * alpha),
    });
    let (x, y, w, h) = (x + w * SPECK_INSET, y + h * SPECK_INSET, w * (1.0 - 2.0 * SPECK_INSET), h * (1.0 - 2.0 * SPECK_INSET));
    let specks = (max_specks as f32 * amount).ceil() as u32;
    let base = seed.wrapping_mul(0x85eb_ca6b);
    for i in 0..specks {
        let at = |k: u32| unit(base.wrapping_add(i * 4 + k));
        let size = (1.5 + 2.0 * at(2)).min(w).min(h);
        let sx = x + at(0) * (w - size);
        let sy = y + at(1) * (h - size);
        let a = (0.35 + 0.35 * at(3)) * (0.5 + 0.5 * amount) * alpha;
        out.push(Draw::Rect {
            x: sx.round(),
            y: sy.round(),
            w: size,
            h: size,
            colour: rgba(SPECK, a),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: (f32, f32, f32, f32) = (100.0, 50.0, 200.0, 120.0);

    fn dust(amount: f32, alpha: f32, seed: u32, max: u32) -> Vec<Draw> {
        let mut out = Vec::new();
        draw_dust(RECT, amount, alpha, seed, max, &mut out);
        out
    }

    #[test]
    fn a_clean_cart_draws_nothing() {
        assert!(dust(0.0, 1.0, 1, FRONT_SPECKS).is_empty());
        assert!(dust(0.8, 0.0, 1, FRONT_SPECKS).is_empty());
    }

    #[test]
    fn the_film_never_goes_past_its_ceiling_and_specks_are_capped() {
        let out = dust(1.0, 1.0, 7, FRONT_SPECKS);
        assert_eq!(out.len(), 1 + FRONT_SPECKS as usize);
        match out[0] {
            Draw::Rect { colour, .. } => assert!((colour[3] - FILM_MAX).abs() < 1e-6),
            _ => panic!("the film is a rectangle"),
        }
        assert_eq!(
            dust(1.0, 1.0, 7, SIDE_SPECKS).len(),
            1 + SIDE_SPECKS as usize
        );
    }

    #[test]
    fn more_dust_means_more_specks_and_a_thicker_film() {
        let light = dust(0.2, 1.0, 3, FRONT_SPECKS);
        let heavy = dust(0.9, 1.0, 3, FRONT_SPECKS);
        assert!(light.len() < heavy.len());
        let film = |out: &[Draw]| match out[0] {
            Draw::Rect { colour, .. } => colour[3],
            _ => 0.0,
        };
        assert!(film(&light) < film(&heavy));
    }

    #[test]
    fn the_specks_stay_inside_the_cart_and_do_not_shimmer() {
        let (x, y, w, h) = RECT;
        for seed in [0, 1, 99, seed_of("Emerald")] {
            let a = dust(1.0, 1.0, seed, FRONT_SPECKS);
            assert_eq!(a, dust(1.0, 1.0, seed, FRONT_SPECKS), "specks moved");
            for d in &a {
                let Draw::Rect {
                    x: dx,
                    y: dy,
                    w: dw,
                    h: dh,
                    ..
                } = *d
                else {
                    panic!("only rectangles");
                };
                assert!(dx >= x - 0.5 && dx + dw <= x + w + 0.5, "{dx} {dw}");
                assert!(dy >= y - 0.5 && dy + dh <= y + h + 0.5, "{dy} {dh}");
            }
        }
        assert_ne!(
            dust(1.0, 1.0, seed_of("Emerald"), FRONT_SPECKS),
            dust(1.0, 1.0, seed_of("Fusion"), FRONT_SPECKS),
            "two carts are dusted the same way"
        );
    }

    #[test]
    fn a_fainter_cart_gets_a_fainter_dusting() {
        let alpha = |out: &[Draw]| match out[0] {
            Draw::Rect { colour, .. } => colour[3],
            _ => 0.0,
        };
        assert!(alpha(&dust(1.0, 0.55, 1, SIDE_SPECKS)) < alpha(&dust(1.0, 1.0, 1, SIDE_SPECKS)));
    }
}
