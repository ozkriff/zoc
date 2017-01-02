use cgmath::{self, SquareMatrix, EuclideanSpace};
use collision::{Plane, Ray, Intersect};
use core::{MapPos};
use core::game_state::{State};
use context::{Context};
use geom;
use camera::Camera;
use types::{WorldPos};

pub fn pick_world_pos(context: &Context, camera: &Camera) -> WorldPos {
    let im = camera.mat().invert()
        .expect("Can`t invert camera matrix");
    let win_size = context.win_size();
    let w = win_size.w as f32;
    let h = win_size.h as f32;
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
    state: &State,
    camera: &Camera,
) -> Option<MapPos> {
    let world_pos = pick_world_pos(context, camera);
    let pos = geom::world_pos_to_map_pos(world_pos);
    if state.map().is_inboard(pos) {
        Some(pos)
    } else {
        None
    }
}
