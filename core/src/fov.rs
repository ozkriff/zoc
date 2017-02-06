/// Field of View

use std::f32::consts::{PI};
use cgmath::{InnerSpace};
use game_state::{State};
use map::{Terrain, spiral_iter};
use geom;
use ::{MapPos, ObjectClass, Distance};

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

fn is_obstacle(state: &State, pos: MapPos) -> bool {
    match *state.map().tile(pos){
        Terrain::Trees | Terrain::City => return true,
        Terrain::Plain | Terrain::Water => {},
    }
    for object in state.objects_at(pos) {
        match object.class {
            ObjectClass::Building |
            ObjectClass::Smoke => return true,
            ObjectClass::ReinforcementSector |
            ObjectClass::Road => {},
        }
    }
    false
}

// TODO: precalculate all 'atan2' and 'asin' stuff
pub fn fov(
    state: &State,
    origin: MapPos,
    range: Distance,
    callback: &mut FnMut(MapPos),
) {
    callback(origin);
    let map = state.map();
    let mut shadows = vec!();
    let origin3d = geom::map_pos_to_world_pos(origin);
    for pos in spiral_iter(origin, range) {
        if !map.is_inboard(pos) {
            continue;
        }
        let pos3d = geom::map_pos_to_world_pos(pos);
        let diff = pos3d - origin3d;
        let distance = diff.magnitude();
        let angle = diff.x.atan2(diff.y); // TODO: optimize
        if is_tile_visible(angle, &shadows) {
            callback(pos);
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

pub fn simple_fov(
    state: &State,
    origin: MapPos,
    range: Distance,
    callback: &mut FnMut(MapPos),
) {
    callback(origin);
    for pos in spiral_iter(origin, range) {
        if state.map().is_inboard(pos) {
            callback(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Vector2};
    use std::rc::{Rc};
    use game_state::{State};
    use db::{Db};
    use fov::{fov};
    use ::{MapPos, Options, GameType, Distance};

    #[test]
    fn test_bug_149_fov_is_asymmetric() {
        let db = Rc::new(Db::new());
        let options = Options {
            game_type: GameType::Hotseat,
            map_name: "map_fov_bug_test".into(),
            players_count: 2,
        };
        let state = State::new_full(db, &options);
        let range = Distance{n: 2};
        let origin = MapPos { v: Vector2 { x: 10, y: 10 } };
        let mut real = vec![];
        fov(&state, origin, range, &mut |pos| real.push(pos));
        let expected = vec![
            MapPos { v: Vector2 {x: 10, y: 10} },

            MapPos { v: Vector2 {x: 10, y: 9} }, // trees here
            MapPos { v: Vector2 {x: 11, y: 9} },
            MapPos { v: Vector2 {x: 11, y: 10} },
            MapPos { v: Vector2 {x: 11, y: 11} },
            MapPos { v: Vector2 {x: 10, y: 11} },
            MapPos { v: Vector2 {x: 9, y: 10} }, // trees here

            MapPos { v: Vector2 {x: 10, y: 8} }, // TODO: WTF?!
            MapPos { v: Vector2 {x: 11, y: 8} },
            MapPos { v: Vector2 {x: 12, y: 9} },
            MapPos { v: Vector2 {x: 12, y: 10} },
            MapPos { v: Vector2 {x: 12, y: 11} },
            MapPos { v: Vector2 {x: 11, y: 12} },
            MapPos { v: Vector2 {x: 10, y: 12} },
            MapPos { v: Vector2 {x: 9, y: 12} }
        ];
        assert_eq!(real, expected);
    }
}
