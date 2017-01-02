use std::f32::consts::{PI};
use cgmath::{Vector3, Vector2, Rad, Angle, InnerSpace};
use core::{ExactPos, MapPos, SlotId, Distance, geom, get_slots_count};
use core::dir::{Dir};
use core::game_state::{State};
use core::map::{spiral_iter};
use types::{VertexCoord, WorldPos, WorldDistance};

pub use core::geom::{HEX_IN_RADIUS, HEX_EX_RADIUS};

pub const MIN_LIFT_HEIGHT: f32 = 0.01;

pub fn vec3_z(z: f32) -> Vector3<f32> {
    Vector3{x: 0.0, y: 0.0, z: z}
}

pub fn world_pos_to_map_pos(pos: WorldPos) -> MapPos {
    let origin = MapPos{v: Vector2 {
        x: (pos.v.x / (HEX_IN_RADIUS * 2.0)) as i32,
        y: (pos.v.y / (HEX_EX_RADIUS * 1.5)) as i32,
    }};
    let origin_world_pos = map_pos_to_world_pos(origin);
    let mut closest_map_pos = origin;
    let mut min_dist = (origin_world_pos.v - pos.v).magnitude();
    for map_pos in spiral_iter(origin, Distance{n: 1}) {
        let world_pos = map_pos_to_world_pos(map_pos);
        let d = (world_pos.v - pos.v).magnitude();
        if d < min_dist {
            min_dist = d;
            closest_map_pos = map_pos;
        }
    }
    closest_map_pos
}

pub fn map_pos_to_world_pos(p: MapPos) -> WorldPos {
    let v = geom::map_pos_to_world_pos(p).extend(0.0);
    WorldPos{v: v}
}

pub fn exact_pos_to_world_pos(state: &State, p: ExactPos) -> WorldPos {
    let v = geom::map_pos_to_world_pos(p.map_pos).extend(0.0);
    let n = get_slots_count(state.map(), p.map_pos);
    match p.slot_id {
        SlotId::TwoTiles(dir) => {
            // TODO: employ index_to_circle_vertex_rnd
            let p2 = Dir::get_neighbour_pos(p.map_pos, dir);
            let v2 = geom::map_pos_to_world_pos(p2).extend(0.0);
            WorldPos{v: (v + v2) / 2.0}
        }
        SlotId::WholeTile => {
            WorldPos{v: v + index_to_circle_vertex_rnd(n, 0, p.map_pos).v * 0.2}
        }
        SlotId::Air => {
            let v = v + vec3_z(2.0);
            WorldPos{v: v + index_to_circle_vertex_rnd(n, 0, p.map_pos).v * 0.2} // TODO
        }
        SlotId::Id(id) => {
            WorldPos{v: v + index_to_circle_vertex_rnd(n, id as i32, p.map_pos).v * 0.5}
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
    let v = Vector3{x: n.cos(), y: n.sin(), z: 0.0};
    VertexCoord{v: v * if count == 1 { 0.1 } else { HEX_EX_RADIUS }}
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

pub fn dist(a: WorldPos, b: WorldPos) -> WorldDistance {
    let dx = (b.v.x - a.v.x).abs();
    let dy = (b.v.y - a.v.y).abs();
    let dz = (b.v.z - a.v.z).abs();
    WorldDistance{n: ((dx.powi(2) + dy.powi(2) + dz.powi(2)) as f32).sqrt()}
}

pub fn get_rot_angle(a: WorldPos, b: WorldPos) -> Rad<f32> {
    let diff = b.v - a.v;
    let angle = diff.x.atan2(diff.y);
    Rad(-angle).normalize()
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
            let diff = (expected_angle - angle.0).abs();
            assert!(diff < EPS, "{} != {}", expected_angle, angle.0);
        }
    }
}
