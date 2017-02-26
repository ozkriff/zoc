use unit::{Unit, UnitId, UnitTypeId};
use position::{ExactPos, MapPos};
use player::{PlayerId};
use sector::{SectorId};
use object::{ObjectId};
use effect::{Effect};
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

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ReactionFireResult {
    Attacked,
    Killed,
    None,
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
    pub attacker_id: Option<UnitId>,
    pub defender_id: UnitId,
    pub mode: FireMode,
    pub killed: i32,
    pub suppression: i32,
    pub remove_move_points: bool,
    pub is_ambush: bool,
    pub is_inderect: bool,
    pub leave_wrecks: bool,
    pub effect: Option<Effect>,
}

#[derive(Clone, Debug)]
pub enum CoreEvent {
    Move {
        unit_id: UnitId,
        from: ExactPos,
        to: ExactPos,
        mode: MoveMode,
        cost: MovePoints,
    },
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
