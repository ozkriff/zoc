use cgmath::{Vector3, Vector2};

pub use core::types::{Size2};

#[derive(Copy, Clone, Debug)]
pub struct WorldPos{pub v: Vector3<f32>}

#[derive(Copy, Clone, Debug)]
pub struct VertexCoord{pub v: Vector3<f32>}

#[derive(Copy, Clone, Debug)]
pub struct ScreenPos{pub v: Vector2<i32>}

#[derive(Copy, Clone, Debug)]
pub struct Time{pub n: f32}

#[derive(Copy, Clone, Debug)]
pub struct Speed{pub n: f32}

#[derive(Copy, Clone, Debug)]
pub struct WorldDistance{pub n: f32}
