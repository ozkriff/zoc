/// Fielf of View

use std::f32::consts::{PI};
use cgmath::{InnerSpace};
use game_state::{GameState};
use map::{Terrain, spiral_iter};
use geom;
use ::{MapPos, ObjectClass};

struct Shadow {
    left: f32,
    right: f32,
}

fn is_tile_visible(angle: f32, shadows: &[Shadow]) -> bool {
    for shadow in shadows {
        if shadow.left < angle && shadow.right > angle {
            return false;
        }
    }
    true
}

fn is_obstacle<S: GameState>(state: &S, pos: MapPos) -> bool {
    match *state.map().tile(&pos){
        Terrain::Trees | Terrain::City => return true,
        Terrain::Plain | Terrain::Water => {},
    }
    for object in state.objects_at(&pos) {
        match object.class {
            ObjectClass::Building | ObjectClass::Smoke => return true,
            ObjectClass::Road => {},
        }
    }
    return false;
}

// TODO: precalculate all 'atan2' and 'asin' stuff
pub fn fov<S: GameState>(
    state: &S,
    origin: &MapPos,
    range: i32,
    callback: &mut FnMut(&MapPos),
) {
    callback(origin);
    let map = state.map();
    let mut shadows = vec!();
    let origin3d = geom::map_pos_to_world_pos(origin);
    for pos in spiral_iter(origin, range) {
        if !map.is_inboard(&pos) {
            continue;
        }
        let pos3d = geom::map_pos_to_world_pos(&pos);
        let diff = pos3d - origin3d;
        let distance = diff.magnitude();
        let angle = diff.x.atan2(diff.y); // TODO: optimize
        if is_tile_visible(angle, &shadows) {
            callback(&pos);
        }
        if is_obstacle(state, pos) {
            let obstacle_radius = geom::HEX_IN_RADIUS * 1.1;
            let a = (obstacle_radius / distance).asin();
            let shadow = Shadow{left: angle - a, right: angle + a};
            if shadow.right > PI {
                shadows.push(Shadow{left: -PI, right: shadow.right - PI * 2.0});
            }
            shadows.push(shadow);
        }
    }
}
