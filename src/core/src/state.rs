// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use common::types::{UnitId, MapPos};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use ::{CoreEvent};

// TODO: rename to GameState
pub trait State<'a> {
    fn map(&'a self) -> &Map<Terrain>;
    fn units_at(&'a self, pos: &MapPos) -> Vec<&'a Unit>;
    fn is_tile_occupied(&self, pos: &MapPos) -> bool;
    fn apply_event(&mut self, db: &Db, event: &CoreEvent);
    fn units(&self) -> &HashMap<UnitId, Unit>;

    fn unit(&'a self, id: &UnitId) -> &'a Unit {
        &self.units()[id]
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
