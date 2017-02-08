use std::io::{Cursor};
use image;
use gfx::handle::{ShaderResourceView};
use gfx::{self, texture};
use gfx_gl;
use types::{Size2};
use pipeline::{ColorFormat};
use context::{Context};

pub type Texture = gfx::handle::ShaderResourceView<gfx_gl::Resources, [f32; 4]>;

pub fn load_texture(context: &mut Context, data: &[u8]) -> ShaderResourceView<gfx_gl::Resources, [f32; 4]> {
    let img = image::load(Cursor::new(data), image::PNG).unwrap().to_rgba();
    let (w, h) = img.dimensions();
    let size = Size2{w: w as i32, h: h as i32};
    load_texture_raw(context.factory_mut(), size, &img.into_vec())
}

pub fn load_texture_raw<R, F>(factory: &mut F, size: Size2, data: &[u8]) -> ShaderResourceView<R, [f32; 4]>
    where R: gfx::Resources, F: gfx::Factory<R>
{
    let kind = texture::Kind::D2(size.w as texture::Size, size.h as texture::Size, texture::AaMode::Single);
    let (_, view) = factory.create_texture_immutable_u8::<ColorFormat>(kind, &[data]).unwrap();
    view
}
