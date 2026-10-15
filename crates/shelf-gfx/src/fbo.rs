use crate::draw::{Draw, Sprites, TexId};
use crate::quad::Quad;
use crate::shaders::{BLIT_FRAG, BLIT_VERT};
use crate::surface::{blit_rect, GfxError, Surface, OUT_H, OUT_W};

pub const BACKDROP: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub struct Compositor {
    fbo: gl::types::GLuint,
    tex: gl::types::GLuint,
    size: (u32, u32),
    blit: gl::types::GLuint,
    u_blit_uv: gl::types::GLint,
    split: Option<f32>,
    lift: f32,
    quad: Quad,
    sprites: Sprites,
}

impl Compositor {
    pub fn new(surface: &dyn Surface) -> Result<Self, GfxError> {
        crate::gl::load(surface);
        let blit = crate::shaders::program(BLIT_VERT, BLIT_FRAG)?;
        let tex = crate::gl::texture(OUT_W, OUT_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::RGBA, None);
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            if status != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteFramebuffers(1, &fbo);
                gl::DeleteTextures(1, &tex);
                gl::DeleteProgram(blit);
                return Err(GfxError::Framebuffer(status));
            }
            gl::UseProgram(blit);
            gl::Uniform1i(crate::gl::uniform_location(blit, "u_tex"), 0);
            let u_blit_uv = crate::gl::uniform_location(blit, "u_uv");
            gl::Uniform4f(u_blit_uv, 0.0, 0.0, 1.0, 1.0);

            Ok(Compositor {
                fbo,
                tex,
                size: (OUT_W, OUT_H),
                blit,
                u_blit_uv,
                split: None,
                lift: 1.0,
                quad: Quad::new(),
                sprites: Sprites::new()?,
            })
        }
    }

    pub fn draw_list(&mut self, items: &[Draw]) {
        self.sprites.draw(items, &self.quad);
    }

    pub fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.sprites.create_texture(w, h, rgba)
    }

    pub fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.sprites.create_texture_nearest(w, h, rgba)
    }

    pub fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        self.sprites.update_texture(id, w, h, rgba);
    }

    /// How many textures have been made or rewritten so far.
    pub fn texture_writes(&self) -> u64 {
        self.sprites.writes()
    }

    pub fn fit(&mut self, window: (u32, u32)) {
        self.sprites.set_band(band(window));
        let size = canvas_size(window);
        if size == self.size {
            return;
        }
        let tex = crate::gl::texture(
            size.0,
            size.1,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            None,
        );
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::DeleteTextures(1, &self.tex);
        }
        self.tex = tex;
        self.size = size;
    }

    pub fn set_frame_split(&mut self, canvas_y: Option<f32>) {
        self.split = canvas_y;
    }

    pub fn set_frame_lift(&mut self, t: f32) {
        self.lift = t;
    }

    pub fn read_frame(&self) -> Vec<u8> {
        let (w, h) = self.size;
        let stride = w as usize * 4;
        let mut buf = vec![0u8; stride * h as usize];
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadPixels(
                0,
                0,
                w as i32,
                h as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
            );
        }
        let mut top_down = Vec::with_capacity(buf.len());
        for row in buf.chunks_exact(stride).rev() {
            top_down.extend_from_slice(row);
        }
        top_down
    }

    pub fn begin_frame(&mut self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
            gl::ClearColor(BACKDROP[0], BACKDROP[1], BACKDROP[2], BACKDROP[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn end_frame(&mut self, window: (u32, u32)) {
        self.output(window);
    }

    fn output(&mut self, window: (u32, u32)) {
        if band(window) > 0.0 {
            // The canvas covers the whole panel, band and all.
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::Viewport(0, 0, window.0 as i32, window.1 as i32);
                gl::UseProgram(self.blit);
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.tex);
            }
            self.blit_part(0, 0, window.0 as i32, window.1 as i32, [0.0, 0.0, 1.0, 1.0]);
            return;
        }
        let framed = crate::panel::framed(window);
        let (x, mut y, w, h) = blit_rect(window, 0.0);
        let drop = if framed {
            crate::panel::lifted_rect(window, self.lift).1
        } else {
            0
        };
        y -= drop;
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, window.0 as i32, window.1 as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.blit);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.tex);
        }
        let cut = self
            .split
            .filter(|_| framed && drop == 0)
            .map(|c| (c * h as f32 / OUT_H as f32).round() as i32)
            .filter(|c| (1..h).contains(c));
        match cut {
            Some(cut) => {
                let ch = h as f32;
                let gap = window.1 as i32 - h;
                let below = h - cut;
                let top = window.1 as i32 - cut;
                self.blit_part(
                    x,
                    top,
                    w,
                    cut,
                    [0.0, below as f32 / ch, 1.0, cut as f32 / ch],
                );
                self.blit_part(x, 0, w, below, [0.0, 0.0, 1.0, below as f32 / ch]);
                let row = (below as f32 + 0.5) / ch;
                self.blit_part(x, below, w, gap, [0.0, row, 1.0, 0.0]);
                unsafe { gl::Uniform4f(self.u_blit_uv, 0.0, 0.0, 1.0, 1.0) };
            }
            None => self.blit_part(x, y, w, h, [0.0, 0.0, 1.0, 1.0]),
        }
    }
}

impl Compositor {
    fn blit_part(&self, x: i32, y: i32, w: i32, h: i32, uv: [f32; 4]) {
        unsafe {
            gl::Viewport(x, y, w, h);
            gl::Uniform4f(self.u_blit_uv, uv[0], uv[1], uv[2], uv[3]);
        }
        self.quad.draw();
    }
}

/// How many canvas rows a taller-than-3:2 panel adds above the 720x480
/// picture. On a 4:3 panel the canvas covers the whole panel, and these
/// rows, at negative y, sit above the picture's top edge.
pub fn band(window: (u32, u32)) -> f32 {
    if !crate::panel::framed(window) {
        return 0.0;
    }
    (OUT_W as f32 * window.1 as f32 / window.0 as f32 - OUT_H as f32).max(0.0)
}

pub fn canvas_size(window: (u32, u32)) -> (u32, u32) {
    if band(window) > 0.0 {
        return window;
    }
    let (_, _, w, h) = blit_rect(window, 0.0);
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    if w % OUT_W == 0 && h % OUT_H == 0 && w / OUT_W == h / OUT_H {
        (OUT_W, OUT_H)
    } else {
        (w, h)
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteTextures(1, &self.tex);
            gl::DeleteProgram(self.blit);
        }
    }
}
