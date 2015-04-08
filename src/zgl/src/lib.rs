// See LICENSE file for copyright and license details.

#![feature(std_misc, str_words, str_char)] // TODO

extern crate rand;
extern crate libc;
extern crate time;
extern crate cgmath;
extern crate zoc_gl as gl;
extern crate image;
extern crate stb_tt;
extern crate common;

pub mod types;
pub mod shader;
pub mod texture;
pub mod zgl;
pub mod mesh;
pub mod camera;
pub mod font_stash;
pub mod obj;
pub mod misc;

pub use types::{
    Color3,
    Color4,
    VertexCoord,
    Normal,
    TextureCoord,
    ScreenPos,
    Time,
    MatId,
    ColorId,
    AttrId,
    ProgramId,
};

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
