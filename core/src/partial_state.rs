use std::collections::{HashMap};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut};
use fow::{Fow};
use ::{CoreEvent, PlayerId, UnitId, ObjectId, Object, MapPos, Score, Sector, SectorId};

#[derive(Clone, Debug)]
pub struct PartialState {
    state: InternalState,
    fow: Fow,
}

impl PartialState {
    pub fn new(map_name: &str, player_id: PlayerId) -> PartialState {
        let state = InternalState::new(map_name);
        let map_size = state.map().size();
        PartialState {
            state: state,
            fow: Fow::new(map_size, player_id),
        }
    }

    pub fn is_tile_visible(&self, pos: MapPos) -> bool {
        self.fow.is_tile_visible(pos)
    }
}

impl GameState for PartialState {
    fn units(&self) -> &HashMap<UnitId, Unit> {
        self.state.units()
    }

    fn objects(&self) -> &HashMap<ObjectId, Object> {
        self.state.objects()
    }

    fn map(&self) -> &Map<Terrain> {
        self.state.map()
    }

    fn sectors(&self) -> &HashMap<SectorId, Sector> {
        self.state.sectors()
    }

    fn score(&self) -> &HashMap<PlayerId, Score> {
        self.state.score()
    }

    fn reinforcement_points(&self) -> &HashMap<PlayerId, i32> {
        self.state.reinforcement_points()
    }
}

impl GameStateMut for PartialState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        self.state.apply_event(db, event);
        self.fow.apply_event(db, &self.state, event);
    }
}
