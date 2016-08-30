use cgmath::{Vector3, Vector2};
use time;

pub use core::types::{Size2};

#[derive(Copy, Clone, Debug)]
pub struct WorldPos{pub v: Vector3<f32>}

#[derive(Copy, Clone, Debug)]
pub struct VertexCoord{pub v: Vector3<f32>}

#[derive(Copy, Clone, Debug)]
pub struct ScreenPos{pub v: Vector2<i32>}

#[derive(Copy, Clone, Debug)]
pub struct Time{pub n: f64}

impl Time {
    pub fn now() -> Time {
        Time{n: time::precise_time_ns() as f64 / 1_000_000_000.0}
    }
}
