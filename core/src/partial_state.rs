use std::collections::{HashMap};
use std::rc::{Rc};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use internal_state::{InternalState};
use game_state::{GameState, GameStateMut, UnitIter};
use fow::{Fow, FakeFow, fake_fow};
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

    pub fn is_ground_tile_visible(&self, pos: MapPos) -> bool {
        self.fow.is_ground_tile_visible(pos)
    }
}

impl GameState for PartialState {
    type Fow = FakeFow;

    fn units(&self) -> UnitIter<Self::Fow> {
        // self.state doesn't have any 'unknown' information
        // so we don't have to filter it
        UnitIter {
            iter: self.state.raw_units(),
            fow: fake_fow(),
        }
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
