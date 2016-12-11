use unit::{UnitType, WeaponType, UnitTypeId, WeaponTypeId};
use ::{MovePoints, AttackPoints};

fn weapon_type_id(weapon_types: &[WeaponType], name: &str)
    -> WeaponTypeId
{
    for (id, weapon_type) in weapon_types.iter().enumerate() {
        if weapon_type.name == name {
            return WeaponTypeId{id: id as i32};
        }
    }
    panic!("No weapon type with name \"{}\"", name);
}

// TODO: read from json/toml config
fn get_weapon_types() -> Vec<WeaponType> {
    vec![
        WeaponType {
            name: "mortar".to_owned(),
            damage: 6,
            ap: 2,
            accuracy: 5,
            max_distance: 5,
            max_air_distance: None,
            min_distance: 1,
            is_inderect: true,
            reaction_fire: false,
            smoke: Some(3),
        },
        WeaponType {
            name: "super_heavy_tank_gun".to_owned(),
            damage: 11,
            ap: 11,
            accuracy: 5,
            max_distance: 6,
            max_air_distance: None,
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "heavy_tank_gun".to_owned(),
            damage: 9,
            ap: 9,
            accuracy: 5,
            max_distance: 5,
            max_air_distance: None,
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "medium_tank_gun".to_owned(),
            damage: 7,
            ap: 7,
            accuracy: 5,
            max_distance: 4,
            max_air_distance: None,
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "light_tank_gun".to_owned(),
            damage: 6,
            ap: 5,
            accuracy: 5,
            max_distance: 4,
            max_air_distance: None,
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "rifle".to_owned(),
            damage: 2,
            ap: 1,
            accuracy: 5,
            max_distance: 3,
            max_air_distance: Some(2),
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "submachine_gun".to_owned(),
            damage: 3,
            ap: 1,
            accuracy: 4,
            max_distance: 2,
            max_air_distance: Some(1),
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
        WeaponType {
            name: "machine_gun".to_owned(),
            damage: 5,
            ap: 2,
            accuracy: 5,
            max_distance: 3,
            max_air_distance: Some(2),
            min_distance: 0,
            is_inderect: false,
            reaction_fire: true,
            smoke: None,
        },
    ]
}

// TODO: read from json/toml config
fn get_unit_types(weapon_types: &[WeaponType]) -> Vec<UnitType> {
    vec![
        UnitType {
            name: "mammoth_tank".to_owned(),
            size: 12,
            count: 1,
            armor: 13,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "super_heavy_tank_gun"),
            move_points: MovePoints{n: 5},
            attack_points: AttackPoints{n: 1},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: true,
            is_air: false,
            is_infantry: false,
            can_be_towed: false,
            cost: 16,
        },
        UnitType {
            name: "heavy_tank".to_owned(),
            size: 8,
            count: 1,
            armor: 11,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "heavy_tank_gun"),
            move_points: MovePoints{n: 7},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 10,
        },
        UnitType {
            name: "medium_tank".to_owned(),
            size: 7,
            count: 1,
            armor: 9,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "medium_tank_gun"),
            move_points: MovePoints{n: 8},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 8,
        },
        UnitType {
            name: "light_tank".to_owned(),
            size: 6,
            count: 1,
            armor: 7,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "light_tank_gun"),
            move_points: MovePoints{n: 10},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 6,
        },
        UnitType {
            name: "light_spg".to_owned(),
            size: 6,
            count: 1,
            armor: 5,
            toughness: 9,
            weapon_skill: 7,
            weapon_type_id: weapon_type_id(weapon_types, "medium_tank_gun"),
            move_points: MovePoints{n: 10},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 6,
        },
        UnitType {
            name: "field_gun".to_owned(),
            size: 6,
            count: 1,
            armor: 3,
            toughness: 7,
            weapon_skill: 7,
            // TODO: "tank_gun" on field gun??
            weapon_type_id: weapon_type_id(weapon_types, "medium_tank_gun"),
            move_points: MovePoints{n: 7},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 7,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: true,
            can_be_towed: true,
            cost: 5,
        },
        UnitType {
            name: "jeep".to_owned(),
            size: 5,
            count: 1,
            armor: 2,
            toughness: 3,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "machine_gun"),
            move_points: MovePoints{n: 12},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 8,
            cover_los_range: 0,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 4,
        },
        UnitType {
            name: "truck".to_owned(),
            size: 6,
            count: 1,
            armor: 2,
            toughness: 3,
            weapon_skill: 0,
            weapon_type_id: weapon_type_id(weapon_types, "machine_gun"), // TODO: remove hack
            move_points: MovePoints{n: 10},
            attack_points: AttackPoints{n: 0},
            reactive_attack_points: AttackPoints{n: 0},
            los_range: 6,
            cover_los_range: 0,
            is_transporter: true,
            is_big: false,
            is_air: false,
            is_infantry: false,
            can_be_towed: true,
            cost: 4,
        },
        UnitType {
            name: "helicopter".to_owned(),
            size: 9,
            count: 1,
            armor: 3,
            toughness: 3,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "machine_gun"),
            move_points: MovePoints{n: 10},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 8,
            cover_los_range: 0,
            is_transporter: false,
            is_big: true,
            is_air: true,
            is_infantry: false,
            can_be_towed: false,
            cost: 10,
        },
        UnitType {
            name: "soldier".to_owned(),
            size: 4,
            count: 4,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "rifle"),
            move_points: MovePoints{n: 9},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 6,
            cover_los_range: 1,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: true,
            can_be_towed: false,
            cost: 2,
        },
        UnitType {
            name: "smg".to_owned(),
            size: 4,
            count: 3,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "submachine_gun"),
            move_points: MovePoints{n: 9},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 6,
            cover_los_range: 1,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: true,
            can_be_towed: false,
            cost: 2,
        },
        UnitType {
            name: "scout".to_owned(),
            size: 4,
            count: 2,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "rifle"),
            move_points: MovePoints{n: 11},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 1},
            los_range: 8,
            cover_los_range: 2,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: true,
            can_be_towed: false,
            cost: 3,
        },
        UnitType {
            name: "mortar".to_owned(),
            size: 4,
            count: 1,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: weapon_type_id(weapon_types, "mortar"),
            move_points: MovePoints{n: 7},
            attack_points: AttackPoints{n: 2},
            reactive_attack_points: AttackPoints{n: 0},
            los_range: 6,
            cover_los_range: 1,
            is_transporter: false,
            is_big: false,
            is_air: false,
            is_infantry: true,
            can_be_towed: false,
            cost: 4,
        },
    ]
}

#[derive(Clone, Debug)]
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

    fn unit_type_id_opt(&self, name: &str) -> Option<UnitTypeId> {
        for (id, unit_type) in self.unit_types.iter().enumerate() {
            if unit_type.name == name {
                return Some(UnitTypeId{id: id as i32});
            }
        }
        None
    }

    pub fn unit_types(&self) -> &[UnitType] {
        &self.unit_types
    }

    pub fn unit_type(&self, unit_type_id: UnitTypeId) -> &UnitType {
        &self.unit_types[unit_type_id.id as usize]
    }

    pub fn weapon_type(&self, type_id: WeaponTypeId) -> &WeaponType {
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
