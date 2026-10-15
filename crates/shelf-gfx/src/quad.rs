pub struct Quad {
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
}

const VERTS: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

pub const POS_LOCATION: gl::types::GLuint = 0;

impl Quad {
    pub fn new() -> Self {
        unsafe {
            let mut vao = 0;
            if !crate::gl::es() {
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);
            }
            let mut vbo = 0;
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(&VERTS) as isize,
                VERTS.as_ptr() as *const std::ffi::c_void,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(POS_LOCATION);
            gl::VertexAttribPointer(POS_LOCATION, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            if !crate::gl::es() {
                gl::BindVertexArray(0);
            }
            Quad { vao, vbo }
        }
    }

    /// Bind the quad's vertices, once for a run of [`Quad::draw_bound`]s.
    pub fn bind(&self) {
        unsafe {
            if crate::gl::es() {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                gl::EnableVertexAttribArray(POS_LOCATION);
                gl::VertexAttribPointer(POS_LOCATION, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            } else {
                gl::BindVertexArray(self.vao);
            }
        }
    }

    pub fn unbind(&self) {
        if !crate::gl::es() {
            unsafe { gl::BindVertexArray(0) };
        }
    }

    pub fn draw_bound(&self) {
        unsafe { gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4) };
    }

    pub fn draw(&self) {
        self.bind();
        self.draw_bound();
        self.unbind();
    }
}

impl Drop for Quad {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            if !crate::gl::es() {
                gl::DeleteVertexArrays(1, &self.vao);
            }
        }
    }
}
