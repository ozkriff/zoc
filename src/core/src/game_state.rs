// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use common::types::{PlayerId, UnitId, MapPos, Size2};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use internal_state::{InternalState};
use fow::{Fow};
use ::{CoreEvent};

pub struct GameState {
    state: InternalState,
    fow: Fow,
}

impl<'a> GameState {
    pub fn new(map_size: &Size2, player_id: &PlayerId) -> GameState {
        GameState {
            state: InternalState::new(map_size),
            fow: Fow::new(map_size, player_id),
        }
    }

    pub fn units(&self) -> &HashMap<UnitId, Unit> {
        &self.state.units()
    }

    pub fn unit(&'a self, id: &UnitId) -> &'a Unit {
        self.state.unit(id)
    }

    pub fn map(&'a self) -> &Map<Terrain> {
        &self.state.map()
    }

    pub fn units_at(&'a self, pos: &MapPos) -> Vec<&'a Unit> {
        self.state.units_at(pos)
    }

    pub fn is_tile_visible(&self, pos: &MapPos) -> bool {
        self.fow.is_tile_visible(pos)
    }

    pub fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        self.state.is_tile_occupied(pos)
    }

    pub fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        self.state.apply_event(db, event);
        self.fow.apply_event(db, &self.state, event);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
