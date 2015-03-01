// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector2};
use common::types::{PlayerId, UnitId, MapPos, Size2, ZInt};
use core::{CoreEvent};
use unit::{Unit};
use object::{ObjectTypes};
use map::{Map, Terrain};
use fov;

pub struct GameState {
    // TODO: remove 'pub'?
    pub units: HashMap<UnitId, Unit>,
    pub map: Map<Terrain>,
    pub fow: Map<bool>, // Fog of War map
    pub player_id: Option<PlayerId>,
}

impl<'a> GameState {
    // TODO: Remove 'player_id'
    pub fn new(map_size: &Size2<ZInt>, player_id: Option<&PlayerId>) -> GameState {
        let mut map = Map::new(map_size, Terrain::Plain);
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 3}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 4}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 5}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 5, y: 5}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 6, y: 4}}) = Terrain::Trees;
        GameState {
            units: HashMap::new(),
            map: map,
            fow: Map::new(map_size, false),
            player_id: if let Some(player_id) = player_id {
                Some(player_id.clone())
            } else {
                None
            },
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

    pub fn clear_fow(&mut self) {
        for pos in self.fow.get_iter() {
            *self.fow.tile_mut(&pos) = false;
        }
    }

    fn update_fow(&mut self) {
        let player_id = self.player_id.as_ref().unwrap();
        for (_, unit) in self.units.iter() {
            if unit.player_id == *player_id {
                *self.fow.tile_mut(&unit.pos) = true;
                fov::fov(&self.map, &mut self.fow, &unit.pos);
            }
        }
    }

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
                if let Some(player_id) = self.player_id.clone() {
                    if unit.player_id == player_id {
                        for &(_, ref pos) in path.nodes() {
                            fov::fov(&self.map, &mut self.fow, pos);
                        }
                    }
                }
            },
            &CoreEvent::EndTurn{ref new_id, old_id: _} => {
                // TODO: remove ugly '.clone()'?
                if let Some(player_id) = self.player_id.clone() {
                    if player_id == *new_id {
                        self.clear_fow();
                    }
                }
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
        if self.player_id.is_some() {
            self.update_fow();
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
