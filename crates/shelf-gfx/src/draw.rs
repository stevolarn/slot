use crate::quad::Quad;
use crate::shaders::{SPRITE_FRAG, SPRITE_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TexId(usize);

#[cfg(feature = "test-support")]
impl TexId {
    pub const fn from_raw(id: usize) -> Self {
        TexId(id)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Draw {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        colour: [f32; 4],
    },
    Tex {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tex: TexId,
        alpha: f32,
    },
    Turned {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tex: TexId,
        alpha: f32,
        turn: f32,
    },
}

/// The most a coordinate or alpha may differ, in canvas pixels, for two
/// frames to look the same: well under what the eye or a pixel can show.
const SAME_WITHIN: f32 = 1.0 / 256.0;

impl Draw {
    fn looks_like(&self, other: &Draw) -> bool {
        let near = |a: f32, b: f32| (a - b).abs() <= SAME_WITHIN;
        let all = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(a, b)| near(*a, *b));
        match (*self, *other) {
            (
                Draw::Rect { x, y, w, h, colour },
                Draw::Rect {
                    x: x2,
                    y: y2,
                    w: w2,
                    h: h2,
                    colour: c2,
                },
            ) => all(&[x, y, w, h], &[x2, y2, w2, h2]) && all(&colour, &c2),
            (
                Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                },
                Draw::Tex {
                    x: x2,
                    y: y2,
                    w: w2,
                    h: h2,
                    tex: t2,
                    alpha: a2,
                },
            ) => tex == t2 && all(&[x, y, w, h, alpha], &[x2, y2, w2, h2, a2]),
            (
                Draw::Turned {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                    turn,
                },
                Draw::Turned {
                    x: x2,
                    y: y2,
                    w: w2,
                    h: h2,
                    tex: t2,
                    alpha: a2,
                    turn: n2,
                },
            ) => tex == t2 && all(&[x, y, w, h, alpha, turn], &[x2, y2, w2, h2, a2, n2]),
            _ => false,
        }
    }
}

/// Whether two draw lists paint the same picture, give or take a hair of
/// a pixel, so a frame that would repeat the last need not be drawn.
pub fn same_frame(a: &[Draw], b: &[Draw]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.looks_like(b))
}

pub struct Sprites {
    prog: gl::types::GLuint,
    white: gl::types::GLuint,
    textures: Vec<gl::types::GLuint>,
    /// The size each texture was last given, so a rewrite of the same size
    /// can reuse its storage.
    sizes: Vec<(u32, u32)>,
    writes: u64,
    u_rect: gl::types::GLint,
    u_colour: gl::types::GLint,
    u_turn: gl::types::GLint,
}

impl Sprites {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(SPRITE_VERT, SPRITE_FRAG)?;
        let white = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            Some(&[255u8; 4]),
        );
        let (u_rect, u_colour, u_turn);
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_tex"), 0);
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_target"),
                OUT_W as f32,
                OUT_H as f32,
            );
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_colour = crate::gl::uniform_location(prog, "u_colour");
            u_turn = crate::gl::uniform_location(prog, "u_turn");
        }
        Ok(Sprites {
            prog,
            white,
            textures: Vec::new(),
            sizes: Vec::new(),
            writes: 0,
            u_rect,
            u_colour,
            u_turn,
        })
    }

    pub fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.push(w, h, rgba, gl::LINEAR)
    }

    pub fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.push(w, h, rgba, gl::NEAREST)
    }

    fn push(&mut self, w: u32, h: u32, rgba: &[u8], filter: gl::types::GLenum) -> TexId {
        let tex = crate::gl::texture(w, h, filter, gl::CLAMP_TO_EDGE, gl::RGBA, Some(rgba));
        self.writes += 1;
        self.textures.push(tex);
        self.sizes.push((w, h));
        TexId(self.textures.len() - 1)
    }

    pub fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        let Some(tex) = self.textures.get(id.0) else {
            return;
        };
        if rgba.len() < (w * h * 4) as usize {
            return;
        }
        self.writes += 1;
        let same = self.sizes.get(id.0) == Some(&(w, h));
        self.sizes[id.0] = (w, h);
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, *tex);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            if same {
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    rgba.as_ptr() as *const std::ffi::c_void,
                );
            } else {
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    crate::gl::internal_format(gl::RGBA),
                    w as i32,
                    h as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    rgba.as_ptr() as *const std::ffi::c_void,
                );
            }
        }
    }

    /// How many textures have been made or rewritten, so a caller can tell
    /// a picture changed under a draw list that did not.
    pub fn writes(&self) -> u64 {
        self.writes
    }

    pub fn draw(&self, items: &[Draw], quad: &Quad) {
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::UseProgram(self.prog);
            gl::ActiveTexture(gl::TEXTURE0);
        }
        quad.bind();
        for item in items {
            let (x, y, w, h, tex, colour, turn) = match *item {
                Draw::Rect { x, y, w, h, colour } => (x, y, w, h, self.white, colour, 0.0),
                Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                } => match self.textures.get(tex.0) {
                    Some(t) => (x, y, w, h, *t, [1.0, 1.0, 1.0, alpha], 0.0),
                    None => continue,
                },
                Draw::Turned {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha,
                    turn,
                } => match self.textures.get(tex.0) {
                    Some(t) => (x, y, w, h, *t, [1.0, 1.0, 1.0, alpha], turn),
                    None => continue,
                },
            };
            let (cos, sin) = if turn == 0.0 {
                (1.0, 0.0)
            } else {
                (turn.cos(), turn.sin())
            };
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, tex);
                gl::Uniform4f(self.u_rect, x, y, w, h);
                gl::Uniform2f(self.u_turn, cos, sin);
                gl::Uniform4f(self.u_colour, colour[0], colour[1], colour[2], colour[3]);
            }
            quad.draw_bound();
        }
        quad.unbind();
        unsafe { gl::Disable(gl::BLEND) };
    }
}

impl Drop for Sprites {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(self.textures.len() as i32, self.textures.as_ptr());
            gl::DeleteTextures(1, &self.white);
            gl::DeleteProgram(self.prog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(x: f32, alpha: f32) -> Draw {
        Draw::Tex {
            x,
            y: 4.0,
            w: 10.0,
            h: 10.0,
            tex: TexId(3),
            alpha,
        }
    }

    #[test]
    fn identical_lists_match_and_a_hair_of_drift_does_not_count() {
        let a = [tex(5.0, 1.0)];
        assert!(same_frame(&a, &a));
        assert!(same_frame(&a, &[tex(5.001, 0.999)]));
    }

    #[test]
    fn a_visible_move_a_fade_a_new_texture_or_a_new_length_is_a_new_frame() {
        let a = [tex(5.0, 1.0)];
        assert!(!same_frame(&a, &[tex(5.1, 1.0)]));
        assert!(!same_frame(&a, &[tex(5.0, 0.9)]));
        let other = Draw::Tex {
            x: 5.0,
            y: 4.0,
            w: 10.0,
            h: 10.0,
            tex: TexId(4),
            alpha: 1.0,
        };
        assert!(!same_frame(&a, &[other]));
        assert!(!same_frame(&a, &[]));
        let rect = Draw::Rect {
            x: 5.0,
            y: 4.0,
            w: 10.0,
            h: 10.0,
            colour: [0.0; 4],
        };
        assert!(!same_frame(&a, &[rect]));
        assert!(same_frame(&[rect], &[rect]));
    }
}
