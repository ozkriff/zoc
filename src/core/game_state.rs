// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use core::core::{ObjectTypes, Unit, CoreEvent};
use core::types::{PlayerId, UnitId, MapPos};

pub struct GameState {
    pub units: HashMap<UnitId, Unit>,
}

impl<'a> GameState {
    pub fn new() -> GameState {
        GameState {
            units: HashMap::new(),
        }
    }

    pub fn units_at(&'a self, pos: &MapPos) -> Vec<&'a Unit> {
        let mut units = Vec::new();
        for (_, unit) in self.units.iter() {
            if unit.pos == *pos {
                units.push(unit);
            }
        }
        units
    }

    fn refresh_units(&mut self, object_types: &ObjectTypes, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                unit.move_points = object_types
                    .get_unit_type(&unit.type_id).move_points;
                unit.attacked = false;
            }
        }
    }

    pub fn apply_event(&mut self, object_types: &ObjectTypes, event: &CoreEvent) {
        match *event {
            CoreEvent::Move{ref unit_id, ref path} => {
                let pos = path.destination().clone();
                let unit = self.units.get_mut(unit_id).unwrap();
                unit.pos = pos;
                assert!(unit.move_points > 0);
                unit.move_points -= path.total_cost().n;
                assert!(unit.move_points >= 0);
            },
            CoreEvent::EndTurn{new_id: _, old_id: ref new_player_id} => {
                self.refresh_units(object_types, new_player_id);
            },
            CoreEvent::CreateUnit{ref unit_id, ref pos, ref type_id, ref player_id} => {
                assert!(self.units.get(unit_id).is_none());
                let move_points
                    = object_types.get_unit_type(type_id).move_points;
                self.units.insert(unit_id.clone(), Unit {
                    id: unit_id.clone(),
                    pos: pos.clone(),
                    player_id: player_id.clone(),
                    type_id: type_id.clone(),
                    move_points: move_points,
                    attacked: false,
                });
            },
            CoreEvent::AttackUnit{ref attacker_id, ref defender_id, ref killed} => {
                if *killed {
                    assert!(self.units.get(defender_id).is_some());
                    self.units.remove(defender_id);
                }
                let unit = self.units.get_mut(attacker_id).unwrap();
                assert!(!unit.attacked);
                unit.attacked = true;
            },
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
