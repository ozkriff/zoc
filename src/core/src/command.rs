// See LICENSE file for copyright and license details.

use common::types::{UnitId, MapPos};
use pathfinder::{MapPath};

#[derive(Clone)]
pub enum MoveMode {
    Fast,
    Hunt,
}

#[derive(Clone)]
pub enum Command {
    Move{unit_id: UnitId, path: MapPath, mode: MoveMode},
    EndTurn,
    CreateUnit{pos: MapPos},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
    LoadUnit{transporter_id: UnitId, passanger_id: UnitId},
    UnloadUnit{transporter_id: UnitId, passanger_id: UnitId, pos: MapPos},
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
