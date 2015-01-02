// See LICENSE file for copyright and license details.

use std::f32::consts::{PI, FRAC_PI_2};
use std::num::{/*Int, Float,*/ FloatMath};
use cgmath::{Vector2, Vector3, Vector};
use core_types::{ZInt, MapPos};
// use core_misc::{rad_to_deg};
use visualizer_types::{WorldPos, ZFloat, VertexCoord};

pub const HEX_EX_RADIUS: ZFloat = 1.0;

// (pow(1.0, 2) - pow(0.5, 2)).sqrt()
pub const HEX_IN_RADIUS: ZFloat = 0.866025403784 * HEX_EX_RADIUS;

/*
pub const MINIMAL_LIFT_HEIGHT: ZFloat = 0.01;

pub fn lift(v: Vector3<ZFloat>) -> Vector3<ZFloat> {
    let mut v = v;
    v.z += MINIMAL_LIFT_HEIGHT;
    v
}
*/

pub fn map_pos_to_world_pos(i: MapPos) -> WorldPos {
    let v = Vector2 {
        x: (i.v.x as ZFloat) * HEX_IN_RADIUS * 2.0,
        y: (i.v.y as ZFloat) * HEX_EX_RADIUS * 1.5,
    };
    WorldPos {
        v: if i.v.y % 2 == 0 {
            Vector3{x: v.x + HEX_IN_RADIUS, y: v.y, z: 0.0}
        } else {
            v.extend(0.0)
        }
    }
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

/*
pub fn index_to_hex_vertex_s(scale: ZFloat, i: ZInt) -> VertexCoord {
    let v = index_to_hex_vertex(i).v.mul_s(scale);
    VertexCoord{v: v}
}

pub fn dist(a: &WorldPos, b: &WorldPos) -> ZFloat {
    let dx = (b.v.x - a.v.x).abs() as ZInt;
    let dy = (b.v.y - a.v.y).abs() as ZInt;
    let dz = (b.v.z - a.v.z).abs() as ZInt;
    ((dx.pow(2) + dy.pow(2) + dz.pow(2)) as ZFloat).sqrt()
}

pub fn get_rot_angle(a: WorldPos, b: WorldPos) -> ZFloat {
    let mut angle = rad_to_deg(((b.v.x - a.v.x) / dist(&a, &b)).asin());
    if b.v.y - a.v.y > 0.0 {
        angle = -(180.0 + angle);
    }
    angle
}
*/

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
