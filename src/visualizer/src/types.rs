use cgmath::{Vector3, Vector2};

pub use core::types::{Size2};

#[derive(Copy, Clone)]
pub struct WorldPos{pub v: Vector3<f32>}

#[derive(Copy, Clone)]
pub struct VertexCoord{pub v: Vector3<f32>}

#[derive(Copy, Clone)]
pub struct ScreenPos{pub v: Vector2<i32>}

#[derive(Copy, Clone)]
pub struct Time{pub n: u64}
