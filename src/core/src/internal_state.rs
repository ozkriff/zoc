// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector2};
use common::types::{PlayerId, UnitId, MapPos, Size2};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use game_state::{GameState};
use ::{CoreEvent, MoveMode, FireMode, UnitInfo, ReactionFireMode};

pub enum InfoLevel {
    Full,
    Partial,
}

pub struct InternalState {
    units: HashMap<UnitId, Unit>,
    map: Map<Terrain>,
}

impl InternalState {
    pub fn new(map_size: &Size2) -> InternalState {
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

    /// Converts active ap (attack points) to reactive
    fn convert_ap(&mut self, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                if let Some(ref mut reactive_attack_points)
                    = unit.reactive_attack_points
                {
                    *reactive_attack_points += unit.attack_points;
                }
                unit.attack_points = 0;
            }
        }
    }

    fn refresh_units(&mut self, db: &Db, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                let unit_type = db.unit_type(&unit.type_id);
                unit.move_points = unit_type.move_points;
                unit.attack_points = unit_type.attack_points;
                if let Some(ref mut reactive_attack_points) = unit.reactive_attack_points {
                    *reactive_attack_points = unit_type.reactive_attack_points;
                }
                unit.morale += 10;
            }
        }
    }

    fn add_unit(&mut self, db: &Db, unit_info: &UnitInfo, info_level: InfoLevel) {
        assert!(self.units.get(&unit_info.unit_id).is_none());
        let unit_type = db.unit_type(&unit_info.type_id);
        self.units.insert(unit_info.unit_id.clone(), Unit {
            id: unit_info.unit_id.clone(),
            pos: unit_info.pos.clone(),
            player_id: unit_info.player_id.clone(),
            type_id: unit_info.type_id.clone(),
            move_points: unit_type.move_points,
            attack_points: unit_type.attack_points,
            reactive_attack_points: if let InfoLevel::Full = info_level {
                Some(unit_type.reactive_attack_points)
            } else {
                None
            },
            reaction_fire_mode: ReactionFireMode::Normal,
            count: unit_type.count,
            morale: 100,
            passenger_id: if let InfoLevel::Full = info_level {
                unit_info.passenger_id.clone()
            } else {
                None
            },
        });
    }
}

impl GameState for InternalState {
    fn units(&self) -> &HashMap<UnitId, Unit> {
        &self.units
    }

    fn map(&self) -> &Map<Terrain> {
        &self.map
    }

    fn units_at(&self, pos: &MapPos) -> Vec<&Unit> {
        let mut units = Vec::new();
        for (_, unit) in &self.units {
            if unit.pos == *pos {
                units.push(unit);
            }
        }
        units
    }

    fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        self.units_at(pos).len() > 0
    }

    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        match event {
            &CoreEvent::Move{ref unit_id, ref path, ref mode} => {
                let pos = path.destination().clone();
                let unit = self.units.get_mut(unit_id)
                    .expect("Bad move unit id");
                unit.pos = pos;
                assert!(unit.move_points > 0);
                if db.unit_type(&unit.type_id).is_transporter {
                    // TODO: get passenger and update its pos
                }
                if let &MoveMode::Fast = mode {
                    unit.move_points -= path.total_cost().n;
                } else {
                    unit.move_points -= path.total_cost().n * 2;
                }
                assert!(unit.move_points >= 0);
            },
            &CoreEvent::EndTurn{ref new_id, ref old_id} => {
                self.refresh_units(db, new_id);
                self.convert_ap(old_id);
            },
            &CoreEvent::CreateUnit{ref unit_info} => {
                self.add_unit(db, unit_info, InfoLevel::Full);
            },
            &CoreEvent::AttackUnit{ref attack_info} => {
                {
                    let unit = self.units.get_mut(&attack_info.defender_id)
                        .expect("Can`t find defender");
                    unit.count -= attack_info.killed;
                    unit.morale -= attack_info.suppression;
                    if attack_info.remove_move_points {
                        unit.move_points = 0;
                    }
                }
                let count = self.units[&attack_info.defender_id].count.clone();
                if count <= 0 {
                    // TODO: kill\unload passengers
                    assert!(self.units.get(&attack_info.defender_id).is_some());
                    self.units.remove(&attack_info.defender_id);
                }
                let attacker_id = match attack_info.attacker_id.clone() {
                    Some(attacker_id) => attacker_id,
                    None => return,
                };
                if let Some(unit) = self.units.get_mut(&attacker_id) {
                    match attack_info.mode {
                        FireMode::Active => {
                            assert!(unit.attack_points >= 1);
                            unit.attack_points -= 1;
                        },
                        FireMode::Reactive => {
                            if let Some(ref mut reactive_attack_points)
                                = unit.reactive_attack_points
                            {
                                assert!(*reactive_attack_points >= 1);
                                *reactive_attack_points -= 1;
                            }
                        },
                    }
                }
            },
            &CoreEvent::ShowUnit{ref unit_info} => {
                self.add_unit(db, unit_info, InfoLevel::Partial);
            },
            &CoreEvent::HideUnit{ref unit_id} => {
                assert!(self.units.get(unit_id).is_some());
                self.units.remove(unit_id);
            },
            &CoreEvent::LoadUnit{ref passenger_id, ref transporter_id} => {
                // TODO: hide info abiut passenger from enemy player
                self.units.get_mut(transporter_id)
                    .expect("Bad transporter_id")
                    .passenger_id = Some(passenger_id.clone());
                let transporter_pos = self.units[transporter_id].pos.clone();
                let passenger = self.units.get_mut(passenger_id)
                    .expect("Bad passenger_id");
                passenger.pos = transporter_pos;
                passenger.move_points = 0;
            },
            &CoreEvent::UnloadUnit{ref transporter_id, ref unit_info} => {
                self.units.get_mut(transporter_id)
                    .expect("Bad transporter_id")
                    .passenger_id = None;
                if let Some(unit) = self.units.get_mut(&unit_info.unit_id) {
                    unit.pos = unit_info.pos.clone();
                    return;
                }
                self.add_unit(db, unit_info, InfoLevel::Partial);
            },
            &CoreEvent::SetReactionFireMode{ref unit_id, ref mode} => {
                self.units.get_mut(unit_id)
                    .expect("Bad unit id")
                    .reaction_fire_mode = mode.clone();
            },
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
