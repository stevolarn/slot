use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{arrows_hint_face, centred_hints, hint_face, UndoFace, HINT_H, LEGEND_GAP};
use crate::power_menu::{MENU_H, MENU_INK, MENU_PAD, MENU_PX};
use crate::slot_chrome::{edge, opening};
use crate::text;
use shelf_store::{DisplayMode, SortKey};

/// The rows of the menu: the top level, then the ones under Options.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickRow {
    Options,
    Search,
    GetArtwork,
    Guide,
    About,
    Receipts,
    Restart,
    PowerOff,
    Exit,
    Display,
    View,
    Sort,
    Crt,
    Dusty,
    Resume,
    ShowHidden,
    PrintReceipts,
    Boot,
    DateTime,
    Favourite,
    Hide,
    Emulator,
}

impl QuickRow {
    pub const ALL: [QuickRow; 22] = [
        QuickRow::Options,
        QuickRow::Search,
        QuickRow::GetArtwork,
        QuickRow::Guide,
        QuickRow::About,
        QuickRow::Receipts,
        QuickRow::Restart,
        QuickRow::PowerOff,
        QuickRow::Exit,
        QuickRow::Display,
        QuickRow::View,
        QuickRow::Sort,
        QuickRow::Crt,
        QuickRow::Dusty,
        QuickRow::Resume,
        QuickRow::ShowHidden,
        QuickRow::PrintReceipts,
        QuickRow::Boot,
        QuickRow::DateTime,
        QuickRow::Favourite,
        QuickRow::Hide,
        QuickRow::Emulator,
    ];

    /// What the menu opens on.
    pub const TOP: [QuickRow; 9] = [
        QuickRow::Options,
        QuickRow::Search,
        QuickRow::GetArtwork,
        QuickRow::Guide,
        QuickRow::About,
        QuickRow::Receipts,
        QuickRow::Restart,
        QuickRow::PowerOff,
        QuickRow::Exit,
    ];

    /// What Options opens.
    pub const SETTINGS: [QuickRow; 10] = [
        QuickRow::Display,
        QuickRow::View,
        QuickRow::Sort,
        QuickRow::Crt,
        QuickRow::Dusty,
        QuickRow::Resume,
        QuickRow::ShowHidden,
        QuickRow::PrintReceipts,
        QuickRow::Boot,
        QuickRow::DateTime,
    ];

    /// What Start opens: what to do with the game in the middle.
    pub const GAME: [QuickRow; 3] = [QuickRow::Favourite, QuickRow::Hide, QuickRow::Emulator];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            QuickRow::Options => "Options",
            QuickRow::Search => "Search",
            QuickRow::Hide => "Hide Game",
            QuickRow::Favourite => "Favourite",
            QuickRow::Emulator => "Emulator",
            QuickRow::GetArtwork => "Get Artwork",
            QuickRow::Guide => "Button Guide",
            QuickRow::About => "About",
            QuickRow::Restart => "Restart",
            QuickRow::PowerOff => "Power Off",
            QuickRow::Exit => "Exit",
            QuickRow::Display => "Theme",
            QuickRow::View => "View",
            QuickRow::Sort => "Sort",
            QuickRow::Crt => "CRT Mode",
            QuickRow::Dusty => "Dusty Carts",
            QuickRow::Resume => "Resume Saves",
            QuickRow::ShowHidden => "Show Hidden",
            QuickRow::DateTime => "Date & Time",
            QuickRow::Receipts => "Receipts",
            QuickRow::PrintReceipts => "Print Receipts",
            QuickRow::Boot => "Open at Power-On",
        }
    }

    /// Whether this row is under Options.
    pub fn in_options(self) -> bool {
        QuickRow::SETTINGS.contains(&self)
    }

    /// Whether this row is on Start's menu for the game in the middle.
    pub fn in_game(self) -> bool {
        QuickRow::GAME.contains(&self)
    }

    /// The rows on the same screen of the menu as this one.
    pub fn level(self) -> &'static [QuickRow] {
        if self.in_game() {
            &QuickRow::GAME
        } else if self.in_options() {
            &QuickRow::SETTINGS
        } else {
            &QuickRow::TOP
        }
    }

    /// Where the row stands among the rows of its screen.
    pub fn position(self) -> usize {
        self.level().iter().position(|r| *r == self).unwrap_or(0)
    }

    /// Whether A does something from this row, rather than the left and
    /// right buttons changing a setting on it.
    pub fn opens(self) -> bool {
        !matches!(
            self,
            QuickRow::Display
                | QuickRow::View
                | QuickRow::Sort
                | QuickRow::Crt
                | QuickRow::Dusty
                | QuickRow::Resume
                | QuickRow::ShowHidden
                | QuickRow::PrintReceipts
                | QuickRow::Boot
                | QuickRow::Favourite
                | QuickRow::Emulator
        )
    }

    pub fn up(self) -> QuickRow {
        let rows = self.level();
        rows[(self.position() + rows.len() - 1) % rows.len()]
    }

    pub fn down(self) -> QuickRow {
        let rows = self.level();
        rows[(self.position() + 1) % rows.len()]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickValue {
    On,
    Off,
    Original,
    Store,
    Carts,
    Boxes,
    ByName,
    ByGenre,
    ByYear,
    ByPublisher,
    ByRating,
    ByPlays,
    Default,
    Gpsp,
    Mgba,
    Gambatte,
    SuperGb,
    Snes9x,
    Supafaust,
    PceFast,
    SuperGrafx,
}

impl QuickValue {
    pub const ALL: [QuickValue; 21] = [
        QuickValue::On,
        QuickValue::Off,
        QuickValue::Original,
        QuickValue::Store,
        QuickValue::Carts,
        QuickValue::Boxes,
        QuickValue::ByName,
        QuickValue::ByGenre,
        QuickValue::ByYear,
        QuickValue::ByPublisher,
        QuickValue::ByRating,
        QuickValue::ByPlays,
        QuickValue::Default,
        QuickValue::Gpsp,
        QuickValue::Mgba,
        QuickValue::Gambatte,
        QuickValue::SuperGb,
        QuickValue::Snes9x,
        QuickValue::Supafaust,
        QuickValue::PceFast,
        QuickValue::SuperGrafx,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            QuickValue::On => "On",
            QuickValue::Off => "Off",
            QuickValue::Original => "Original",
            QuickValue::Store => "Video Store",
            QuickValue::Carts => "Carts",
            QuickValue::Boxes => "Boxes",
            QuickValue::ByName => "Name",
            QuickValue::ByGenre => "Genre",
            QuickValue::ByYear => "Year",
            QuickValue::ByPublisher => "Publisher",
            QuickValue::ByRating => "Rating",
            QuickValue::ByPlays => "Plays",
            QuickValue::Default => "Default",
            QuickValue::Gpsp => "gpSP",
            QuickValue::Mgba => "mGBA",
            QuickValue::Gambatte => "Gambatte",
            QuickValue::SuperGb => "Super GB",
            QuickValue::Snes9x => "Snes9x",
            QuickValue::Supafaust => "Supafaust",
            QuickValue::PceFast => "PCE",
            QuickValue::SuperGrafx => "SuperGrafx",
        }
    }

    /// The Theme row: the video store, or the original shelves.
    pub fn display(mode: DisplayMode) -> QuickValue {
        match mode {
            DisplayMode::Store => QuickValue::Store,
            DisplayMode::Carts | DisplayMode::Boxes => QuickValue::Original,
        }
    }

    /// The View row: carts or boxes, in either theme.
    pub fn view(carts: bool) -> QuickValue {
        match carts {
            true => QuickValue::Carts,
            false => QuickValue::Boxes,
        }
    }

    pub fn sort(key: SortKey) -> QuickValue {
        match key {
            SortKey::Name => QuickValue::ByName,
            SortKey::Genre => QuickValue::ByGenre,
            SortKey::Year => QuickValue::ByYear,
            SortKey::Publisher => QuickValue::ByPublisher,
            SortKey::Rating => QuickValue::ByRating,
            SortKey::Plays => QuickValue::ByPlays,
        }
    }

    /// The Emulator row: the emulator pak, by its tag.
    pub fn emu(tag: &str) -> QuickValue {
        match tag.to_ascii_uppercase().as_str() {
            "GBA" => QuickValue::Gpsp,
            "MGBA" => QuickValue::Mgba,
            "GB" | "GBC" => QuickValue::Gambatte,
            "SGB" => QuickValue::SuperGb,
            "SFC" => QuickValue::Snes9x,
            "SUPA" => QuickValue::Supafaust,
            "PCE" => QuickValue::PceFast,
            "SGX" => QuickValue::SuperGrafx,
            _ => QuickValue::Default,
        }
    }

    pub fn flag(on: bool) -> QuickValue {
        if on {
            QuickValue::On
        } else {
            QuickValue::Off
        }
    }
}

pub const QUICK_PITCH: f32 = 42.0;
/// Both screens of the menu start at the same height, so Options does not
/// shift the rows: the longer screen sits centred above the legend, and the
/// Options heading over it reaches into the band a 4:3 screen adds.
pub const QUICK_TOP: f32 = (QUICK_SPLIT - QUICK_PITCH * QuickRow::SETTINGS.len() as f32) / 2.0;
/// Start's menu has few rows and sits centred.
const GAME_TOP: f32 = (QUICK_SPLIT - QUICK_PITCH * QuickRow::GAME.len() as f32) / 2.0;
pub const QUICK_EDGE: f32 = 32.0;
const BAR_INSET: f32 = 4.0;
const TYPE_DROP: f32 = 4.0;
const CARET_GAP: f32 = 14.0;
const CARET_PX: f32 = 24.0;
const LEGEND_Y: f32 = 427.0;
pub const QUICK_SPLIT: f32 = LEGEND_Y - 2.0;
const DIM_INK: [u8; 3] = [0x9a, 0x9a, 0xa4];

pub fn quick_label_face(row: QuickRow) -> UndoFace {
    quick_text_face(row.label(), MENU_INK)
}

/// The small heading over Start's menu.
pub fn quick_game_heading_face() -> UndoFace {
    quick_text_face("THIS GAME", DIM_INK)
}

/// What the Hide Game row says over a game already hidden, shown while
/// hidden games are: it sits after the rows' own labels.
pub const UNHIDE_LABEL: &str = "Unhide Game";

pub fn quick_unhide_face() -> UndoFace {
    quick_text_face(UNHIDE_LABEL, MENU_INK)
}

pub fn quick_value_face(text: &str, lit: bool) -> UndoFace {
    quick_text_face(text, if lit { MENU_INK } else { DIM_INK })
}

fn quick_text_face(label: &str, colour: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, label, OUT_W as f32, 1, MENU_PX, MENU_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = set.ceil() as u32 + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    text::draw_centred(&mut rgba, w, MENU_H, &layout, colour);
    UndoFace { rgba, w, h: MENU_H }
}

/// The small heading over the Options screen.
pub fn quick_heading_face() -> UndoFace {
    quick_text_face("OPTIONS", DIM_INK)
}

pub fn quick_caret_face(right: bool) -> UndoFace {
    let glyph = if right { '\u{f0da}' } else { '\u{f0d9}' };
    let (Some(symbols), Some(label)) = (crate::icon::symbols_font(), text::label_font()) else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (m, cov) = symbols.rasterize(glyph, CARET_PX);
    let (w, h) = (m.width as u32, MENU_H);
    let centre = match label.horizontal_line_metrics(MENU_PX) {
        Some(v) => {
            (h as f32 - v.new_line_size) / 2.0 + v.ascent
                - label.metrics('H', MENU_PX).height as f32 / 2.0
        }
        None => h as f32 / 2.0,
    };
    let top = (centre - m.height as f32 / 2.0).round() as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for gy in 0..m.height {
        let y = top + gy as i32;
        if y < 0 || y >= h as i32 {
            continue;
        }
        for gx in 0..m.width {
            let at = ((y as u32 * w + gx as u32) * 4) as usize;
            let a = cov[gy * m.width + gx];
            rgba[at..at + 4].copy_from_slice(&[MENU_INK[0], MENU_INK[1], MENU_INK[2], a]);
        }
    }
    UndoFace { rgba, w, h }
}

pub fn quick_legend_faces() -> [UndoFace; 3] {
    [
        hint_face("B", "Back"),
        arrows_hint_face("Change"),
        hint_face("A", "Open"),
    ]
}

pub struct QuickMenuFaces {
    pub labels: Vec<(TexId, u32, u32)>,
    pub values: Vec<[(TexId, u32, u32); 2]>,
    pub carets: [(TexId, u32, u32); 2],
    pub heading: Option<(TexId, u32, u32)>,
    /// The heading over Start's menu.
    pub game_heading: Option<(TexId, u32, u32)>,
    pub legend: [(TexId, u32); 3],
}

pub struct QuickMenu<'a> {
    pub row: QuickRow,
    pub values: [Option<QuickValue>; QuickRow::ALL.len()],
    pub clock: Option<[(TexId, u32, u32); 2]>,
    pub faces: Option<&'a QuickMenuFaces>,
    /// The game in the middle is hidden: Hide Game reads Unhide Game.
    pub unhide: bool,
}

impl QuickMenu<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: -crate::shelf::band(),
            w: OUT_W as f32,
            h: OUT_H as f32 + crate::shelf::band(),
            colour: opening(),
        });
        out.push(Draw::Rect {
            x: 0.0,
            y: row_top(self.row) + BAR_INSET,
            w: OUT_W as f32,
            h: QUICK_PITCH - 2.0 * BAR_INSET,
            colour: edge(),
        });
        let Some(faces) = self.faces else {
            return;
        };
        let (right, pad) = (OUT_W as f32 - QUICK_EDGE, MENU_PAD as f32);
        let heading = match self.row.in_game() {
            true => faces.game_heading,
            false => faces.heading.filter(|_| self.row.in_options()),
        };
        if let Some((tex, w, h)) = heading {
            let y = level_top(self.row) - QUICK_PITCH * 0.9 + TYPE_DROP;
            push(out, tex, QUICK_EDGE - pad, y, w, h);
        }
        for &row in self.row.level() {
            let y = row_top(row) + TYPE_DROP;
            let lit = row == self.row;
            let label = match row == QuickRow::Hide && self.unhide {
                true => faces.labels.get(QuickRow::ALL.len()),
                false => None,
            };
            if let Some(&(tex, w, h)) = label.or(faces.labels.get(row.index())) {
                push(out, tex, QUICK_EDGE - pad, y, w, h);
            }
            let value = match row {
                QuickRow::DateTime => self.clock.map(|c| c[lit as usize]),
                // Options opens a screen of its own: a caret points the way.
                QuickRow::Options => {
                    let (tex, w, h) = faces.carets[1];
                    push(out, tex, right - w as f32, y, w, h);
                    continue;
                }
                _ => self.values[row.index()]
                    .and_then(|v| faces.values.get(v.index()))
                    .map(|v| v[lit as usize]),
            };
            let Some((tex, w, h)) = value else {
                continue;
            };
            if !lit || row.opens() {
                push(out, tex, right + pad - w as f32, y, w, h);
                continue;
            }
            let [(left_tex, lw, lh), (right_tex, rw, rh)] = faces.carets;
            let rx = right - rw as f32;
            push(out, right_tex, rx, y, rw, rh);
            let vx = rx - CARET_GAP + pad - w as f32;
            push(out, tex, vx, y, w, h);
            push(out, left_tex, vx + pad - CARET_GAP - lw as f32, y, lw, lh);
        }
        let [back, change, open] = faces.legend;
        let other = if self.row.opens() { open } else { change };
        for (tex, w, x) in centred_hints(&[back, other], LEGEND_GAP) {
            push(out, tex, x, LEGEND_Y, w, HINT_H);
        }
    }
}

fn level_top(row: QuickRow) -> f32 {
    if row.in_game() {
        GAME_TOP
    } else {
        QUICK_TOP
    }
}

fn row_top(row: QuickRow) -> f32 {
    level_top(row) + QUICK_PITCH * row.position() as f32
}

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: w as f32,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
}
