// See LICENSE file for copyright and license details.

use std::f32::consts::{PI, FRAC_PI_2};
use std::num::{Float};
use cgmath::{Vector3, Vector, Deg, Angle, deg};
use common::types::{ZInt, ZFloat, MapPos};
use zgl::types::{VertexCoord, WorldPos};
use core::geom;

pub use core::geom::{HEX_IN_RADIUS, HEX_EX_RADIUS};

pub const MINIMAL_LIFT_HEIGHT: ZFloat = 0.01;

pub fn map_pos_to_world_pos(i: &MapPos) -> WorldPos {
    WorldPos{v: geom::map_pos_to_world_pos(i).extend(0.0)}
}

pub fn lift(v: Vector3<ZFloat>) -> Vector3<ZFloat> {
    let mut v = v;
    v.z += MINIMAL_LIFT_HEIGHT;
    v
}

pub fn index_to_circle_vertex(count: ZInt, i: ZInt) -> VertexCoord {
    let n = FRAC_PI_2 + 2.0 * PI * (i as ZFloat) / (count as ZFloat);
    VertexCoord {
        v: Vector3 {
            x: n.cos(),
            y: n.sin(),
            z: 0.0
        }.mul_s(HEX_EX_RADIUS)
    }
}

pub fn index_to_hex_vertex(i: ZInt) -> VertexCoord {
    index_to_circle_vertex(6, i)
}

pub fn index_to_hex_vertex_s(scale: ZFloat, i: ZInt) -> VertexCoord {
    let v = index_to_hex_vertex(i).v.mul_s(scale);
    VertexCoord{v: v}
}

// TODO: ZFloat -> WorldDistance
pub fn dist(a: &WorldPos, b: &WorldPos) -> ZFloat {
    let dx = (b.v.x - a.v.x).abs();
    let dy = (b.v.y - a.v.y).abs();
    let dz = (b.v.z - a.v.z).abs();
    ((dx.powi(2) + dy.powi(2) + dz.powi(2)) as ZFloat).sqrt()
}

pub fn get_rot_angle(a: &WorldPos, b: &WorldPos) -> Deg<ZFloat> {
    let diff = b.v - a.v;
    let angle = Float::atan2(diff.x, diff.y).to_degrees();
    let angle = -(180.0 + angle); // TODO: remove this
    deg(angle)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
