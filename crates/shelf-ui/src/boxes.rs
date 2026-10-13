//! The box shelf: a third way to show a console's games, as box fronts in
//! the dark. The middle box is as large as the screen allows, between the
//! game's name and the slot, so its art can be read; of the boxes either
//! side only an edge shows, at the screen's sides, to scroll to. Boxes come
//! in three shapes (tall for the NES and Mega Drive, square for the Game
//! Boys, wide for the SNES), so each console's row is laid out for its own.

use shelf_gfx::{Draw, OUT_H, OUT_W};
use shelf_store::Platform;

use crate::blow::{rgba, unit, DUST};
use crate::slot_chrome::MOUTH_H;

/// The most a box picture is made for the box shelf. The middle box is
/// drawn at up to about 400 by 560 of the canvas, which the Brick's panel
/// shows at about 1.4 times that, so this is about one picture pixel to
/// one on the panel.
pub const TALL_BOX_W: u32 = 600;
pub const TALL_BOX_H: u32 = 600;
/// How far below the game's details the boxes' tops stay.
pub const BOXES_TITLE_GAP: f32 = 8.0;
/// How far above the slot's mouth the boxes' feet stay.
const FLOOR_GAP: f32 = 10.0;
/// The size the boxes beside the middle one are drawn at, and how much of
/// them shows.
pub const BOXES_SIDE_SCALE: f32 = 0.78;
const BOXES_SIDE_ALPHA: f32 = 0.55;
/// How much of each box beside the middle one shows at the screen's edge.
pub const SLIVER: f32 = 40.0;
/// The least gap between the middle box and the edges either side of it.
const GAP: f32 = 16.0;

/// A console's boxes, by their usual shape: width over height.
pub fn box_aspect(platform: Platform) -> f32 {
    match platform {
        Platform::Nes | Platform::Md | Platform::Sms => 0.70,
        Platform::Gg => 0.72,
        Platform::Snes | Platform::N64 => 1.40,
        Platform::Pce => 0.70,
        Platform::PceCd => 1.13,
        Platform::SegaCd => 0.70,
        Platform::Ps => 1.13,
        Platform::Ngp | Platform::Ngpc => 0.72,
        Platform::Gb | Platform::Gbc | Platform::Gba => 1.0,
    }
}

/// How one console's row of boxes is laid out under a ceiling.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoxesRow {
    /// The space a middle box fills: any box is fitted inside it.
    pub w: f32,
    pub h: f32,
    /// How far apart the boxes stand, middle to middle.
    pub pitch: f32,
    /// Where the boxes' feet stand.
    pub foot: f32,
    /// The size the boxes beside the middle one are drawn at.
    pub side: f32,
}

impl BoxesRow {
    /// The row for `platform`'s boxes with their tops kept at or below
    /// `ceiling`, the foot of the game's name and details.
    pub fn new(platform: Platform, ceiling: f32) -> BoxesRow {
        let floor = OUT_H as f32 - MOUTH_H - FLOOR_GAP;
        let row = BoxesRow::fit(box_aspect(platform), ceiling, floor, SLIVER, BOXES_SIDE_SCALE);
        let room = (floor - ceiling).max(1.0);
        BoxesRow {
            foot: ceiling + (room + row.h) / 2.0,
            ..row
        }
    }

    /// The video store's row of things `aspect` wide for their height (box
    /// fronts, or loose carts), standing on the shelf at `foot` and kept
    /// at or below `ceiling`, with more of the ones either side showing.
    pub fn store(aspect: f32, ceiling: f32, foot: f32) -> BoxesRow {
        BoxesRow::fit(
            aspect,
            ceiling,
            foot,
            crate::store::STORE_SLIVER,
            crate::store::STORE_SIDE_SCALE,
        )
    }

    /// As tall as there is room for between `ceiling` and `floor`, and no
    /// wider than leaves `sliver` of the ones either side showing, clear of
    /// it; standing on `floor`.
    fn fit(aspect: f32, ceiling: f32, floor: f32, sliver: f32, side: f32) -> BoxesRow {
        let room = (floor - ceiling).max(1.0);
        let widest = OUT_W as f32 - 2.0 * (sliver + GAP);
        let w = (room * aspect).min(widest);
        let h = w / aspect;
        // The ones beside it stand off the screen but for `sliver` of them.
        let pitch = OUT_W as f32 / 2.0 - sliver + side * w / 2.0;
        BoxesRow {
            w,
            h,
            pitch,
            foot: floor,
            side,
        }
    }

    /// Where a box whose picture is `pw` by `ph` stands, `offset` places
    /// off the middle: x, y, width and height. It is fitted inside the
    /// row's space and stands on the row's foot, smaller off the middle.
    pub fn rect(&self, (pw, ph): (f32, f32), offset: f32) -> (f32, f32, f32, f32) {
        let fit = if pw > 0.0 && ph > 0.0 {
            (self.w / pw).min(self.h / ph)
        } else {
            0.0
        };
        let (bw, bh) = match fit > 0.0 {
            true => (pw * fit, ph * fit),
            false => (self.w, self.h),
        };
        let scale = 1.0 + (self.side - 1.0) * offset.abs().min(1.0);
        let (w, h) = (bw * scale, bh * scale);
        (
            OUT_W as f32 / 2.0 + offset * self.pitch - w / 2.0,
            self.foot - h,
            w,
            h,
        )
    }
}

/// How much of a box `offset` places off the middle shows.
pub(crate) fn side_alpha(offset: f32) -> f32 {
    1.0 + (BOXES_SIDE_ALPHA - 1.0) * offset.abs().min(1.0)
}

/// The colour of rubbed-off print where a box has been handled.
const WORN: [f32; 3] = [0.86, 0.84, 0.80];

/// Shelf wear on a box at `x`, `y`, `w` by `h`, as dusty as `amount` (0 to
/// 1), shown at `alpha`: corners rubbed pale and a few scuffs along its
/// edges, in the same places every frame for the same `seed`.
pub fn draw_wear(
    (x, y, w, h): (f32, f32, f32, f32),
    amount: f32,
    alpha: f32,
    seed: u32,
    out: &mut Vec<Draw>,
) {
    let amount = amount.clamp(0.0, 1.0);
    if amount <= 0.0 || alpha <= 0.0 || w < 16.0 || h < 16.0 {
        return;
    }
    let base = seed.wrapping_mul(0x9e37_79b9);
    let at = |k: u32| unit(base.wrapping_add(k));
    // Any wear at all shows a little; the dustiest boxes show the most.
    let a = (0.22 + 0.33 * amount) * alpha;
    // Rubbed corners: a small square and a smaller one inside it, so the
    // wear fades in from the very corner.
    for (i, (cx, cy)) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        .into_iter()
        .enumerate()
    {
        let size = (4.0 + 5.0 * (0.4 + 0.6 * at(i as u32)) * amount).min(w / 4.0);
        for (s, share) in [(size, 0.55), (size * 0.5, 1.0)] {
            out.push(Draw::Rect {
                x: x + cx * (w - s),
                y: y + cy * (h - s),
                w: s,
                h: s,
                colour: rgba(WORN, a * share),
            });
        }
    }
    // Scuffs along the edges: thin pale streaks hugging one side.
    let scuffs = 2 + (2.0 * amount).round() as u32;
    for i in 0..scuffs {
        let k = 8 + i * 4;
        let along = 0.1 + 0.8 * at(k);
        let len = (0.08 + 0.14 * at(k + 1)) * if i % 2 == 0 { h } else { w };
        let thick = 1.0 + at(k + 2) * 1.5;
        let r = match (i + (at(k + 3) * 4.0) as u32) % 4 {
            0 => (x, y + along * (h - len), thick, len),
            1 => (x + w - thick, y + along * (h - len), thick, len),
            2 => (x + along * (w - len), y, len, thick),
            _ => (x + along * (w - len), y + h - thick, len, thick),
        };
        out.push(Draw::Rect {
            x: r.0,
            y: r.1,
            w: r.2,
            h: r.3,
            colour: rgba(WORN, a * 0.8),
        });
    }
    // A breath of the grime the carts get, much lighter.
    out.push(Draw::Rect {
        x,
        y,
        w,
        h,
        colour: rgba(DUST, 0.06 * amount * alpha),
    });
}

/// A scan of a box's back, fitted to the most any box is drawn at: the box
/// shelf's, the largest. It is always drawn inside the front's outline, so
/// its own size only sets how sharp it is.
pub fn box_back_scan(path: &std::path::Path) -> Option<crate::CartFace> {
    let (rgba, w, h) = crate::art::fit(path, TALL_BOX_W, TALL_BOX_H)?;
    Some(crate::CartFace { rgba, w, h })
}

/// The box picture at `path`, fitted to the most the box shelf draws one.
pub fn tall_box_face(path: &std::path::Path) -> Option<crate::CartFace> {
    let (rgba, w, h) = crate::art::fit(path, TALL_BOX_W, TALL_BOX_H)?;
    Some(crate::CartFace { rgba, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ceiling on a 4:3 panel, as the app works it out: the name at 14
    /// under the 60 rows of band, its details below it.
    const CEILING: f32 = 14.0 - 60.0 + 32.0 + 20.0 + BOXES_TITLE_GAP;

    fn rows() -> impl Iterator<Item = (Platform, BoxesRow)> {
        Platform::ALL
            .into_iter()
            .map(|p| (p, BoxesRow::new(p, CEILING)))
    }

    fn nominal(p: Platform) -> (f32, f32) {
        (box_aspect(p) * 100.0, 100.0)
    }

    #[test]
    fn the_middle_box_is_on_the_screen_under_the_name_and_over_the_slot() {
        for (p, row) in rows() {
            for aspect in [0.55, box_aspect(p), 1.0, 1.45] {
                // At rest, and overshooting it either way; part way along a
                // scroll it slides off the side, as the carts do.
                for off in [0.0, 0.12, -0.12] {
                    let (x, y, w, h) = row.rect((aspect * 100.0, 100.0), off);
                    assert!(x >= 0.0 && x + w <= OUT_W as f32, "{p:?} {aspect} {off}");
                    assert!(y >= CEILING - 0.01, "{p:?} {aspect} {off}: top {y}");
                    assert!(y + h <= OUT_H as f32 - MOUTH_H, "{p:?} {aspect} {off}");
                }
            }
        }
    }

    #[test]
    fn of_the_boxes_beside_the_middle_only_an_edge_shows_clear_of_it() {
        for (p, row) in rows() {
            let size = nominal(p);
            let (mx, _, mw, _) = row.rect(size, 0.0);
            let (x, _, w, _) = row.rect(size, 1.0);
            let shows = OUT_W as f32 - x;
            assert!((shows - SLIVER).abs() < 0.5, "{p:?}: shows {shows} of {w}");
            assert!(x - (mx + mw) >= GAP - 0.01, "{p:?}: gap {}", x - (mx + mw));
            let (x, _, w, _) = row.rect(size, -1.0);
            assert!((x + w - SLIVER).abs() < 0.5, "{p:?}");
            // The ones two off are out of sight.
            let (x, _, _, _) = row.rect(size, 2.0);
            assert!(x >= OUT_W as f32, "{p:?}");
        }
    }

    #[test]
    fn the_middle_box_fills_the_room_under_the_name() {
        let room = OUT_H as f32 - MOUTH_H - FLOOR_GAP - CEILING;
        for p in [Platform::Nes, Platform::Md, Platform::Gb, Platform::Gba] {
            let row = BoxesRow::new(p, CEILING);
            assert!(row.h >= 0.99 * room, "{p:?}: {} of {room}", row.h);
        }
        // The SNES's wide boxes are as wide as leaves the edges showing.
        let row = BoxesRow::new(Platform::Snes, CEILING);
        assert!(row.w >= 500.0, "{}", row.w);
    }

    #[test]
    fn a_box_with_no_picture_fills_the_row_space() {
        let row = BoxesRow::new(Platform::Md, CEILING);
        let (_, _, w, h) = row.rect((0.0, 0.0), 0.0);
        assert_eq!((w, h), (row.w, row.h));
    }

    #[test]
    fn no_wear_without_dust() {
        let mut out = Vec::new();
        draw_wear((0.0, 0.0, 100.0, 140.0), 0.0, 1.0, 7, &mut out);
        assert!(out.is_empty());
        draw_wear((0.0, 0.0, 100.0, 140.0), 0.5, 1.0, 7, &mut out);
        assert!(!out.is_empty());
        let inside = |d: &Draw| match d {
            Draw::Rect { x, y, w, h, .. } => {
                *x >= 0.0 && *y >= 0.0 && x + w <= 100.01 && y + h <= 140.01
            }
            _ => false,
        };
        assert!(out.iter().all(inside));
    }
}
