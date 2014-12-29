// See LICENSE file for copyright and license details.

use gl::types::{GLfloat, GLuint};
use core_types::{MInt};
use cgmath::{Vector3, Vector2};

#[deriving(Copy)]
pub struct Color3 {
    pub r: MFloat,
    pub g: MFloat,
    pub b: MFloat,
}

#[deriving(Copy)]
pub struct Color4 {
    pub r: MFloat,
    pub g: MFloat,
    pub b: MFloat,
    pub a: MFloat,
}

pub type MFloat = GLfloat;

/*
#[deriving(Clone)]
pub struct VertexCoord{pub v: Vector3<MFloat>}

#[deriving(Clone)]
pub struct Normal{pub v: Vector3<MFloat>}

#[deriving(Clone)]
pub struct TextureCoord{pub v: Vector2<MFloat>}
*/

#[deriving(Clone)]
pub struct WorldPos{pub v: Vector3<MFloat>}

#[deriving(Clone)]
pub struct ScreenPos{pub v: Vector2<MInt>}

/*
pub struct Time{pub n: u64}
*/

#[deriving(Copy)]
pub struct MatId{pub id: GLuint}

#[deriving(Copy)]
pub struct ColorId{pub id: GLuint}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
