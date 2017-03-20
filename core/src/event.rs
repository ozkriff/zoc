use std::collections::{HashMap};
use unit::{Unit, UnitId, UnitTypeId};
use position::{ExactPos, MapPos};
use player::{PlayerId};
use sector::{SectorId};
use object::{ObjectId};
use effect::{TimedEffect};
use movement::{MovePoints};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FireMode {
    Active,
    Reactive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ReactionFireMode {
    Normal,
    HoldFire,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MoveMode {
    Fast,
    Hunt,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Command {
    Move{unit_id: UnitId, path: Vec<ExactPos>, mode: MoveMode},
    EndTurn,
    CreateUnit{pos: ExactPos, type_id: UnitTypeId},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
    LoadUnit{transporter_id: UnitId, passenger_id: UnitId},
    UnloadUnit{transporter_id: UnitId, passenger_id: UnitId, pos: ExactPos},
    Attach{transporter_id: UnitId, attached_unit_id: UnitId},
    Detach{transporter_id: UnitId, pos: ExactPos},
    SetReactionFireMode{unit_id: UnitId, mode: ReactionFireMode},
    Smoke{unit_id: UnitId, pos: MapPos},
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttackInfo {
    // эти поля останутся тут,
    // потому что описывают атаку со стороны атакующего
    pub attacker_id: Option<UnitId>,
    pub mode: FireMode, // TODO: нельзя ли удалить это поле?
    pub is_ambush: bool,
    pub is_inderect: bool,

    // это поле нужно для визуализации атаки
    pub target_pos: ExactPos,
}

#[derive(Clone, Debug)]
pub struct CoreEvent {
    // TODO: точно оба поля должны быть публичными?
    pub event: Event,

    // список целей и примененные к ним эффекты (урон в том числе)
    pub effects: HashMap<UnitId, Vec<TimedEffect>>, // TODO: UnitId -> ObjectId
}

// TODO: заменить тут unit_id на actor_id?
#[derive(Clone, Debug)]
pub enum Event {
    // TODO: эффект отступления тоже надо бы обработать
    // так что эти поля надо сложить во вспомогательную стркутуру,
    // кторая будет общая у события движения и эффекта бегства
    Move {
        unit_id: UnitId,
        from: ExactPos,
        to: ExactPos,
        mode: MoveMode,
        cost: MovePoints,
    },
    // TODO: разбить конец хода на несколько событий?
    EndTurn {
        old_id: PlayerId,
        new_id: PlayerId,
    },
    CreateUnit {
        unit_info: Unit,
    },
    AttackUnit {
        attack_info: AttackInfo,
    },
    // Reveal is like ShowUnit but is generated directly by Core
    Reveal {
        unit_info: Unit,
    },
    ShowUnit {
        unit_info: Unit,
    },
    HideUnit {
        unit_id: UnitId,
    },
    LoadUnit {
        transporter_id: Option<UnitId>,
        passenger_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    UnloadUnit {
        unit_info: Unit,
        transporter_id: Option<UnitId>,
        from: ExactPos,
        to: ExactPos,
    },
    Attach {
        transporter_id: UnitId,
        attached_unit_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    Detach {
        transporter_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    SetReactionFireMode {
        unit_id: UnitId,
        mode: ReactionFireMode,
    },
    SectorOwnerChanged {
        sector_id: SectorId,
        new_owner_id: Option<PlayerId>,
    },
    VictoryPoint {
        player_id: PlayerId,
        pos: MapPos,
        count: i32,
    },
    // TODO: CreateObject
    Smoke {
        id: ObjectId,
        pos: MapPos,
        unit_id: Option<UnitId>,
    },
    // TODO: RemoveObject
    RemoveSmoke {
        id: ObjectId,
    },
}

impl Event {
    pub fn to_core_event(self) -> CoreEvent {
        CoreEvent {
            event: self,
            effects: HashMap::new(),
        }
    }
}
