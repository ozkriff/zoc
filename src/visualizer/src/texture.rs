// See LICENSE file for copyright and license details.

use std::io::{Cursor};
use image;
use gfx::handle::{ShaderResourceView};
use gfx::{self, tex};
use gfx_gl;
use pipeline::{ColorFormat};

pub type Texture = gfx::handle::ShaderResourceView<gfx_gl::Resources, [f32; 4]>;

pub fn load_texture<R, F>(factory: &mut F, data: &[u8]) -> ShaderResourceView<R, [f32; 4]>
    where R: gfx::Resources, F: gfx::Factory<R>
{
    let img = image::load(Cursor::new(data), image::PNG).unwrap().to_rgba();
    let (width, height) = img.dimensions();
    let kind = tex::Kind::D2(width as tex::Size, height as tex::Size, tex::AaMode::Single);
    let t = &img.into_vec();
    let (_, view) = factory.create_texture_const_u8::<ColorFormat>(kind, &[t]).unwrap();
    view
}

// TODO: w, h: u16 -> size: Size2
pub fn load_texture_raw<R, F>(factory: &mut F, w: u16, h: u16, data: &[u8]) -> ShaderResourceView<R, [f32; 4]>
    where R: gfx::Resources, F: gfx::Factory<R>
{
    let kind = tex::Kind::D2(w as tex::Size, h as tex::Size, tex::AaMode::Single);
    let (_, view) = factory.create_texture_const_u8::<ColorFormat>(kind, &[data]).unwrap();
    view
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
