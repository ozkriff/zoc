// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use common::types::{PlayerId, UnitId, MapPos, Size2, ZInt};
use core::{CoreEvent};
use unit::{Unit};
use object::{ObjectTypes};
use map::{Map, Terrain};
use internal_state::{InternalState};
use fow::{Fow};

pub struct GameState {
    state: InternalState,
    fow: Fow,
}

impl<'a> GameState {
    pub fn new(map_size: &Size2<ZInt>, player_id: &PlayerId) -> GameState {
        GameState {
            state: InternalState::new(map_size),
            fow: Fow::new(map_size, player_id),
        }
    }

    pub fn units(&self) -> &HashMap<UnitId, Unit> {
        &self.state.units
    }

    pub fn map(&'a self) -> &Map<Terrain> {
        &self.state.map
    }

    pub fn units_at(&'a self, pos: &MapPos) -> Vec<&'a Unit> {
        self.state.units_at(pos)
    }

    pub fn is_tile_visible(&self, pos: &MapPos) -> bool {
        self.fow.is_visible(pos)
    }

    pub fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        self.state.is_tile_occupied(pos)
    }

    pub fn apply_event(&mut self, object_types: &ObjectTypes, event: &CoreEvent) {
        self.state.apply_event(object_types, event);
        self.fow.apply_event(&self.state, event);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
