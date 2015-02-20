// See LICENSE file for copyright and license details.

use core::types::{UnitId, MapPos};
use core::pathfinder::{MapPath};

#[derive(Clone)]
pub enum Command {
    Move{unit_id: UnitId, path: MapPath},
    EndTurn,
    CreateUnit{pos: MapPos},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
