//! The start-up screen a seated cart's console shows while its game is
//! handed off and loads, drawn here in the spirit of each console's own:
//! the Game Boy's logo scrolling down to its "ba-ding", the Game Boy
//! Color's letters dropping in, the Game Boy Advance's light sweeping
//! across its name, the Mega Drive's licence screen, and for the NES and
//! SNES, which had none, the picture flickering on. It ends on its last
//! picture rather than fading, so that picture stays up while the game
//! loads. Any button skips it.

use slot_store::Platform;

use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::hud::Millis;
use crate::plate::UndoFace;
use crate::text;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum IntroKind {
    Gb,
    Gbc,
    Gba,
    Md,
    /// The NES and SNES: the screen flickering on.
    Flicker,
}

impl IntroKind {
    pub fn for_platform(platform: Platform) -> Self {
        match platform {
            Platform::Gb => IntroKind::Gb,
            Platform::Gbc => IntroKind::Gbc,
            Platform::Gba => IntroKind::Gba,
            Platform::Md => IntroKind::Md,
            Platform::Nes | Platform::Snes => IntroKind::Flicker,
        }
    }

    pub fn length(self) -> Millis {
        match self {
            IntroKind::Gb => 3600,
            IntroKind::Gbc => 3200,
            IntroKind::Gba => 3200,
            IntroKind::Md => 2600,
            IntroKind::Flicker => 1100,
        }
    }

    /// When the start-up sound plays, from the start of the screen.
    pub fn chime_at(self) -> Millis {
        match self {
            IntroKind::Gb => GB_LANDS,
            IntroKind::Gbc => 900,
            IntroKind::Gba => 700,
            IntroKind::Md => 0,
            IntroKind::Flicker => 40,
        }
    }

    /// The Mega Drive's licence screen came up in silence.
    pub fn has_chime(self) -> bool {
        self != IntroKind::Md
    }
}

/// When the Game Boy's logo comes to rest in the middle.
const GB_LANDS: Millis = 2300;

/// Each line of words the screens use, made into a face by the frontend.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum IntroText {
    GbNintendo,
    GbcGameBoy,
    GbcC,
    GbcO,
    GbcL,
    GbcO2,
    GbcR,
    GbaGameBoy,
    GbaAdvance,
    GbaNintendo,
    Tmss1,
    Tmss2,
    Tmss3,
}

const GB_SCREEN: [f32; 4] = [0.77, 0.81, 0.63, 1.0];
const GB_INK: [u8; 3] = [0x3d, 0x4a, 0x2c];
const NAVY: [u8; 3] = [0x1c, 0x22, 0x6e];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

impl IntroText {
    pub const ALL: [IntroText; 13] = [
        IntroText::GbNintendo,
        IntroText::GbcGameBoy,
        IntroText::GbcC,
        IntroText::GbcO,
        IntroText::GbcL,
        IntroText::GbcO2,
        IntroText::GbcR,
        IntroText::GbaGameBoy,
        IntroText::GbaAdvance,
        IntroText::GbaNintendo,
        IntroText::Tmss1,
        IntroText::Tmss2,
        IntroText::Tmss3,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    fn words(self) -> (&'static str, f32, [u8; 3]) {
        match self {
            IntroText::GbNintendo => ("Nintendo", 44.0, GB_INK),
            IntroText::GbcGameBoy => ("GAME BOY", 52.0, NAVY),
            IntroText::GbcC => ("C", 44.0, [0xe0, 0x2a, 0x3a]),
            IntroText::GbcO => ("O", 44.0, [0x8e, 0x3c, 0xc8]),
            IntroText::GbcL => ("L", 44.0, [0x2c, 0xa8, 0x4a]),
            IntroText::GbcO2 => ("O", 44.0, [0xf0, 0xb0, 0x10]),
            IntroText::GbcR => ("R", 44.0, [0x2a, 0x6c, 0xe0]),
            IntroText::GbaGameBoy => ("GAME BOY", 56.0, NAVY),
            IntroText::GbaAdvance => ("ADVANCE", 30.0, NAVY),
            IntroText::GbaNintendo => ("Nintendo", 22.0, [0x80, 0x80, 0x88]),
            IntroText::Tmss1 => ("PRODUCED BY OR", 26.0, [0xff; 3]),
            IntroText::Tmss2 => ("UNDER LICENSE FROM", 26.0, [0xff; 3]),
            IntroText::Tmss3 => ("SEGA ENTERPRISES LTD.", 26.0, [0xff; 3]),
        }
    }

    pub fn face(self) -> UndoFace {
        let (words, px, ink) = self.words();
        text_face(words, px, ink)
    }
}

/// `words` on one line at `px`, in `ink`, on a clear face just big enough.
pub fn text_face(words: &str, px: f32, ink: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    // Laid out unbounded first, so the face is as wide as the lettering
    // with its spacing, and nothing is cut.
    let layout = text::fit(font, words, 4000.0, 1, px, px);
    let inked = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = inked.ceil() as u32 + 16;
    let h = (px * 1.5).ceil() as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let layout = text::fit(font, words, w as f32, 1, px, px);
    text::draw_centred(&mut rgba, w, h, &layout, ink);
    UndoFace { rgba, w, h }
}

pub fn intro_faces() -> Vec<UndoFace> {
    IntroText::ALL.iter().map(|t| t.face()).collect()
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Intro {
    pub kind: IntroKind,
    started: Millis,
}

impl Intro {
    pub fn started(kind: IntroKind, now: Millis) -> Self {
        Intro { kind, started: now }
    }

    pub fn age(&self, now: Millis) -> Millis {
        now.saturating_sub(self.started)
    }

    pub fn done(&self, now: Millis) -> bool {
        self.age(now) >= self.kind.length()
    }

    /// The screen at this moment. `faces` holds a face and its size for
    /// each of `IntroText::ALL`, in that order.
    pub fn draw(&self, now: Millis, faces: &[(TexId, u32, u32)], out: &mut Vec<Draw>) {
        let t = self.age(now);
        let face = |w: IntroText| faces.get(w.index()).copied();
        let mid_x = OUT_W as f32 / 2.0;
        let mid_y = OUT_H as f32 / 2.0;
        let put = |out: &mut Vec<Draw>, w: IntroText, cx: f32, cy: f32, alpha: f32| {
            if let Some((tex, fw, fh)) = face(w) {
                out.push(Draw::Tex {
                    x: (cx - fw as f32 / 2.0).round(),
                    y: (cy - fh as f32 / 2.0).round(),
                    w: fw as f32,
                    h: fh as f32,
                    tex,
                    alpha: alpha.clamp(0.0, 1.0),
                });
            }
        };
        match self.kind {
            IntroKind::Gb => {
                fill(out, GB_SCREEN);
                // The logo scrolls down a pixel at a time from above the
                // screen and stops in the middle.
                let f = (t as f32 / GB_LANDS as f32).min(1.0);
                let y = (-40.0 + (mid_y + 40.0) * f).floor();
                put(out, IntroText::GbNintendo, mid_x, y - (y % 2.0), 1.0);
            }
            IntroKind::Gbc => {
                fill(out, WHITE);
                let slide = ease((t as f32 / 700.0).min(1.0));
                put(
                    out,
                    IntroText::GbcGameBoy,
                    mid_x + (1.0 - slide) * OUT_W as f32 * 0.6,
                    mid_y - 34.0,
                    1.0,
                );
                let letters = [
                    IntroText::GbcC,
                    IntroText::GbcO,
                    IntroText::GbcL,
                    IntroText::GbcO2,
                    IntroText::GbcR,
                ];
                for (i, letter) in letters.into_iter().enumerate() {
                    let from = 750 + i as Millis * 120;
                    if t < from {
                        continue;
                    }
                    let drop = ease(((t - from) as f32 / 260.0).min(1.0));
                    let x = mid_x + (i as f32 - 2.0) * 38.0;
                    let y = mid_y + 34.0 - (1.0 - drop) * 60.0;
                    put(out, letter, x, y, drop * 1.5);
                }
            }
            IntroKind::Gba => {
                fill(out, WHITE);
                let show = ease((t as f32 / 500.0).min(1.0));
                put(out, IntroText::GbaGameBoy, mid_x, mid_y - 20.0, show);
                // A band of light sweeps across the name.
                let sweep = (t as f32 - 300.0) / 900.0;
                if (0.0..1.0).contains(&sweep) {
                    let x = -80.0 + sweep * (OUT_W as f32 + 160.0);
                    for (dx, a) in [(0.0, 0.35), (14.0, 0.6), (28.0, 0.35)] {
                        out.push(Draw::Rect {
                            x: x + dx,
                            y: mid_y - 60.0,
                            w: 14.0,
                            h: 80.0,
                            colour: [1.0, 1.0, 1.0, a],
                        });
                    }
                }
                let adv = ((t as f32 - 900.0) / 400.0).clamp(0.0, 1.0);
                put(out, IntroText::GbaAdvance, mid_x, mid_y + 28.0, adv);
                let maker = ((t as f32 - 1500.0) / 400.0).clamp(0.0, 1.0);
                put(
                    out,
                    IntroText::GbaNintendo,
                    mid_x,
                    OUT_H as f32 - 70.0,
                    maker,
                );
            }
            IntroKind::Md => {
                fill(out, BLACK);
                let a = ((t as f32 - 150.0) / 250.0).clamp(0.0, 1.0);
                put(out, IntroText::Tmss1, mid_x, mid_y - 34.0, a);
                put(out, IntroText::Tmss2, mid_x, mid_y, a);
                put(out, IntroText::Tmss3, mid_x, mid_y + 34.0, a);
            }
            IntroKind::Flicker => {
                fill(out, BLACK);
                // A grey flash as the picture comes up, and a few bright
                // lines rolling through it, settling to black.
                let f = t as f32 / 450.0;
                if f < 1.0 {
                    let a = (1.0 - f) * (1.0 - f) * 0.7;
                    out.push(Draw::Rect {
                        x: 0.0,
                        y: 0.0,
                        w: OUT_W as f32,
                        h: OUT_H as f32,
                        colour: [0.55, 0.56, 0.6, a],
                    });
                    for k in 0..6u64 {
                        let y = ((k * 83 + t / 7 * 37) % OUT_H as u64) as f32;
                        out.push(Draw::Rect {
                            x: 0.0,
                            y,
                            w: OUT_W as f32,
                            h: 2.0,
                            colour: [0.9, 0.9, 0.95, a],
                        });
                    }
                }
            }
        }
    }
}

fn fill(out: &mut Vec<Draw>, colour: [f32; 4]) {
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: OUT_W as f32,
        h: OUT_H as f32,
        colour,
    });
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
