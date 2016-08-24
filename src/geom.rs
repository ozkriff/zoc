use std::f32::consts::{PI};
use cgmath::{Vector3, Rad, Angle, rad};
use core::{ExactPos, MapPos, SlotId, geom};
use core::dir::{Dir};
use types::{VertexCoord, WorldPos};

pub use core::geom::{HEX_IN_RADIUS, HEX_EX_RADIUS};

pub const MIN_LIFT_HEIGHT: f32 = 0.01;

pub fn map_pos_to_world_pos(p: MapPos) -> WorldPos {
    let v = geom::map_pos_to_world_pos(p).extend(0.0);
    WorldPos{v: v}
}

pub fn exact_pos_to_world_pos(p: ExactPos) -> WorldPos {
    let v = geom::map_pos_to_world_pos(p.map_pos).extend(0.0);
    match p.slot_id {
        SlotId::TwoTiles(dir) => {
            // TODO: employ index_to_circle_vertex_rnd
            let p2 = Dir::get_neighbour_pos(p.map_pos, dir);
            let v2 = geom::map_pos_to_world_pos(p2).extend(0.0);
            WorldPos{v: (v + v2) / 2.0}
        }
        SlotId::WholeTile => {
            WorldPos{v: v + index_to_circle_vertex_rnd(3, 0, p.map_pos).v * 0.2}
        }
        SlotId::Id(n) => {
            WorldPos{v: v + index_to_circle_vertex_rnd(3, n as i32, p.map_pos).v * 0.5}
        }
    }
}

pub fn lift(v: Vector3<f32>) -> Vector3<f32> {
    let mut v = v;
    v.z += MIN_LIFT_HEIGHT;
    v
}

pub fn index_to_circle_vertex_rnd(count: i32, i: i32, pos: MapPos) -> VertexCoord {
    let n = 2.0 * PI * (i as f32) / (count as f32);
    let n = n + ((pos.v.x as f32 + pos.v.y as f32) * 7.0) % 4.0; // TODO: remove magic numbers
    VertexCoord {
        v: Vector3{x: n.cos(), y: n.sin(), z: 0.0} * HEX_EX_RADIUS
    }
}

pub fn index_to_circle_vertex(count: i32, i: i32) -> VertexCoord {
    let n = PI / 2.0 + 2.0 * PI * (i as f32) / (count as f32);
    VertexCoord {
        v: Vector3{x: n.cos(), y: n.sin(), z: 0.0} * HEX_EX_RADIUS
    }
}

pub fn index_to_hex_vertex(i: i32) -> VertexCoord {
    index_to_circle_vertex(6, i)
}

pub fn index_to_hex_vertex_s(scale: f32, i: i32) -> VertexCoord {
    let v = index_to_hex_vertex(i).v * scale;
    VertexCoord{v: v}
}

// TODO: f32 -> WorldDistance
pub fn dist(a: WorldPos, b: WorldPos) -> f32 {
    let dx = (b.v.x - a.v.x).abs();
    let dy = (b.v.y - a.v.y).abs();
    let dz = (b.v.z - a.v.z).abs();
    ((dx.powi(2) + dy.powi(2) + dz.powi(2)) as f32).sqrt()
}

pub fn get_rot_angle(a: WorldPos, b: WorldPos) -> Rad<f32> {
    let diff = b.v - a.v;
    let angle = diff.x.atan2(diff.y);
    rad(-angle).normalize()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{PI};
    use cgmath::{Vector3};
    use ::types::{WorldPos};
    use super::{get_rot_angle, index_to_circle_vertex};

    const EPS: f32 = 0.001;

    #[test]
    fn test_get_rot_angle_30_deg() {
        let count = 12;
        for i in 0 .. count {
            let a = WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}};
            let b = WorldPos{v: index_to_circle_vertex(count, i).v};
            let expected_angle = i as f32 * (PI * 2.0) / (count as f32);
            let angle = get_rot_angle(a, b);
            let diff = (expected_angle - angle.s).abs();
            assert!(diff < EPS, "{} != {}", expected_angle, angle.s);
        }
    }
}
