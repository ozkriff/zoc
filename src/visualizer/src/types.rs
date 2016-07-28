// See LICENSE file for copyright and license details.

use cgmath::{Vector3, Vector2};

pub use core::types::{ZInt, ZFloat, Size2};

#[derive(Copy, Clone)]
pub struct WorldPos{pub v: Vector3<ZFloat>}

#[derive(Copy, Clone)]
pub struct VertexCoord{pub v: Vector3<ZFloat>}

#[derive(Copy, Clone)]
pub struct ScreenPos{pub v: Vector2<ZInt>}

#[derive(Copy, Clone)]
pub struct Time{pub n: u64}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
