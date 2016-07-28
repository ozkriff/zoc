// See LICENSE file for copyright and license details.

/// Fielf of View

use std::f32::consts::{PI};
use cgmath::{EuclideanVector};
use types::{ZInt, ZFloat};
use map::{Map, Terrain, distance, spiral_iter};
use geom;
use ::{MapPos};

struct Shadow {
    left: ZFloat,
    right: ZFloat,
}

fn is_tile_visible(angle: ZFloat, shadows: &[Shadow]) -> bool {
    for shadow in shadows {
        if shadow.left < angle && shadow.right > angle {
            return false;
        }
    }
    true
}

fn is_obstacle(terrain: &Terrain) -> bool {
    match terrain {
        &Terrain::Trees => true,
        &Terrain::City => true,
        &Terrain::Plain => false,
    }
}

// TODO: precalculate all 'atan2' and 'asin' stuff
pub fn fov(
    map: &Map<Terrain>,
    origin: &MapPos,
    range: ZInt,
    callback: &mut FnMut(&MapPos),
) {
    callback(origin);
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
        if is_obstacle(map.tile(&pos)) {
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

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
