// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
use types::{ZFloat};
use ::{MapPos};

pub const HEX_EX_RADIUS: ZFloat = 1.4;

// (pow(1.0, 2) - pow(0.5, 2)).sqrt()
pub const HEX_IN_RADIUS: ZFloat = 0.866025403784 * HEX_EX_RADIUS;

pub fn map_pos_to_world_pos(i: &MapPos) -> Vector2<ZFloat> {
    let v = Vector2 {
        x: (i.v.x as ZFloat) * HEX_IN_RADIUS * 2.0,
        y: (i.v.y as ZFloat) * HEX_EX_RADIUS * 1.5,
    };
    if i.v.y % 2 == 0 {
        Vector2{x: v.x + HEX_IN_RADIUS, y: v.y}
    } else {
        v
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
