// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector2};
use common::types::{PlayerId, UnitId, MapPos, Size2, ZInt};
use core::{CoreEvent};
use unit::{Unit, UnitTypeId};
use db::{Db};
use map::{Map, Terrain};

pub struct InternalState {
    // TODO: remove 'pub'?
    pub units: HashMap<UnitId, Unit>,
    pub map: Map<Terrain>,
}

impl<'a> InternalState {
    pub fn new(map_size: &Size2<ZInt>) -> InternalState {
        let mut map = Map::new(map_size, Terrain::Plain);
        // TODO: read from scenario.json?
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 3}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 4}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 5}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 5, y: 5}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 6, y: 4}}) = Terrain::Trees;
        InternalState {
            units: HashMap::new(),
            map: map,
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
        // TODO: optimize
        self.units_at(pos).len() > 0
    }

    fn refresh_units(&mut self, db: &Db, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                let unit_type = db.unit_type(&unit.type_id);
                unit.move_points = unit_type.move_points;
                unit.attack_points = unit_type.attack_points;
            }
        }
    }

    fn add_passive_ap(&mut self, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                unit.attack_points += 1;
            }
        }
    }

    fn add_unit(
        &mut self,
        db: &Db,
        unit_id: &UnitId,
        pos: &MapPos,
        type_id: &UnitTypeId,
        player_id: &PlayerId,
    ) {
        assert!(self.units.get(unit_id).is_none());
        let unit_type = db.unit_type(type_id);
        let move_points = unit_type.move_points;
        let attack_points = unit_type.attack_points;
        self.units.insert(unit_id.clone(), Unit {
            id: unit_id.clone(),
            pos: pos.clone(),
            player_id: player_id.clone(),
            type_id: type_id.clone(),
            move_points: move_points,
            attack_points: attack_points,
            count: unit_type.count,
        });
    }

    pub fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let pos = path.destination().clone();
                let unit = self.units.get_mut(unit_id)
                    .expect("Bad move unit id");
                unit.pos = pos;
                assert!(unit.move_points > 0);
                unit.move_points -= path.total_cost().n;
                assert!(unit.move_points >= 0);
            },
            &CoreEvent::EndTurn{ref new_id, ref old_id} => {
                self.refresh_units(db, new_id);
                self.add_passive_ap(old_id);
            },
            &CoreEvent::CreateUnit {
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                self.add_unit(db, unit_id, pos, type_id, player_id);
            },
            &CoreEvent::AttackUnit {
                ref attacker_id,
                ref defender_id,
                ref killed,
                ..
            } => {
                self.units.get_mut(defender_id)
                    .expect("Can`t find defender")
                    .count -= *killed;
                let count = self.units[defender_id].count.clone();
                if count <= 0 {
                    assert!(self.units.get(defender_id).is_some());
                    self.units.remove(defender_id);
                }
                if let Some(unit) = self.units.get_mut(attacker_id) {
                    assert!(unit.attack_points >= 1);
                    unit.attack_points -= 1;
                }
            },
            &CoreEvent::ShowUnit{
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                self.add_unit(db, unit_id, pos, type_id, player_id);
            },
            &CoreEvent::HideUnit{ref unit_id} => {
                assert!(self.units.get(unit_id).is_some());
                self.units.remove(unit_id);
            },
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
