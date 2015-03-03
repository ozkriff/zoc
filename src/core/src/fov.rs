// See LICENSE file for copyright and license details.

/// Fielf of View

use std::f32::consts::{PI, PI_2};
use std::num::{Float};
use cgmath::{EuclideanVector};
use common::types::{MapPos, ZFloat};
use map::{Map, Terrain, spiral_iter};
use geom;

struct Shadow {
    left: ZFloat,
    right: ZFloat,
}

fn is_tile_visible(angle: ZFloat, shadows: &Vec<Shadow>) -> bool {
    for shadow in shadows.iter() {
        if shadow.left < angle && shadow.right > angle {
            return false;
        }
    }
    true
}

fn is_obstacle(terrain: &Terrain) -> bool {
    match terrain {
        &Terrain::Trees => true,
        _ => false,
    }
}

// TODO: precalculate all 'atan2' and 'asin' stuff
pub fn fov(map: &Map<Terrain>, fow: &mut Map<bool>, origin: &MapPos) {
    let mut shadows: Vec<Shadow> = vec!();
    let origin3d = geom::map_pos_to_world_pos(origin);
    let range = 7; // TODO
    for pos in spiral_iter(origin, range) {
        if !map.is_inboard(&pos) {
            continue;
        }
        let pos3d = geom::map_pos_to_world_pos(&pos);
        let diff = pos3d - origin3d;
        let distance = diff.length();
        let angle = Float::atan2(diff.x, diff.y); // TODO: optimize
        if is_tile_visible(angle, &shadows) {
            *fow.tile_mut(&pos) = true;
        }
        if is_obstacle(map.tile(&pos)) {
            let obstacle_radius = geom::HEX_IN_RADIUS * 1.1;
            let a = (obstacle_radius / distance).asin();
            let shadow = Shadow{left: angle - a, right: angle + a};
            if shadow.right > PI {
                shadows.push(Shadow{left: -PI, right: shadow.right - PI_2});
            }
            shadows.push(shadow);
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
