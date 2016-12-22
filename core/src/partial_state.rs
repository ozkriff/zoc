use std::collections::{HashMap};
use std::rc::{Rc};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut};
use fow::{Fow};
use ::{
    CoreEvent,
    PlayerId,
    UnitId,
    ObjectId,
    Object,
    MapPos,
    Score,
    Sector,
    SectorId,
    Options,
    ReinforcementPoints,
};

#[derive(Clone, Debug)]
pub struct PartialState {
    state: InternalState,
    fow: Fow,
    db: Rc<Db>,
}

impl PartialState {
    pub fn new(db: Rc<Db>, options: &Options, player_id: PlayerId) -> PartialState {
        let state = InternalState::new(db.clone(), options);
        let map_size = state.map().size();
        PartialState {
            state: state,
            fow: Fow::new(db.clone(), map_size, player_id),
            db: db,
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

    fn unit_opt(&self, id: UnitId) -> Option<&Unit> {
        self.state.unit_opt(id)
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

impl GameStateMut for PartialState {
    fn apply_event(&mut self, event: &CoreEvent) {
        self.state.apply_event(event);
        self.fow.apply_event(&self.state, event);
    }
}
