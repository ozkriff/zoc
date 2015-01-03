// See LICENSE file for copyright and license details.

use std::mem;
use std::ptr;
use std::str;
use cgmath::{Matrix4};
use gl;
use gl::types::{GLuint, GLint, GLenum, GLchar};
use zgl::{Zgl};
use core_types::{ZInt};
use visualizer_types::{ZFloat, Color4, ColorId, MatId};

pub struct Shader {
    id: GLuint,
}

impl Shader {
    pub fn new(zgl: &Zgl, vs_src: &str, fs_src: &str) -> Shader {
        let vs = compile_shader(zgl, vs_src, gl::VERTEX_SHADER);
        let fs = compile_shader(zgl, fs_src, gl::FRAGMENT_SHADER);
        let program_id = link_program(zgl, vs, fs);
        Shader{id: program_id}
    }

    pub fn activate(&self, zgl: &Zgl) {
        unsafe {
            zgl.gl.UseProgram(self.id);
        }
        zgl.check();
    }

    pub fn set_uniform_mat4f(&self, zgl: &Zgl, mat_id: &MatId, mat: &Matrix4<ZFloat>) {
        unsafe {
            let data_ptr = mem::transmute(mat);
            // TODO: give name to magic parameters
            zgl.gl.UniformMatrix4fv(mat_id.id as ZInt, 1, gl::FALSE, data_ptr);
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
        let id = self.get_uniform(zgl, name);
        ColorId{id: id as GLuint}
    }

    pub fn get_uniform_mat(&self, zgl: &Zgl, name: &str) -> MatId {
        let id = self.get_uniform(zgl, name);
        MatId{id: id as GLuint}
    }

    fn get_uniform(&self, zgl: &Zgl, name: &str) -> GLuint {
        let id = name.with_c_str(|name| {
            unsafe {
                zgl.gl.GetUniformLocation(self.id, name) as GLuint
            }
        });
        assert!(id != -1);
        zgl.check();
        id
    }
}

fn compile_shader(zgl: &Zgl, src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = zgl.gl.CreateShader(ty);
        zgl.check();
        src.with_c_str(|ptr| zgl.gl.ShaderSource(shader, 1, &ptr, ptr::null()));
        zgl.check();
        zgl.gl.CompileShader(shader);
        zgl.check();
        let mut status = gl::FALSE as GLint;
        zgl.gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let mut len = 0;
            zgl.gl.GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            // subtract 1 to skip the trailing null character
            let mut buf = Vec::from_elem(len as uint - 1, 0u8);
            zgl.gl.GetShaderInfoLog(
                shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", str::from_utf8(buf.as_slice())
                .ok().expect("ShaderInfoLog not valid utf8"));
        }
    }
    shader
}

fn link_program(zgl: &Zgl, vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = zgl.gl.CreateProgram();
        zgl.check();
        zgl.gl.AttachShader(program, vs);
        zgl.check();
        zgl.gl.AttachShader(program, fs);
        zgl.check();
        zgl.gl.LinkProgram(program);
        zgl.check();
        zgl.gl.DeleteShader(vs);
        zgl.check();
        zgl.gl.DeleteShader(fs);
        zgl.check();
        zgl.gl.UseProgram(program);
        zgl.check();
        zgl.gl.DeleteProgram(program); // mark for deletion
        zgl.check();
        let mut status = gl::FALSE as GLint;
        zgl.gl.GetProgramiv(program, gl::LINK_STATUS, &mut status);
        if status != gl::TRUE as GLint {
            let mut len: GLint = 0;
            zgl.gl.GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            // subtract 1 to skip the trailing null character
            let mut buf = Vec::from_elem(len as uint - 1, 0u8);
            zgl.gl.GetProgramInfoLog(
                program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", str::from_utf8(buf.as_slice())
                .ok().expect("ProgramInfoLog not valid utf8"));
        }
        program
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
