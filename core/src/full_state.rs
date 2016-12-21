use std::collections::{HashMap};
use db::{Db};
use unit::{Unit};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut};
use ::{
    CoreEvent,
    PlayerId,
    UnitId,
    ObjectId,
    Object,
    Score,
    Sector,
    SectorId,
    Options,
    ReinforcementPoints,
};

#[derive(Clone, Debug)]
pub struct FullState {
    state: InternalState,
}

impl FullState {
    pub fn new(options: &Options) -> FullState {
        FullState {
            state: InternalState::new(options),
        }
    }

    pub fn inner(&self) -> &InternalState {
        &self.state
    }
}

impl GameState for FullState {
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

    fn reinforcement_points(&self) -> &HashMap<PlayerId, ReinforcementPoints> {
        self.state.reinforcement_points()
    }
}

impl GameStateMut for FullState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        self.state.apply_event(db, event);
    }
}
