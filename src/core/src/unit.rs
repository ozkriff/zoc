// See LICENSE file for copyright and license details.

use common::types::{ZInt, UnitId, PlayerId, MapPos};

#[derive(Clone)]
pub struct UnitTypeId{pub id: ZInt}

#[derive(Clone)]
pub enum UnitClass {
    Infantry,
    Vehicle,
}

pub struct Unit {
    pub id: UnitId,
    pub pos: MapPos,
    pub player_id: PlayerId,
    pub type_id: UnitTypeId,
    pub move_points: ZInt,
    pub attack_points: ZInt,
    pub count: ZInt,
}

pub struct WeaponType {
    pub name: String,
    pub damage: ZInt,
    pub ap: ZInt,
    pub accuracy: ZInt,
    pub max_distance: ZInt,
}

#[derive(Clone)]
pub struct WeaponTypeId{pub id: ZInt}

#[derive(Clone)]
pub struct UnitType {
    pub name: String,
    pub class: UnitClass,
    pub count: ZInt,
    pub size: ZInt,
    pub armor: ZInt,
    pub toughness: ZInt,
    pub weapon_skill: ZInt,
    pub weapon_type_id: WeaponTypeId,
    pub move_points: ZInt,
    pub attack_points: ZInt,
    pub los_range: ZInt,
    pub cover_los_range: ZInt,
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
