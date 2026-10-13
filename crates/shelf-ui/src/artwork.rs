//! The Get Artwork screen: what it will fetch, then how far it has got,
//! over the dimmed screen, with the buttons for it along the bottom.

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{centred_hints, hint_face, title_face, UndoFace, HINT_H, LEGEND_GAP};

/// What the screen says: a line per row, and the buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtworkText {
    pub lines: Vec<String>,
    pub hints: Vec<(&'static str, &'static str)>,
}

/// The screen's words as textures: the title, a face and size per line,
/// and a face and width per button hint.
pub struct ArtworkFaces {
    pub text: ArtworkText,
    pub title: (TexId, u32, u32),
    pub lines: Vec<(TexId, u32, u32)>,
    pub hints: Vec<(TexId, u32)>,
}

pub const ARTWORK_TITLE: &str = "Get Artwork";
/// Who the pictures and details come from, shown under them.
pub const ARTWORK_CREDIT: &str = "Images and details: LaunchBox Games Database";

pub fn artwork_title_face() -> UndoFace {
    title_face(ARTWORK_TITLE)
}

const LINE_W: u32 = 660;
const LINE_H: u32 = 28;
const LINE_PX: f32 = 20.0;
const LINE_MIN_PX: f32 = 13.0;
const LINE_INK: [u8; 3] = [0xf6, 0xf4, 0xef];

/// A line of the screen, as wide as the screen allows, shrunk to fit.
pub fn artwork_line_face(line: &str) -> UndoFace {
    let mut rgba = vec![0u8; (LINE_W * LINE_H * 4) as usize];
    if let Some(font) = crate::text::label_font() {
        let layout = crate::text::fit(font, line, LINE_W as f32, 1, LINE_PX, LINE_MIN_PX);
        crate::text::draw_centred(&mut rgba, LINE_W, LINE_H, &layout, LINE_INK);
    }
    UndoFace {
        rgba,
        w: LINE_W,
        h: LINE_H,
    }
}

pub fn artwork_hint_face(key: &str, label: &str) -> UndoFace {
    hint_face(key, label)
}

const TITLE_Y: f32 = 70.0;
const LINES_Y: f32 = 140.0;
const LINE_PITCH: f32 = 34.0;
const BAR_W: f32 = 440.0;
const BAR_H: f32 = 10.0;
const HINTS_Y: f32 = 427.0;
const DIM: [f32; 4] = [0.03, 0.03, 0.04, 0.94];
const BAR_BACK: [f32; 4] = [1.0, 1.0, 1.0, 0.15];
const BAR_FILL: [f32; 4] = [0.98, 0.80, 0.18, 1.0];

/// The screen over what is drawn already; `progress`, 0 to 1, draws a bar
/// under the lines.
pub fn draw_artwork(faces: Option<&ArtworkFaces>, progress: Option<f32>, out: &mut Vec<Draw>) {
    let band = crate::shelf::band();
    out.push(Draw::Rect {
        x: 0.0,
        y: -band,
        w: OUT_W as f32,
        h: OUT_H as f32 + band,
        colour: DIM,
    });
    let Some(faces) = faces else {
        return;
    };
    let centre = |w: u32| ((OUT_W as f32 - w as f32) / 2.0).round();
    let (tex, w, h) = faces.title;
    out.push(Draw::Tex {
        x: centre(w),
        y: TITLE_Y,
        w: w as f32,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
    let mut y = LINES_Y;
    for &(tex, w, h) in &faces.lines {
        out.push(Draw::Tex {
            x: centre(w),
            y,
            w: w as f32,
            h: h as f32,
            tex,
            alpha: 1.0,
        });
        y += LINE_PITCH;
    }
    if let Some(p) = progress {
        let x = centre(BAR_W as u32);
        let y = y + LINE_PITCH * 0.5;
        out.push(Draw::Rect {
            x,
            y,
            w: BAR_W,
            h: BAR_H,
            colour: BAR_BACK,
        });
        out.push(Draw::Rect {
            x,
            y,
            w: BAR_W * p.clamp(0.0, 1.0),
            h: BAR_H,
            colour: BAR_FILL,
        });
    }
    for (tex, w, x) in centred_hints(&faces.hints, LEGEND_GAP) {
        out.push(Draw::Tex {
            x,
            y: HINTS_Y,
            w: w as f32,
            h: HINT_H as f32,
            tex,
            alpha: 1.0,
        });
    }
}
