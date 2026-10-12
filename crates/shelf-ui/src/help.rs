//! The button guide Y opens on the shelf: each button and what it does
//! there, one to a line, over the dimmed shelf.

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{
    arrows_hint_face, cap_width, hint_face, up_down_hint_face, UndoFace, ARROW_GAP, CAP, HINT_H,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum HelpLine {
    Insert,
    Browse,
    System,
    Letter,
    Blow,
    Box,
    Random,
    PowerMenu,
    Settings,
    Close,
}

impl HelpLine {
    pub const ALL: [HelpLine; 10] = [
        HelpLine::Insert,
        HelpLine::Browse,
        HelpLine::System,
        HelpLine::Letter,
        HelpLine::Blow,
        HelpLine::Box,
        HelpLine::Random,
        HelpLine::PowerMenu,
        HelpLine::Settings,
        HelpLine::Close,
    ];

    /// The lines the guide shows.
    pub fn shown() -> [HelpLine; 10] {
        HelpLine::ALL
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The button's key cap, or `None` for the arrows.
    fn key(self) -> Option<&'static str> {
        Some(match self {
            HelpLine::Insert => "A",
            HelpLine::Browse | HelpLine::System => return None,
            HelpLine::Letter => "L / R",
            HelpLine::Blow => "X",
            HelpLine::Box => "B",
            HelpLine::Random => "SELECT",
            HelpLine::PowerMenu => "START",
            HelpLine::Settings => "MENU",
            HelpLine::Close => "Y",
        })
    }

    fn label(self) -> &'static str {
        match self {
            HelpLine::Insert => "Insert cart",
            HelpLine::Browse => "Browse",
            HelpLine::System => "Change system",
            HelpLine::Letter => "Jump by letter",
            HelpLine::Blow => "Blow on cart",
            HelpLine::Box => "Box front / back",
            HelpLine::Random => "Random game",
            HelpLine::PowerMenu => "Menu",
            HelpLine::Settings => "Menu",
            HelpLine::Close => "Close",
        }
    }

    /// How wide the key part of the line is, so the words can line up.
    pub fn key_width(self) -> u32 {
        match self.key() {
            Some(key) => cap_width(key),
            None => 2 * CAP + ARROW_GAP,
        }
    }

    pub fn face(self) -> UndoFace {
        match (self, self.key()) {
            (HelpLine::System, _) => up_down_hint_face(self.label()),
            (_, None) => arrows_hint_face(self.label()),
            (_, Some(key)) => hint_face(key, self.label()),
        }
    }
}

pub fn help_faces() -> Vec<UndoFace> {
    HelpLine::ALL.iter().map(|l| l.face()).collect()
}

pub const HELP_PITCH: f32 = 36.0;
pub const HELP_DIM: [f32; 4] = [0.04, 0.04, 0.05, 0.86];

/// The guide over whatever is already drawn. `faces` holds a face and its
/// width for each of `HelpLine::ALL`, in that order.
pub fn draw_help(faces: &[(TexId, u32)], lines: &[HelpLine], out: &mut Vec<Draw>) {
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: OUT_W as f32,
        h: OUT_H as f32,
        colour: HELP_DIM,
    });
    // Keys sit right-aligned in one column so the words start together.
    let rows: Vec<(TexId, u32, u32)> = lines
        .iter()
        .filter_map(|l| faces.get(l.index()).map(|&(t, w)| (t, w, l.key_width())))
        .collect();
    let keys = rows.iter().map(|r| r.2).max().unwrap_or(0);
    let widest = rows.iter().map(|r| keys - r.2 + r.1).max().unwrap_or(0) as f32;
    let left = ((OUT_W as f32 - widest) / 2.0).round();
    let top = (OUT_H as f32 - HELP_PITCH * rows.len() as f32) / 2.0;
    for (row, (tex, w, key)) in rows.into_iter().enumerate() {
        out.push(Draw::Tex {
            x: left + (keys - key) as f32,
            y: (top + HELP_PITCH * row as f32 + (HELP_PITCH - HINT_H as f32) / 2.0).round(),
            w: w as f32,
            h: HINT_H as f32,
            tex,
            alpha: 1.0,
        });
    }
}
