// See LICENSE file for copyright and license details.

use gl::types::{GLfloat, GLuint};
use core::types::{ZInt};
use cgmath::{Vector3, Vector2};

#[derive(Clone)]
pub struct Color3 {
    pub r: ZFloat,
    pub g: ZFloat,
    pub b: ZFloat,
}

#[derive(Clone)]
pub struct Color4 {
    pub r: ZFloat,
    pub g: ZFloat,
    pub b: ZFloat,
    pub a: ZFloat,
}

pub type ZFloat = GLfloat;

#[derive(Clone)]
pub struct VertexCoord{pub v: Vector3<ZFloat>}

#[derive(Clone)]
pub struct Normal{pub v: Vector3<ZFloat>}

#[derive(Clone)]
pub struct TextureCoord{pub v: Vector2<ZFloat>}

#[derive(Clone)]
pub struct WorldPos{pub v: Vector3<ZFloat>}

#[derive(Clone)]
pub struct ScreenPos{pub v: Vector2<ZInt>}

/*
pub struct Time{pub n: u64}
*/

#[derive(Clone)]
pub struct MatId{pub id: GLuint}

#[derive(Clone)]
pub struct ColorId{pub id: GLuint}

#[derive(Clone)]
pub struct AttrId{pub id: GLuint}

/// Stores result of glLinkProgram
#[derive(Clone)]
pub struct ProgramId{pub id: GLuint}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
