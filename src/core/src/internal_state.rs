// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector2};
use types::{ZInt, Size2};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use game_state::{GameState, GameStateMut};
use ::{
    CoreEvent,
    FireMode,
    UnitInfo,
    ReactionFireMode,
    PlayerId,
    UnitId,
    MapPos,
    ExactPos,
    SlotId,
    Object,
    ObjectId,
    ObjectClass,
    get_free_slot_for_building,
};

pub enum InfoLevel {
    Full,
    Partial,
}

pub struct InternalState {
    units: HashMap<UnitId, Unit>,
    objects: HashMap<ObjectId, Object>,
    map: Map<Terrain>,
}

impl InternalState {
    pub fn new(map_size: &Size2) -> InternalState {
        let mut map = Map::new(map_size);
        // TODO: read from scenario.json?
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 3}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 4}}) = Terrain::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 4, y: 5}}) = Terrain::Trees;
        let mut state = InternalState {
            units: HashMap::new(),
            objects: HashMap::new(),
            map: map,
        };
        state.add_buildings(&MapPos{v: Vector2{x: 5, y: 4}}, 2);
        state.add_buildings(&MapPos{v: Vector2{x: 5, y: 5}}, 2);
        state.add_buildings(&MapPos{v: Vector2{x: 5, y: 6}}, 1);
        state.add_big_building(&MapPos{v: Vector2{x: 6, y: 4}});
        state.add_buildings(&MapPos{v: Vector2{x: 6, y: 5}}, 3);
        state.add_buildings(&MapPos{v: Vector2{x: 6, y: 6}}, 1);
        state
    }

    fn add_object(&mut self, object: Object) {
        let id = ObjectId{id: self.objects.len() as ZInt + 1};
        self.objects.insert(id, object);
    }

    fn add_big_building(&mut self, pos: &MapPos) {
        *self.map.tile_mut(pos) = Terrain::City;
        let object = Object {
            class: ObjectClass::Building,
            pos: ExactPos {
                map_pos: pos.clone(),
                slot_id: SlotId::WholeTile,
            },
        };
        self.add_object(object);
    }

    fn add_buildings(&mut self, pos: &MapPos, count: ZInt) {
        *self.map.tile_mut(pos) = Terrain::City;
        for _ in 0 .. count {
            let slot_id = get_free_slot_for_building(self, pos).unwrap();
            let obj_pos = ExactPos{map_pos: pos.clone(), slot_id: slot_id};
            let object = Object {
                class: ObjectClass::Building,
                pos: obj_pos,
            };
            self.add_object(object);
        }
    }

    /// Converts active ap (attack points) to reactive
    fn convert_ap(&mut self, db: &Db, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            let unit_type = db.unit_type(&unit.type_id);
            let weapon_type = db.weapon_type(&unit_type.weapon_type_id);
            if unit.player_id != *player_id || !weapon_type.reaction_fire {
                continue;
            }
            if let Some(ref mut reactive_attack_points)
                = unit.reactive_attack_points
            {
                reactive_attack_points.n += unit.attack_points.n;
            }
            unit.attack_points.n = 0;
        }
    }

    fn refresh_units(&mut self, db: &Db, player_id: &PlayerId) {
        for (_, unit) in self.units.iter_mut() {
            if unit.player_id == *player_id {
                let unit_type = db.unit_type(&unit.type_id);
                unit.move_points = unit_type.move_points.clone();
                unit.attack_points = unit_type.attack_points.clone();
                if let Some(ref mut reactive_attack_points) = unit.reactive_attack_points {
                    *reactive_attack_points = unit_type.reactive_attack_points.clone();
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
            move_points: unit_type.move_points.clone(),
            attack_points: unit_type.attack_points.clone(),
            reactive_attack_points: if let InfoLevel::Full = info_level {
                Some(unit_type.reactive_attack_points.clone())
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

    fn objects(&self) -> &HashMap<ObjectId, Object> {
        &self.objects
    }

    fn map(&self) -> &Map<Terrain> {
        &self.map
    }
}

impl GameStateMut for InternalState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        match event {
            &CoreEvent::Move{ref unit_id, ref to, ref cost, ..} => {
                {
                    let unit = self.units.get_mut(unit_id).unwrap();
                    unit.pos = to.clone();
                    assert!(unit.move_points.n > 0);
                    unit.move_points.n -= cost.n;
                    assert!(unit.move_points.n >= 0);
                }
                if let Some(passenger_id) = self.units[unit_id].passenger_id.clone() {
                    let passenger = self.units.get_mut(&passenger_id).unwrap();
                    passenger.pos = to.clone();
                }
            },
            &CoreEvent::EndTurn{ref new_id, ref old_id} => {
                self.refresh_units(db, new_id);
                self.convert_ap(db, old_id);
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
                        unit.move_points.n = 0;
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
                            assert!(unit.attack_points.n >= 1);
                            unit.attack_points.n -= 1;
                        },
                        FireMode::Reactive => {
                            if let Some(ref mut reactive_attack_points)
                                = unit.reactive_attack_points
                            {
                                assert!(reactive_attack_points.n >= 1);
                                reactive_attack_points.n -= 1;
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
            &CoreEvent::LoadUnit{ref passenger_id, ref transporter_id, ref to, ..} => {
                // TODO: hide info about passenger from enemy player
                if let &Some(ref transporter_id) = transporter_id {
                    self.units.get_mut(transporter_id)
                        .expect("Bad transporter_id")
                        .passenger_id = Some(passenger_id.clone());
                }
                let passenger = self.units.get_mut(passenger_id)
                    .expect("Bad passenger_id");
                passenger.pos = to.clone();
                passenger.move_points.n = 0;
            },
            &CoreEvent::UnloadUnit{ref transporter_id, ref unit_info, ..} => {
                if let &Some(ref transporter_id) = transporter_id {
                    self.units.get_mut(transporter_id)
                        .expect("Bad transporter_id")
                        .passenger_id = None;
                }
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
