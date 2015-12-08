// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use common::types::{UnitId, MapPos};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use ::{CoreEvent};

pub trait GameState {
    fn map(&self) -> &Map<Terrain>;
    fn units_at(&self, pos: &MapPos) -> Vec<&Unit>;
    fn is_tile_occupied(&self, pos: &MapPos) -> bool;
    fn units(&self) -> &HashMap<UnitId, Unit>;

    fn unit(&self, id: &UnitId) -> &Unit {
        &self.units()[id]
    }
}

pub trait GameStateMut: GameState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent);
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
