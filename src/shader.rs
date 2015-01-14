// See LICENSE file for copyright and license details.

use std::iter;
use std::mem;
use std::ptr;
use std::ffi::CString;
use cgmath::{Matrix4};
use gl;
use gl::types::{GLuint, GLint, GLenum, GLchar};
use zgl::{Zgl};
use core_types::{ZInt};
use visualizer_types::{ZFloat, Color4, ColorId, MatId, AttrId, ProgramId};

fn get_attr_location(program_id: &ProgramId, zgl: &Zgl, name: &str) -> AttrId {
    let name_c = CString::from_slice(name.as_bytes()).as_slice_with_nul().as_ptr();
    let attr_id = unsafe {
        zgl.gl.GetAttribLocation(program_id.id, name_c)
    };
    zgl.check();
    assert!(attr_id >= 0);
    AttrId{id: attr_id as GLuint}
}

fn get_uniform(program_id: &ProgramId, zgl: &Zgl, name: &str) -> GLuint {
    let name_c = CString::from_slice(name.as_bytes())
        .as_slice_with_nul().as_ptr();
    let id = unsafe {
        zgl.gl.GetUniformLocation(program_id.id, name_c) as GLuint
    };
    assert!(id != -1);
    zgl.check();
    id
}

pub struct Shader {
    program_id: ProgramId,
    mvp_uniform_location: MatId,
    position_attr_id: AttrId,
    color_attr_id: Option<AttrId>,
    texture_coords_attr_id: Option<AttrId>,
}

impl Shader {
    pub fn new(zgl: &Zgl, vs_src: &str, fs_src: &str) -> Shader {
        let vs = compile_shader(zgl, vs_src, gl::VERTEX_SHADER);
        let fs = compile_shader(zgl, fs_src, gl::FRAGMENT_SHADER);
        let program_id = link_program(zgl, vs, fs);
        let mvp_uniform_location = MatId {
            id: get_uniform(&program_id, zgl, "mvp_mat"),
        };
        let position_attr_id = get_attr_location(
            &program_id, zgl, "position");
        zgl.enable_vertex_attrib_array(&position_attr_id);
        Shader {
            program_id: program_id,
            mvp_uniform_location: mvp_uniform_location,
            position_attr_id: position_attr_id,
            color_attr_id: None,
            texture_coords_attr_id: None,
        }
    }

    // TODO: convert to Builder pattern (see WindowBuilder in glutin)
    pub fn enable_color(&mut self, zgl: &Zgl) {
        let color_attr_id = get_attr_location(
            &self.program_id, zgl, "a_color");
        zgl.enable_vertex_attrib_array(&color_attr_id);
        self.color_attr_id = Some(color_attr_id);
    }

    // TODO: Rename
    pub fn enable_texture_coords(&mut self, zgl: &Zgl) {
        let texture_coords_attr_id = get_attr_location(
            &self.program_id, zgl, "in_texture_coordinates");
        zgl.enable_vertex_attrib_array(&texture_coords_attr_id);
        self.texture_coords_attr_id = Some(texture_coords_attr_id);
    }

    pub fn enable_attr(&self, zgl: &Zgl, attr_id: &AttrId, components_count: ZInt) {
        let is_normalized = gl::FALSE;
        let stride = 0;
        unsafe {
            zgl.gl.VertexAttribPointer(
                attr_id.id,
                components_count,
                gl::FLOAT,
                is_normalized,
                stride,
                ptr::null_mut(),
            );
        }
        zgl.check();
    }

    pub fn activate(&self, zgl: &Zgl) {
        unsafe {
            assert!(zgl.gl.IsProgram(self.program_id.id) != gl::FALSE);
            zgl.gl.UseProgram(self.program_id.id);
        }
        zgl.check();
    }

    pub fn prepare_pos(&self, zgl: &Zgl) {
        self.enable_attr(zgl, &self.position_attr_id, 3);
    }

    pub fn prepare_color(&self, zgl: &Zgl) {
        let attr_id = self.color_attr_id.as_ref().expect("Can`t get color vbo");
        self.enable_attr(zgl, attr_id, 3);
    }

    pub fn prepare_texture_coords(&self, zgl: &Zgl) {
        let attr_id = self.texture_coords_attr_id.as_ref()
            .expect("Can`t get texture coords vbo");
        self.enable_attr(zgl, attr_id, 2);
    }

    pub fn set_uniform_mat4f(&self, zgl: &Zgl, mat_id: &MatId, mat: &Matrix4<ZFloat>) {
        let count = 1;
        let transpose = gl::FALSE;
        unsafe {
            let data_ptr = mem::transmute(mat);
            zgl.gl.UniformMatrix4fv(
                mat_id.id as ZInt, count, transpose, data_ptr);
        }
        zgl.check();
    }

    pub fn set_uniform_color(&self, zgl: &Zgl, color_id: &ColorId, color: &Color4) {
        unsafe {
            let data_ptr = mem::transmute(color);
            zgl.gl.Uniform4fv(color_id.id as ZInt, 1, data_ptr);
        }
        zgl.check();
    }

    pub fn get_uniform_color(&self, zgl: &Zgl, name: &str) -> ColorId {
        let id = get_uniform(&self.program_id, zgl, name);
        ColorId{id: id}
    }

    // TODO: GLint -> ...
    pub fn get_uniform_texture(&self, zgl: &Zgl, name: &str) -> GLint {
        let id = get_uniform(&self.program_id, zgl, name);
        id as GLint
    }

    pub fn get_mvp_mat(&self) -> &MatId {
        &self.mvp_uniform_location
    }
}

fn compile_shader(zgl: &Zgl, src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = zgl.gl.CreateShader(ty);
        zgl.check();
        let src_c = CString::from_slice(src.as_bytes())
            .as_slice_with_nul().as_ptr();
        zgl.gl.ShaderSource(shader, 1, &src_c, ptr::null());
        zgl.check();
        zgl.gl.CompileShader(shader);
        zgl.check();
        let mut status = gl::FALSE as GLint;
        zgl.gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let mut len = 0;
            zgl.gl.GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut err_log = String::with_capacity(len as usize);
            err_log.extend(iter::repeat('\0').take(len as usize));
            let raw_ptr = err_log.as_slice().as_ptr() as *mut GLchar;
            zgl.gl.GetShaderInfoLog(shader, len, &mut len, raw_ptr);
            err_log.truncate(len as usize);
            panic!("{}", err_log);
        }
    }
    shader
}

fn link_program(zgl: &Zgl, vs: GLuint, fs: GLuint) -> ProgramId {
    unsafe {
        let program_id = zgl.gl.CreateProgram();
        zgl.check();
        zgl.gl.AttachShader(program_id, vs);
        zgl.check();
        zgl.gl.AttachShader(program_id, fs);
        zgl.check();
        zgl.gl.LinkProgram(program_id);
        zgl.check();
        zgl.gl.DeleteShader(vs);
        zgl.check();
        zgl.gl.DeleteShader(fs);
        zgl.check();
        zgl.gl.UseProgram(program_id);
        zgl.check();
        // zgl.gl.DeleteProgram(program_id); // mark for deletion // TODO: d-tor?
        zgl.check();
        let mut status = gl::FALSE as GLint;
        zgl.gl.GetProgramiv(program_id, gl::LINK_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let mut len = 0;
            zgl.gl.GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            let mut err_log = String::with_capacity(len as usize);
            err_log.extend(iter::repeat('\0').take(len as usize));
            let raw_ptr = err_log.as_slice().as_ptr() as *mut GLchar;
            zgl.gl.GetProgramInfoLog(program_id, len, &mut len, raw_ptr);
            err_log.truncate(len as usize);
            panic!("{}", err_log);
        }
        assert!(zgl.gl.IsProgram(program_id) != gl::FALSE);
        ProgramId{id: program_id}
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
