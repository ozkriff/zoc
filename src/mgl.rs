// See LICENSE file for copyright and license details.

#![macro_escape]

// use std;
// use gl;
// use gl::types::{GLuint, GLsizeiptr};
// use cgmath::{Vector3, rad, ortho};
// use core_misc::deg_to_rad;
use core_types::{Size2, MInt};
use visualizer_types::{Color3, Color4, ColorId};
// use cgmath::{Matrix, Matrix4, Matrix3, ToMatrix4};
use libc::c_void;
use gl;
use gl::types::{GLuint, GLint, GLenum, GLchar};
use std::ptr;
use std::str;
use std::mem;
use std::c_str::CString;

use gl::Gles2 as Gl;

/*
pub const GREY_3: Color3 = Color3{r: 0.3, g: 0.3, b: 0.3};
pub const BLACK_3: Color3 = Color3{r: 0.0, g: 0.0, b: 0.0};
pub const WHITE: Color4 = Color4{r: 1.0, g: 1.0, b: 1.0, a: 1.0};
pub const BLUE: Color4 = Color4{r: 0.0, g: 0.0, b: 1.0, a: 1.0};
pub const BLACK: Color4 = Color4{r: 0.0, g: 0.0, b: 0.0, a: 1.0};
*/

pub struct Mgl {
    pub gl: Gl,
}

impl Mgl {
    pub fn new(get_proc_address: |&str| -> *const c_void) -> Mgl {
        let gl = Gl::load_with(|s| get_proc_address(s));
    	Mgl{gl: gl}
    }

    pub fn set_clear_color(&mut self, color: Color3) {
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
        let version = unsafe {
            CString::new(self.gl.GetString(name) as *const i8, false)
        };
        String::from_str(version.as_str()
            .expect("Can`t convert gl.GetString result to rust string"))
    }

    pub fn set_viewport(&mut self, size: Size2<MInt>) {
        unsafe {
            self.gl.Viewport(0, 0, size.w, size.h);
        }
        self.check();
    }

    pub fn set_uniform_color(&self, color_id: ColorId, color: Color4) {
        unsafe {
            let data_ptr = mem::transmute(&color);
            self.gl.Uniform4fv(color_id.id as MInt, 1, data_ptr);
        }
        self.check();
    }

    pub fn get_uniform(&self, progrma_id: GLuint, name: &str) -> GLuint {
        let id = name.with_c_str(|name| {
            unsafe {
                self.gl.GetUniformLocation(progrma_id, name) as GLuint
            }
        });
        assert!(id != -1);
        self.check();
        id
    }

    /*
    // TODO: replace with something from cgmath-rs
    pub fn tr(&self, m: Matrix4<MFloat>, v: Vector3<MFloat>) -> Matrix4<MFloat> {
        let mut t = Matrix4::<MFloat>::identity();
        t[3][0] = v.x;
        t[3][1] = v.y;
        t[3][2] = v.z;
        m.mul_m(&t)
    }

    pub fn scale(&self, m: Matrix4<MFloat>, scale: MFloat) -> Matrix4<MFloat> {
        let mut t = Matrix4::<MFloat>::identity();
        t[0][0] = scale;
        t[1][1] = scale;
        t[2][2] = scale;
        m.mul_m(&t)
    }

    pub fn rot_x(&self, m: Matrix4<MFloat>, angle: MFloat) -> Matrix4<MFloat> {
        let rad = rad(deg_to_rad(angle));
        let r = Matrix3::from_angle_x(rad).to_matrix4();
        m.mul_m(&r)
    }

    pub fn rot_z(&self, m: Matrix4<MFloat>, angle: MFloat) -> Matrix4<MFloat> {
        let rad = rad(deg_to_rad(angle));
        let r = Matrix3::from_angle_z(rad).to_matrix4();
        m.mul_m(&r)
    }
    */

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
}

/*
pub enum MeshRenderMode {
    Triangles,
    Lines,
}

impl MeshRenderMode {
    fn to_gl_type(&self) -> GLuint {
        match *self {
            MeshRenderMode::Triangles => gl::TRIANGLES,
            MeshRenderMode::Lines => gl::LINES,
        }
    }
}
*/

/*
pub fn init_opengl() {
    verify!(gl::Enable(gl::DEPTH_TEST));
    verify!(gl::Enable(gl::BLEND));
    verify!(gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA));
}
*/

/*
pub struct Vao {
    id: GLuint,
}

impl Vao {
    pub fn new(mgl: &Mgl) -> Vao {
        let mut id = 0;
        unsafe {
            mgl.gl.GenVertexArrays(1, &mut id);
        }
        mgl.check();
        let vao = Vao{id: id};
        vao.bind();
        vao
    }

    pub fn bind(&self) {
        gl.BindVertexArray(self.id);
        mgl.check();
    }

    pub fn unbind(&self) {
        gl.BindVertexArray(0);
        mgl.check();
    }

    pub fn draw_array(&self, mesh_mode: MeshRenderMode, faces_count: MInt) {
        let starting_index = 0;
        let vertices_count = faces_count * 3;
        let mode = mesh_mode.to_gl_type();
        gl.DrawArrays(mode, starting_index, vertices_count);
        mgl.check();
    }
}

impl Drop for Vao {
    fn drop(&mut self) {
        unsafe {
            gl.DeleteVertexArrays(1, &self.id);
        }
        mgl.check();
    }
}
*/

/*
pub struct Vbo {
    id: GLuint,
}

fn get_new_vbo_id() -> GLuint {
    let mut id = 0;
    unsafe {
        verify!(gl::GenBuffers(1, &mut id));
    }
    id
}

impl Vbo {
    pub fn from_data<T>(data: &[T]) -> Vbo {
        let vbo = Vbo{id: get_new_vbo_id()};
        vbo.bind();
        let size = std::mem::size_of::<T>();
        let buf_size = (data.len() * size) as GLsizeiptr;
        if data.len() != 0 {
            unsafe {
                let data_ptr = std::mem::transmute(&data[0]);
                verify!(gl::BufferData(
                    gl::ARRAY_BUFFER,
                    buf_size,
                    data_ptr,
                    gl::STATIC_DRAW,
                ));
            }
        }
        vbo
    }

    pub fn bind(&self) {
        verify!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id));
    }
}

impl Drop for Vbo {
    fn drop(&mut self) {
        unsafe {
            verify!(gl::DeleteBuffers(1, &self.id));
        }
    }
}
*/

/*
pub fn read_pixel_bytes(
    win_size: Size2<MInt>,
    mouse_pos: ScreenPos,
) -> (MInt, MInt, MInt, MInt) {
    let height = win_size.h;
    let reverted_h = height - mouse_pos.v.y;
    let data: [u8, ..4] = [0, 0, 0, 0]; // mut
    unsafe {
        let data_ptr = std::mem::transmute(&data[0]);
        verify!(gl::ReadPixels(
            mouse_pos.v.x, reverted_h, 1, 1,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            data_ptr
        ));
    }
    (data[0] as MInt, data[1] as MInt, data[2] as MInt, data[3] as MInt)
}

pub fn get_2d_screen_matrix(win_size: Size2<MInt>) -> Matrix4<MFloat> {
    let left = 0.0;
    let right = win_size.w as MFloat;
    let bottom = 0.0;
    let top = win_size.h as MFloat;
    let near = -1.0;
    let far = 1.0;
    ortho(left, right, bottom, top, near, far)
}
*/

pub fn compile_shader(gl: &Gl, src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl.CreateShader(ty);
        src.with_c_str(|ptr| gl.ShaderSource(shader, 1, &ptr, ptr::null()));
        gl.CompileShader(shader);
        {
            let mut status = gl::FALSE as GLint;
            gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
            if status != gl::TRUE as GLint {
                let mut len = 0;
                gl.GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                // subtract 1 to skip the trailing null character
                let mut buf = Vec::from_elem(len as uint - 1, 0u8);
                gl.GetShaderInfoLog(
                    shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
                panic!("{}", str::from_utf8(buf.as_slice())
                    .expect("ShaderInfoLog not valid utf8"));
            }
        }
    }
    shader
}

pub fn link_program(gl: &Gl, vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl.CreateProgram();
        gl.AttachShader(program, vs);
        gl.AttachShader(program, fs);
        gl.LinkProgram(program);
        {
            let mut status = gl::FALSE as GLint;
            gl.GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status != gl::TRUE as GLint {
                let mut len: GLint = 0;
                gl.GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                // subtract 1 to skip the trailing null character
                let mut buf = Vec::from_elem(len as uint - 1, 0u8);
                gl.GetProgramInfoLog(
                    program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
                panic!("{}", str::from_utf8(buf.as_slice())
                    .expect("ProgramInfoLog not valid utf8"));
            }
        }
        program
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
