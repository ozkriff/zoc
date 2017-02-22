use position::{ExactPos};
use event::{ReactionFireMode};
use player::{PlayerId};
use map::{Distance};
use movement::{MovePoints};
use attack::{AttackPoints};
use game_state::{ReinforcementPoints};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct UnitId{pub id: i32}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct UnitTypeId{pub id: i32}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub pos: ExactPos,
    pub player_id: PlayerId,
    pub type_id: UnitTypeId,
    pub move_points: Option<MovePoints>,
    pub attack_points: Option<AttackPoints>,
    pub reactive_attack_points: Option<AttackPoints>,
    pub reaction_fire_mode: ReactionFireMode,
    pub count: i32,
    pub morale: i32,
    pub passenger_id: Option<UnitId>,
    pub attached_unit_id: Option<UnitId>,
    pub is_alive: bool,
    pub is_loaded: bool,
    pub is_attached: bool,
}

#[derive(Clone, Debug)]
pub struct WeaponType {
    pub name: String,
    pub damage: i32,
    pub ap: i32,
    pub accuracy: i32,
    pub max_distance: Distance,
    pub min_distance: Distance,
    pub max_air_distance: Option<Distance>,
    pub is_inderect: bool,
    pub reaction_fire: bool,
    pub smoke: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponTypeId{pub id: i32}

#[derive(Clone, Debug)]
pub struct UnitType {
    pub name: String,
    pub count: i32,
    pub size: i32,
    pub armor: i32,
    pub toughness: i32,
    pub weapon_skill: i32,
    pub weapon_type_id: WeaponTypeId,
    pub move_points: MovePoints,
    pub attack_points: AttackPoints,
    pub reactive_attack_points: AttackPoints,
    pub los_range: Distance,
    pub cover_los_range: Distance,
    pub is_transporter: bool,
    pub is_big: bool,
    pub is_air: bool,
    pub is_infantry: bool,
    pub can_be_towed: bool,
    pub cost: ReinforcementPoints,
}

pub fn is_commandable(player_id: PlayerId, unit: &Unit) -> bool {
    unit.is_alive && unit.player_id == player_id
        && !is_loaded_or_attached(unit)
}

pub fn is_loaded_or_attached(unit: &Unit) -> bool {
    unit.is_loaded || unit.is_attached
}
