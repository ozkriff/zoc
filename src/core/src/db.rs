// See LICENSE file for copyright and license details.

use common::types::{ZInt};
use unit::{UnitType, WeaponType, UnitClass, UnitTypeId, WeaponTypeId};
use ::{MovePoints};

fn weapon_type_id(weapon_types: &[WeaponType], name: &str)
    -> WeaponTypeId
{
    for (id, weapon_type) in weapon_types.iter().enumerate() {
        if weapon_type.name == name {
            return WeaponTypeId{id: id as ZInt};
        }
    }
    panic!("No weapon type with name \"{}\"", name);
}

// TODO: read from json/toml config
fn get_weapon_types() -> Vec<WeaponType> {
    vec![
        WeaponType {
            name: "cannon".to_owned(),
            damage: 9,
            ap: 9,
            accuracy: 5,
            max_distance: 5,
            min_distance: 2,
        },
        WeaponType {
            name: "rifle".to_owned(),
            damage: 2,
            ap: 1,
            accuracy: 5,
            max_distance: 3,
            min_distance: 1,
        },
    ]
}

// TODO: read from json/toml config
fn get_unit_types(weapon_types: &[WeaponType]) -> Vec<UnitType> {
    let cannon_id = weapon_type_id(weapon_types, "cannon");
    let rifle_id = weapon_type_id(weapon_types, "rifle");
    vec![
        UnitType {
            name: "tank".to_owned(),
            class: UnitClass::Vehicle,
            size: 6,
            count: 1,
            armor: 11,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: cannon_id.clone(),
            move_points: MovePoints{n: 5},
            attack_points: 2,
            reactive_attack_points: 1,
            los_range: 6,
            cover_los_range: 0,
            is_transporter: false,
        },
        UnitType {
            name: "truck".to_owned(),
            class: UnitClass::Vehicle,
            size: 6,
            count: 1,
            armor: 2,
            toughness: 3,
            weapon_skill: 0,
            weapon_type_id: cannon_id.clone(), // TODO: remove hack
            move_points: MovePoints{n: 7},
            attack_points: 0,
            reactive_attack_points: 0,
            los_range: 6,
            cover_los_range: 0,
            is_transporter: true,
        },
        UnitType {
            name: "soldier".to_owned(),
            class: UnitClass::Infantry,
            size: 4,
            count: 4,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: rifle_id.clone(),
            move_points: MovePoints{n: 4},
            attack_points: 2,
            reactive_attack_points: 1,
            los_range: 6,
            cover_los_range: 1,
            is_transporter: false,
        },
        UnitType {
            name: "scout".to_owned(),
            class: UnitClass::Infantry,
            size: 4,
            count: 2,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: rifle_id.clone(),
            move_points: MovePoints{n: 6},
            attack_points: 2,
            reactive_attack_points: 1,
            los_range: 8,
            cover_los_range: 2,
            is_transporter: false,
        },
    ]
}

pub struct Db {
    unit_types: Vec<UnitType>,
    weapon_types: Vec<WeaponType>,
}

impl Db {
    pub fn new() -> Db {
        let weapon_types = get_weapon_types();
        let unit_types = get_unit_types(&weapon_types);
        Db {
            weapon_types: weapon_types,
            unit_types: unit_types,
        }
    }

    pub fn unit_types_count(&self) -> ZInt {
        self.unit_types.len() as ZInt
    }

    fn unit_type_id_opt(&self, name: &str) -> Option<UnitTypeId> {
        for (id, unit_type) in self.unit_types.iter().enumerate() {
            if unit_type.name == name {
                return Some(UnitTypeId{id: id as ZInt});
            }
        }
        None
    }

    pub fn unit_type(&self, unit_type_id: &UnitTypeId) -> &UnitType {
        &self.unit_types[unit_type_id.id as usize]
    }

    pub fn weapon_type(&self, type_id: &WeaponTypeId) -> &WeaponType {
        &self.weapon_types[type_id.id as usize]
    }

    pub fn unit_type_id(&self, name: &str) -> UnitTypeId {
        match self.unit_type_id_opt(name) {
            Some(id) => id,
            None => panic!("No unit type with name: \"{}\"", name),
        }
    }

    pub fn weapon_type_id(&self, name: &str) -> WeaponTypeId {
        weapon_type_id(&self.weapon_types, name)
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
