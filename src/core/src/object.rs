// See LICENSE file for copyright and license details.

use common::types::{ZInt};
use unit::{Unit, UnitType, WeaponType, UnitClass, UnitTypeId, WeaponTypeId};

// TODO: Rename?
pub struct ObjectTypes {
    unit_types: Vec<UnitType>,
    weapon_types: Vec<WeaponType>,
}

impl ObjectTypes {
    pub fn new() -> ObjectTypes {
        let mut object_types = ObjectTypes {
            unit_types: vec![],
            weapon_types: vec![],
        };
        object_types.get_weapon_types();
        object_types.get_unit_types();
        object_types
    }

    pub fn get_unit_types_count(&self) -> ZInt {
        self.unit_types.len() as ZInt
    }

    // TODO: read from json/toml config
    fn get_weapon_types(&mut self) {
        self.weapon_types.push(WeaponType {
            name: "cannon".to_string(),
            damage: 9,
            ap: 9,
            accuracy: 5,
            max_distance: 5,
        });
        self.weapon_types.push(WeaponType {
            name: "rifle".to_string(),
            damage: 2,
            ap: 1,
            accuracy: 5,
            max_distance: 3,
        });
    }

    // TODO: read from json/toml config
    fn get_unit_types(&mut self) {
        let cannon_id = self.get_weapon_type_id("cannon");
        let rifle_id = self.get_weapon_type_id("rifle");
        self.unit_types.push(UnitType {
            name: "tank".to_string(),
            class: UnitClass::Vehicle,
            size: 6,
            count: 1,
            armor: 11,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: cannon_id,
            move_points: 5,
            attack_points: 2,
            los_range: 6,
            cover_los_range: 0,
        });
        self.unit_types.push(UnitType {
            name: "soldier".to_string(),
            class: UnitClass::Infantry,
            size: 4,
            count: 4,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: rifle_id.clone(),
            move_points: 4,
            attack_points: 2,
            los_range: 6,
            cover_los_range: 1,
        });
        self.unit_types.push(UnitType {
            name: "scout".to_string(),
            class: UnitClass::Infantry,
            size: 4,
            count: 2,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: rifle_id.clone(),
            move_points: 6,
            attack_points: 2,
            los_range: 8,
            cover_los_range: 2,
        });
    }

    fn get_unit_type_id_opt(&self, name: &str) -> Option<UnitTypeId> {
        for (id, unit_type) in self.unit_types.iter().enumerate() {
            if unit_type.name == name {
                return Some(UnitTypeId{id: id as ZInt});
            }
        }
        None
    }

    pub fn get_unit_type<'a>(&'a self, unit_type_id: &UnitTypeId) -> &'a UnitType {
        &self.unit_types[unit_type_id.id as usize]
    }

    pub fn get_weapon_type<'a>(&'a self, type_id: &WeaponTypeId) -> &'a WeaponType {
        &self.weapon_types[type_id.id as usize]
    }

    pub fn get_unit_type_id(&self, name: &str) -> UnitTypeId {
        match self.get_unit_type_id_opt(name) {
            Some(id) => id,
            None => panic!("No unit type with name: \"{}\"", name),
        }
    }

    pub fn get_weapon_type_id(&self, name: &str) -> WeaponTypeId {
        for (id, weapon_type) in self.weapon_types.iter().enumerate() {
            if weapon_type.name == name {
                return WeaponTypeId{id: id as ZInt};
            }
        }
        panic!("No weapon type with name \"{}\"", name);
    }

    pub fn get_unit_max_attack_dist(&self, unit: &Unit) -> ZInt {
        let attacker_type = self.get_unit_type(&unit.type_id);
        let weapon_type = &self
            .weapon_types[attacker_type.weapon_type_id.id as usize];
        weapon_type.max_distance
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
