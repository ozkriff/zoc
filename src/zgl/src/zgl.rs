// See LICENSE file for copyright and license details.

use std::mem;
use common::types::{Size2, ZInt, ZFloat};
use types::{Color3, Color4, ScreenPos, AttrId};
use cgmath::{Matrix, Matrix4, Matrix3, ToMatrix4, Vector3, rad, Deg, ToRad, ortho};
use libc::c_void;
use gl;
use gl::Gl;
use gl::types::{GLuint, GLsizeiptr};
use std::ffi::{CStr};

// pub const GREY_3: Color3 = Color3{r: 0.3, g: 0.3, b: 0.3};
pub const BLACK_3: Color3 = Color3{r: 0.0, g: 0.0, b: 0.0};
pub const WHITE: Color4 = Color4{r: 1.0, g: 1.0, b: 1.0, a: 1.0};
pub const BLUE: Color4 = Color4{r: 0.0, g: 0.0, b: 1.0, a: 1.0};
// pub const RED: Color4 = Color4{r: 1.0, g: 0.0, b: 0.0, a: 1.0};
pub const BLACK: Color4 = Color4{r: 0.0, g: 0.0, b: 0.0, a: 1.0};
pub const GREY: Color4 = Color4{r: 0.7, g: 0.7, b: 0.7, a: 1.0};

pub enum MeshRenderMode {
    Triangles,
    Lines,
}

impl MeshRenderMode {
    pub fn to_gl_type(&self) -> GLuint {
        match *self {
            MeshRenderMode::Triangles => gl::TRIANGLES,
            MeshRenderMode::Lines => gl::LINES,
        }
    }
}

pub struct Zgl {
    pub gl: Gl, // TODO: make private
}

impl Zgl {
    pub fn new<F>(get_proc_address: F) -> Zgl
        where F: Fn(&str) -> *const c_void
    {
        let gl = Gl::load_with(|s| get_proc_address(s));
        Zgl{gl: gl}
    }

    pub fn init_opengl(&self) {
        unsafe {
            self.gl.Enable(gl::DEPTH_TEST);
        }
        self.check();
        unsafe {
            self.gl.Enable(gl::BLEND);
        }
        self.check();
        unsafe {
            self.gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.check();
    }

    pub fn print_gl_info(&self) {
        println!("GL_VERSION: {}", self.get_info(gl::VERSION));
        println!("GL_SHADING_LANGUAGE_VERSION: {}", self.get_info(gl::SHADING_LANGUAGE_VERSION));
        println!("GL_VENDOR: {}", self.get_info(gl::VENDOR));
        println!("GL_RENDERER: {}", self.get_info(gl::RENDERER));
        // println!("GL_EXTENSIONS: {}", self.get_info(gl::EXTENSIONS));
    }

    pub fn set_clear_color(&mut self, color: &Color3) {
        unsafe {
            self.gl.ClearColor(color.r, color.g, color.b, 1.0);
        }
        self.check();
    }

    pub fn clear_screen(&self) {
        unsafe {
            self.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        self.check();
    }

    pub fn get_info(&self, name: GLuint) -> String {
        unsafe {
            let version = self.gl.GetString(name) as *const i8;
            String::from_utf8_lossy(CStr::from_ptr(version).to_bytes()).into_owned()
        }
    }

    pub fn set_viewport(&mut self, size: &Size2<ZInt>) {
        unsafe {
            self.gl.Viewport(0, 0, size.w, size.h);
        }
        self.check();
    }

    // TODO: replace with something from cgmath-rs
    // from cgmath-rs`s docs:
    // "Transformations are not usually done directly on matrices, but go
    // through transformation objects that can be converted to matrices.
    // Rotations go through the Basis types, which are guaranteed to be
    // orthogonal matrices."
    pub fn tr(&self, m: Matrix4<ZFloat>, v: &Vector3<ZFloat>) -> Matrix4<ZFloat> {
        let mut t = Matrix4::identity();
        t[3][0] = v.x;
        t[3][1] = v.y;
        t[3][2] = v.z;
        m.mul_m(&t)
    }

    pub fn scale(&self, m: Matrix4<ZFloat>, scale: ZFloat) -> Matrix4<ZFloat> {
        let mut t = Matrix4::identity();
        t[0][0] = scale;
        t[1][1] = scale;
        t[2][2] = scale;
        m.mul_m(&t)
    }

    pub fn rot_x(&self, m: Matrix4<ZFloat>, angle: &Deg<ZFloat>) -> Matrix4<ZFloat> {
        let rad = angle.to_rad();
        let r = Matrix3::from_angle_x(rad).to_matrix4();
        m.mul_m(&r)
    }

    pub fn rot_z(&self, m: Matrix4<ZFloat>, angle: &Deg<ZFloat>) -> Matrix4<ZFloat> {
        let rad = angle.to_rad();
        let r = Matrix3::from_angle_z(rad).to_matrix4();
        m.mul_m(&r)
    }

    pub fn check(&self) {
        let error_code = unsafe { self.gl.GetError() };
        if error_code != 0 {
            let description = match error_code {
                gl::INVALID_ENUM => "GL_INVALID_ENUM",
                gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
                gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
                gl::INVALID_VALUE => "GL_INVALID_VALUE",
                gl::NO_ERROR => "GL_NO_ERROR",
                gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
                _ => panic!("Bad gl error code: {}", error_code),
            };
            panic!("gl error: {}({})", description, error_code);
        }
    }

    pub fn read_pixel_bytes(
        &self,
        win_size: &Size2<ZInt>,
        mouse_pos: &ScreenPos,
    ) -> (ZInt, ZInt, ZInt, ZInt) {
        let height = win_size.h;
        let reverted_h = height - mouse_pos.v.y;
        let mut data = [0u8, 0, 0, 0];
        unsafe {
            self.gl.ReadPixels(
                mouse_pos.v.x, reverted_h, 1, 1,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                mem::transmute(&mut data[0]),
            );
        }
        self.check();
        (data[0] as ZInt, data[1] as ZInt, data[2] as ZInt, data[3] as ZInt)
    }

    pub fn draw_arrays(&self, mode: &MeshRenderMode, length: ZInt) {
        let starting_index = 0;
        unsafe {
            self.gl.DrawArrays(mode.to_gl_type(), starting_index, length);
        }
        self.check();
    }

    pub fn enable_vertex_attrib_array(&self, attr_id: &AttrId) {
        unsafe {
            self.gl.EnableVertexAttribArray(attr_id.id);
        }
        self.check();
    }

    pub fn get_2d_screen_matrix(&self, win_size: &Size2<ZInt>) -> Matrix4<ZFloat> {
        let left = 0.0;
        let right = win_size.w as ZFloat;
        let bottom = 0.0;
        let top = win_size.h as ZFloat;
        let near = -1.0;
        let far = 1.0;
        ortho(left, right, bottom, top, near, far)
    }

    pub fn flush(&self) {
        unsafe {
            self.gl.Flush();
            self.check();
        }
    }
}

pub struct Vbo {
    id: GLuint,
}

impl Vbo {
    pub fn from_data<T>(zgl: &Zgl, data: &[T]) -> Vbo {
        let vbo = Vbo{id: Vbo::get_new_vbo_id(zgl)};
        vbo.bind(zgl);
        let size = mem::size_of::<T>();
        let buf_size = (data.len() * size) as GLsizeiptr;
        if data.len() != 0 {
            unsafe {
                let data_ptr = mem::transmute(&data[0]);
                zgl.gl.BufferData(
                    gl::ARRAY_BUFFER,
                    buf_size,
                    data_ptr,
                    gl::STATIC_DRAW,
                );
                zgl.check();
            }
        }
        vbo
    }

    fn get_new_vbo_id(zgl: &Zgl) -> GLuint {
        let mut id = 0;
        unsafe {
            zgl.gl.GenBuffers(1, &mut id);
            zgl.check();
        }
        id
    }

    pub fn bind(&self, zgl: &Zgl) {
        unsafe {
            zgl.gl.BindBuffer(gl::ARRAY_BUFFER, self.id);
        }
        zgl.check();
    }
}

/*
// TODO: how to pass Zgl to d-tor?
impl Drop for Vbo {
    fn drop(&mut self) {
        unsafe {
            verify!(gl::DeleteBuffers(1, &self.id));
        }
    }
}
*/

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
