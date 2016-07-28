// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use types::{Size2};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut};
use fow::{Fow};
use ::{CoreEvent, PlayerId, UnitId, ObjectId, Object, MapPos};

pub struct PartialState {
    state: InternalState,
    fow: Fow,
}

impl PartialState {
    pub fn new(map_size: &Size2, player_id: &PlayerId) -> PartialState {
        PartialState {
            state: InternalState::new(map_size),
            fow: Fow::new(map_size, player_id),
        }
    }

    pub fn is_tile_visible(&self, pos: &MapPos) -> bool {
        self.fow.is_tile_visible(pos)
    }
}

impl GameState for PartialState {
    fn units(&self) -> &HashMap<UnitId, Unit> {
        &self.state.units()
    }

    fn objects(&self) -> &HashMap<ObjectId, Object> {
        &self.state.objects()
    }

    fn map(&self) -> &Map<Terrain> {
        &self.state.map()
    }
}

impl GameStateMut for PartialState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        self.state.apply_event(db, event);
        self.fow.apply_event(db, &self.state, event);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
