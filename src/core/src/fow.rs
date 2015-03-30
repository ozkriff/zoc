// See LICENSE file for copyright and license details.

use common::types::{PlayerId, MapPos, Size2, ZInt};
use core::{CoreEvent};
use internal_state::{InternalState};
use map::{Map};
use fov::{fov};

/// Fog of War
pub struct Fow {
    map: Map<bool>,
    player_id: PlayerId,
}

impl Fow {
    pub fn new(map_size: &Size2<ZInt>, player_id: &PlayerId) -> Fow {
        Fow {
            map: Map::new(map_size, false),
            player_id: player_id.clone(),
        }
    }

    pub fn is_visible(&self, pos: &MapPos) -> bool {
        *self.map.tile(pos)
    }

    fn clear_fow(&mut self) {
        for pos in self.map.get_iter() {
            *self.map.tile_mut(&pos) = false;
        }
    }

    fn reset_fow(&mut self, state: &InternalState) {
        self.clear_fow();
        for (_, unit) in state.units.iter() {
            if unit.player_id == self.player_id {
                *self.map.tile_mut(&unit.pos) = true;
                fov(&state.map, &mut self.map, &unit.pos);
            }
        }
    }

    pub fn apply_event(&mut self, state: &InternalState, event: &CoreEvent) {
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let unit = state.units.get(unit_id)
                    .expect("BAD MOVE UNIT ID"); // TODO: fix errmsg
                if unit.player_id == self.player_id {
                    for path_node in path.nodes() {
                        fov(&state.map, &mut self.map, &path_node.pos);
                    }
                }
            },
            &CoreEvent::EndTurn{ref new_id, ..} => {
                if self.player_id == *new_id {
                    self.reset_fow(state);
                }
            },
            &CoreEvent::CreateUnit{ref pos, ref player_id, ..} => {
                if self.player_id == *player_id {
                    fov(&state.map, &mut self.map, pos);
                }
            },
            &CoreEvent::AttackUnit{..} => {},
            &CoreEvent::ShowUnit{..} => {},
            &CoreEvent::HideUnit{..} => {},
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
