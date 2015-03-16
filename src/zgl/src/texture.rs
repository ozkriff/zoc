// See LICENSE file for copyright and license details.

use std::iter::{repeat};
use std::mem;
use std::path::{Path};
use image;
use image::{GenericImage};
use gl;
use gl::types::{GLint, GLuint, GLsizei};
use cgmath::{Vector2};
use shader::{Shader};
use common::types::{Size2, ZInt};
use zgl::{Zgl};
use common::{fs};

#[derive(Clone)]
pub struct Texture {
    id: GLuint,
}

impl Texture {
    pub fn new(zgl: &Zgl, path: &Path) -> Texture {
        load_texture(zgl, path)
    }

    pub fn new_empty(zgl: &Zgl, size: Size2<ZInt>) -> Texture {
        get_empty_texture(zgl, size)
    }

    pub fn enable(&self, zgl: &Zgl, shader: &Shader) {
        let basic_texture_loc = shader.get_uniform_texture(zgl, "basic_texture");
        unsafe {
            zgl.gl.Uniform1i(basic_texture_loc, 0);
        }
        zgl.check();
        self.bind(zgl);
    }

    pub fn bind(&self, zgl: &Zgl) {
        unsafe {
            zgl.gl.ActiveTexture(gl::TEXTURE0);
        }
        zgl.check();
        unsafe {
            zgl.gl.BindTexture(gl::TEXTURE_2D, self.id);
        }
        zgl.check();
    }

    pub fn set_sub_image(
        &self,
        zgl: &Zgl,
        pos: Vector2<ZInt>,
        size: Size2<ZInt>,
        data: &Vec<u8>
    ) {
        let bytes_per_pixel = 4;
        let expected_data_length = size.w * size.h * bytes_per_pixel;
        assert_eq!(data.len(), expected_data_length as usize);
        let format = gl::RGBA;
        let level = 0;
        unsafe {
            zgl.gl.TexSubImage2D(
                gl::TEXTURE_2D,
                level,
                pos.x,
                pos.y,
                size.w,
                size.h,
                format,
                gl::UNSIGNED_BYTE,
                mem::transmute(&data[0]),
            );
            zgl.check();
        }
    }
}

#[allow(deprecated)] // TODO: remove
fn load_image(path: &Path) -> image::DynamicImage {
    use std::old_io::{MemReader};

    let buf = fs::load(path);
    let mem_reader = MemReader::new(buf.into_inner());
    image::load(mem_reader, image::ImageFormat::PNG)
        .ok().expect("Can`t open img")
}

fn get_empty_texture(zgl: &Zgl, size: Size2<ZInt>) -> Texture {
    let s = size.w;
    assert_eq!(size.w, size.h);
    let data: Vec<_> = repeat(0u8).take((s * s) as usize * 4 ).collect();
    let mut id = 0;
    unsafe {
        zgl.gl.GenTextures(1, &mut id);
        zgl.check();
    };
    unsafe {
        zgl.gl.ActiveTexture(gl::TEXTURE0);
    }
    zgl.check();
    unsafe {
        zgl.gl.BindTexture(gl::TEXTURE_2D, id);
    }
    zgl.check();
    let format = gl::RGBA;
    unsafe {
        let level = 0;
        let border = 0;
        zgl.gl.TexImage2D(
            gl::TEXTURE_2D,
            level,
            format as GLint,
            s,
            s,
            border,
            format,
            gl::UNSIGNED_BYTE,
            mem::transmute(&data[0]),
        );
        zgl.check();
    }
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
    }
    zgl.check();
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
    }
    zgl.check();
    Texture{id: id}
}

fn load_texture(zgl: &Zgl, path: &Path) -> Texture {
    let img = load_image(path);
    let mut id = 0;
    unsafe {
        zgl.gl.GenTextures(1, &mut id);
    };
    zgl.check();
    unsafe {
        zgl.gl.ActiveTexture(gl::TEXTURE0);
    }
    zgl.check();
    unsafe {
        zgl.gl.BindTexture(gl::TEXTURE_2D, id);
    }
    zgl.check();
    let format = match img.color() {
        image::RGBA(_) => gl::RGBA,
        image::RGB(_) => gl::RGB,
        _ => panic!("Bad image format")
    };
    let (w, h) = img.dimensions();
    let pixels = img.raw_pixels();
    unsafe {
        let level = 0;
        let border = 0;
        zgl.gl.TexImage2D(
            gl::TEXTURE_2D,
            level,
            format as GLint,
            w as GLsizei,
            h as GLsizei,
            border,
            format,
            gl::UNSIGNED_BYTE,
            mem::transmute(&pixels[0]),
        );
        zgl.check();
    }
    // TODO: zgl wrappers
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
    }
    zgl.check();
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
    }
    zgl.check();
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
    }
    zgl.check();
    unsafe {
        zgl.gl.TexParameteri(gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
    }
    zgl.check();
    Texture{id: id}
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
