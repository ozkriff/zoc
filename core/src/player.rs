use std::collections::{HashSet, VecDeque};
use event::{CoreEvent};
use unit::{UnitId};
use fow::{Fow};
use db::{Db};
use game_state::{State};
use check::{CommandError, check_command};
use event::{Command};
use filter;

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerId{pub id: i32}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PlayerClass {
    Human,
    Ai,
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub id: PlayerId,
    pub class: PlayerClass,
}

#[derive(Clone, Debug)]
pub struct PlayerInfo {
    id: PlayerId,
    events: VecDeque<CoreEvent>,
    visible_enemies: HashSet<UnitId>,

    // This filed is optional because we need to temporary
    // put its Fow into Core's State for filtering events.
    //
    // See State::to_full, State:to_partial
    fow: Option<Fow>,
}

impl PlayerInfo {
    pub fn new(state: &State, id: PlayerId) -> PlayerInfo {
        let fow = Fow::new(state, id);
        PlayerInfo {
            id: id,
            fow: Some(fow),
            events: VecDeque::new(),
            visible_enemies: HashSet::new(),
        }
    }

    pub fn filter_event(&mut self, state: &State, event: &CoreEvent) {
        let (filtered_events, active_unit_ids) = filter::filter_events(
            state, self.id, self.fow(), event);
        for filtered_event in filtered_events {
            self.fow_mut().apply_event(state, &filtered_event);
            self.events.push_back(filtered_event);
            let new_enemies = filter::get_visible_enemies(
                state, self.fow(), self.id);
            let show_hide_events = filter::show_or_hide_passive_enemies(
                state, &active_unit_ids, &self.visible_enemies, &new_enemies);
            self.events.extend(show_hide_events);
            self.visible_enemies = new_enemies;
        }
    }

    pub fn get_event(&mut self) -> Option<CoreEvent> {
        self.events.pop_front()
    }

    pub fn visible_enemies(&self) -> &HashSet<UnitId> {
        &self.visible_enemies
    }

    pub fn check_command(
        &mut self,
        db: &Db,
        state: &mut State,
        command: &Command,
    ) -> Result<(), CommandError> {
        state.to_partial(self.fow.take().unwrap());
        let result = check_command(db, self.id, state, command);
        self.fow = Some(state.to_full());
        result
    }

    pub fn fow(&self) -> &Fow {
        self.fow.as_ref().unwrap()
    }

    pub fn fow_mut(&mut self) -> &mut Fow {
        self.fow.as_mut().unwrap()
    }
}
