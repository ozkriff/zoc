// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use core::core::{ObjectTypes, Unit, CoreEvent};
use core::map::{Map};
use core::types::{PlayerId, UnitId, MapPos, Size2, ZInt};

pub struct GameState {
    pub units: HashMap<UnitId, Unit>,
    pub map: Map,
}

impl<'a> GameState {
    pub fn new(map_size: &Size2<ZInt>) -> GameState {
        GameState {
            units: HashMap::new(),
            map: Map::new(map_size)
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

    pub fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        self.units_at(pos).len() > 0
    }

    fn refresh_units(&mut self, object_types: &ObjectTypes, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                let unit_type = object_types.get_unit_type(&unit.type_id);
                unit.move_points = unit_type.move_points;
                unit.attack_points = unit_type.attack_points;
            }
        }
    }

    /*
    fn add_passive_ap(&mut self, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                unit.attack_points += 1;
            }
        }
    }
    */

    pub fn apply_event(&mut self, object_types: &ObjectTypes, event: &CoreEvent) {
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let pos = path.destination().clone();
                let unit = self.units.get_mut(unit_id)
                    .expect("BAD MOVE UNIT ID");
                unit.pos = pos;
                assert!(unit.move_points > 0);
                unit.move_points -= path.total_cost().n;
                assert!(unit.move_points >= 0);
            },
            &CoreEvent::EndTurn{ref new_id, old_id: _} => {
                self.refresh_units(object_types, new_id);
                // self.add_passive_ap(old_id);
            },
            &CoreEvent::CreateUnit {
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                assert!(self.units.get(unit_id).is_none());
                let unit_type = object_types.get_unit_type(type_id);
                let move_points = unit_type.move_points;
                let attack_points = unit_type.attack_points;
                self.units.insert(unit_id.clone(), Unit {
                    id: unit_id.clone(),
                    pos: pos.clone(),
                    player_id: player_id.clone(),
                    type_id: type_id.clone(),
                    move_points: move_points,
                    attack_points: attack_points,
                });
            },
            &CoreEvent::AttackUnit {
                ref attacker_id,
                ref defender_id,
                ref killed,
                ..
            } => {
                if *killed {
                    assert!(self.units.get(defender_id).is_some());
                    self.units.remove(defender_id);
                }
                if let Some(unit) = self.units.get_mut(attacker_id) {
                    assert!(unit.attack_points >= 1);
                    unit.attack_points -= 1;
                }
            },
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
