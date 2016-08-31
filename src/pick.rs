use cgmath::{self, InnerSpace, SquareMatrix, EuclideanSpace};
use collision::{Plane, Ray, Intersect};
use core::{MapPos};
use core::partial_state::{PartialState};
use core::map::{spiral_iter};
use core::game_state::{GameState};
use context::{Context};
use geom;
use camera::Camera;
use types::{WorldPos};

pub fn pick_world_pos(context: &Context, camera: &Camera) -> WorldPos {
    let im = camera.mat().invert()
        .expect("Can`t invert camera matrix");
    let w = context.win_size.w as f32;
    let h = context.win_size.h as f32;
    let x = context.mouse().pos.v.x as f32;
    let y = context.mouse().pos.v.y as f32;
    let x = (2.0 * x) / w - 1.0;
    let y = 1.0 - (2.0 * y) / h;
    let p0_raw = im * cgmath::Vector4{x: x, y: y, z: 0.0, w: 1.0};
    let p0 = (p0_raw / p0_raw.w).truncate();
    let p1_raw = im * cgmath::Vector4{x: x, y: y, z: 1.0, w: 1.0};
    let p1 = (p1_raw / p1_raw.w).truncate();
    let plane = Plane::from_abcd(0.0, 0.0, 1.0, 0.0);
    let ray = Ray::new(cgmath::Point3::from_vec(p0), p1 - p0);
    let intersection_pos = (plane, ray).intersection()
        .expect("Can`t find mouse ray/plane intersection");
    WorldPos{v: intersection_pos.to_vec()}
}

pub fn pick_tile(
    context: &Context,
    state: &PartialState,
    camera: &Camera,
) -> Option<MapPos> {
    let p = pick_world_pos(context, camera);
    let origin = MapPos{v: cgmath::Vector2 {
        x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as i32,
        y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as i32,
    }};
    let origin_world_pos = geom::map_pos_to_world_pos(origin);
    let mut closest_map_pos = origin;
    let mut min_dist = (origin_world_pos.v - p.v).magnitude();
    for map_pos in spiral_iter(origin, 1) {
        let pos = geom::map_pos_to_world_pos(map_pos);
        let d = (pos.v - p.v).magnitude();
        if d < min_dist {
            min_dist = d;
            closest_map_pos = map_pos;
        }
    }
    let pos = closest_map_pos;
    if state.map().is_inboard(pos) {
        Some(pos)
    } else {
        None
    }
}
