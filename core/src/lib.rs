extern crate cgmath;
extern crate rand;

pub mod geom;
pub mod map;
pub mod db;
pub mod unit;
pub mod dir;
pub mod game_state;
pub mod movement;
pub mod misc;
pub mod types;
pub mod check;
pub mod sector;
pub mod print_info;
pub mod position;
pub mod event;
pub mod player;
pub mod object;
pub mod options;
pub mod attack;
pub mod effect;

mod ai;
mod fov;
mod fow;
mod filter;

use std::{cmp};
use std::collections::{HashMap};
use std::rc::{Rc};
use rand::{thread_rng, Rng};
use game_state::{State};
use options::{Options};
use movement::{MovePoints, tile_cost, move_cost_modifier};
use unit::{Unit, UnitId};
use db::{Db};
use ai::{Ai};
use dir::{Dir};
use attack::{AttackPoints, hit_chance, get_killed_count};
use sector::{check_sectors};
use check::{check_attack};
use player::{Player, PlayerId, PlayerClass, PlayerInfo};
use object::{ObjectId};
use event::{CoreEvent, Event, Command};
use effect::{TimedEffect, Effect, Time};

#[derive(PartialEq, Clone, Copy, Debug)]
enum ReactionFireResult {
    Attacked,
    Killed,
    None,
}

fn get_players_list(options: &Options) -> Vec<Player> {
    assert_eq!(options.players_count, 2);
    vec!(
        Player {
            id: PlayerId{id: 0},
            class: PlayerClass::Human,
        },
        Player {
            id: PlayerId{id: 1},
            class: match options.game_type {
                options::GameType::SingleVsAi => PlayerClass::Ai,
                options::GameType::Hotseat => PlayerClass::Human,
            },
        },
    )
}

// TODO: rename to 'get_player_info_list'
fn get_player_info_lists(state: &State) -> HashMap<PlayerId, PlayerInfo> {
    let mut map = HashMap::new();
    map.insert(PlayerId{id: 0}, PlayerInfo::new(state, PlayerId{id: 0}));
    map.insert(PlayerId{id: 1}, PlayerInfo::new(state, PlayerId{id: 1}));
    map
}

#[derive(Clone, Debug)]
pub struct Core {
    state: State,
    players: Vec<Player>,
    current_player_id: PlayerId,
    db: Rc<Db>,
    ai: Ai,
    players_info: HashMap<PlayerId, PlayerInfo>,
    next_unit_id: UnitId,
    next_object_id: ObjectId,
}

impl Core {
    pub fn new(options: &Options) -> Core {
        let db = Rc::new(Db::new());
        let state = State::new_full(db.clone(), options);
        let players_info = get_player_info_lists(&state);
        let ai = Ai::new(db.clone(), options, PlayerId{id:1});
        let next_object_id = ObjectId{id: state.objects().len() as i32};
        Core {
            state: state,
            players: get_players_list(options),
            current_player_id: PlayerId{id: 0},
            db: db,
            ai: ai,
            players_info: players_info,
            next_unit_id: UnitId{id: 0},
            next_object_id: next_object_id,
        }
    }

    pub fn db(&self) -> &Rc<Db> {
        &self.db
    }

    fn get_new_unit_id(&mut self) -> UnitId {
        self.next_unit_id.id += 1;
        self.next_unit_id
    }

    fn get_new_object_id(&mut self) -> ObjectId {
        self.next_object_id.id += 1;
        self.next_object_id
    }

    fn player(&self) -> &Player {
        &self.players[self.player_id().id as usize]
    }

    pub fn player_id(&self) -> PlayerId {
        self.current_player_id
    }

    // TODO: возвращать сразу вектор?
    pub fn get_event(&mut self) -> Option<CoreEvent> {
        let mut i = self.players_info.get_mut(&self.current_player_id).unwrap();
        i.get_event()
    }

    fn command_attack_unit_to_event(
        &self,
        attacker_id: UnitId,
        defender_id: UnitId,
        fire_mode: event::FireMode,
    ) -> Option<CoreEvent> {
        let attacker = self.state.unit(attacker_id);
        let defender = self.state.unit(defender_id);
        let check_attack_result = check_attack(
            &self.db, &self.state, attacker, defender, fire_mode);
        if check_attack_result.is_err() {
            return None;
        }
        let attacker_type = self.db.unit_type(attacker.type_id);
        let weapon_type = self.db.weapon_type(attacker_type.weapon_type_id);
        let hit_chance = hit_chance(&self.db, &self.state, attacker, defender);
        let suppression = hit_chance.n / 2;
        let defender_type = self.db.unit_type(defender.type_id);
        let is_ground_vehicle = !defender_type.is_infantry && !defender_type.is_air;
        // let mut effect = None;
        let killed = cmp::min(
            defender.count,
            get_killed_count(&self.db, &self.state, attacker, defender),
        );
        /*
        // TODO: создать отдельное событие CoreEvent::Effect
        //
        // Ээээм, ок, допустим что я отменяю убийства и создаю еще события с эффектами
        // Но что мне в событии сохранить тогда?
        // Будет странно, если визуализатор напишет "Missed", а машину
        // обездвижет О.о
        //
        if killed > 0 {
            // TODO: вернуть шанс:
            // if is_ground_vehicle && thread_rng().gen_range(1, 100) <= 50 {
            if is_ground_vehicle {
                // TODO: надо бы переделать всю систему, что бы эффекты
                // были на одном уровне с убийствами
                killed = 0;
                effect = Some(TimedEffect {
                    time: Time::Turns(2),
                    effect: Effect::Immobilized,
                });
            } else if defender_type.is_infantry {
                killed = 0;
                effect = Some(TimedEffect {
                    time: Time::Instant,
                    effect: Effect::Pinned,
                });
            }
            // TODO: добавить другие эффекты
        }
        */
        let fow = self.players_info[&defender.player_id].fow();
        let is_visible = fow.is_visible(attacker);
        let ambush_chance = 70;
        let is_ambush = !is_visible
            && thread_rng().gen_range(1, 100) <= ambush_chance;
        let per_death_suppression = 20;
        // TODO: destroyed helicopters must kill everyone
        // on the ground in their tile
        let attack_info = event::AttackInfo {
            attacker_id: Some(attacker_id),
            mode: fire_mode,
            is_ambush: is_ambush,
            is_inderect: weapon_type.is_inderect,
            target_pos: defender.pos,
        };
        Some(CoreEvent {
            event: Event::AttackUnit{attack_info: attack_info},
            effects: {
                let mut effects = HashMap::new();
                let effect = TimedEffect {
                    time: Time::Instant,
                    effect: Effect::Attacked(effect::Attacked {
                        killed: killed,
                        suppression: suppression + per_death_suppression * killed,
                        remove_move_points: false,
                        leave_wrecks: is_ground_vehicle,
                    }),
                };
                effects.insert(defender_id, vec![effect]);
                effects
            },
        })
    }

    fn can_unit_make_reaction_attack(
        &self,
        defender: &Unit,
        attacker: &Unit,
    ) -> bool {
        assert!(attacker.player_id != defender.player_id);
        if attacker.reaction_fire_mode == event::ReactionFireMode::HoldFire {
            return false;
        }
        // TODO: move to `check_attack`
        let fow = self.players_info[&attacker.player_id].fow();
        if !fow.is_visible(defender) {
            return false;
        }
        let check_attack_result = check_attack(
            &self.db,
            &self.state,
            attacker,
            defender,
            event::FireMode::Reactive,
        );
        check_attack_result.is_ok()
    }

    fn reaction_fire_internal(
        &mut self,
        unit_id: UnitId,
        stop_on_attack: bool,
    ) -> ReactionFireResult {
        let unit_ids: Vec<_> = self.state.units().map(|(&id, _)| id).collect();
        let mut result = ReactionFireResult::None;
        for enemy_unit_id in unit_ids {
            if unit::is_loaded_or_attached(self.state.unit(enemy_unit_id)) {
                continue;
            }
            let event = {
                let enemy_unit = self.state.unit(enemy_unit_id);
                let unit = self.state.unit(unit_id);
                if enemy_unit.player_id == unit.player_id {
                    continue;
                }
                if !self.can_unit_make_reaction_attack(unit, enemy_unit) {
                    continue;
                }
                let event = self.command_attack_unit_to_event(
                    enemy_unit.id, unit_id, event::FireMode::Reactive);
                if let Some(CoreEvent{event: Event::AttackUnit{attack_info}, ..}) = event {
                    let hit_chance = attack::hit_chance(
                        &self.db, &self.state, enemy_unit, unit);
                    let unit_type = self.db.unit_type(unit.type_id);
                    // if hit_chance.n > 15 && !unit_type.is_air && stop_on_attack {
                    if hit_chance.n > 15 && unit_type.is_infantry && stop_on_attack {
                        // attack_info.remove_move_points = true;
                        // TODO: создать новое событие - Effect::Pinned
                    }
                    Event::AttackUnit{attack_info: attack_info}.to_core_event()
                } else {
                    continue;
                }
            };
            self.do_core_event(&event);
            result = ReactionFireResult::Attacked;
            if self.state.unit_opt(unit_id).is_none() {
                return ReactionFireResult::Killed;
            }
        }
        result
    }

    fn reaction_fire(&mut self, unit_id: UnitId) {
        self.reaction_fire_internal(unit_id, false);
    }

    pub fn next_player_id(&self, id: PlayerId) -> PlayerId {
        let old_id = id.id;
        let max_id = self.players.len() as i32;
        PlayerId{id: if old_id + 1 == max_id {
            0
        } else {
            old_id + 1
        }}
    }

    fn check_command(&mut self, command: &Command) {
        let db = &self.db;
        let player_id = self.current_player_id;
        let mut i = self.players_info.get_mut(&player_id).unwrap();
        if let Err(err) = i.check_command(db, &mut self.state, command) {
            panic!("Bad command: {:?} ({:?})", err, command);
        }
    }

    fn simulation_step(&mut self, command: Command) {
        match command {
            Command::EndTurn => {
                let old_id = self.current_player_id;
                let new_id = self.next_player_id(old_id);
                // TODO: extruct func
                let mut end_turn_events = Vec::new();
                for sector in self.state.sectors().values() {
                    if let Some(player_id) = sector.owner_id {
                        if player_id != new_id {
                            continue;
                        }
                        end_turn_events.push(Event::VictoryPoint {
                            player_id: player_id,
                            pos: sector.center(),
                            count: 1,
                        }.to_core_event());
                    }
                }
                for (&object_id, object) in self.state.objects() {
                    if let Some(timer) = object.timer {
                        if timer <= 0 {
                            end_turn_events.push(Event::RemoveSmoke {
                                id: object_id,
                            }.to_core_event());
                        }
                    }
                }
                for event in end_turn_events {
                    self.do_core_event(&event);
                }
                self.do_core_event(&Event::EndTurn {
                    old_id: old_id,
                    new_id: new_id,
                }.to_core_event());
            },
            Command::CreateUnit{pos, type_id} => {
                let event = {
                    let id = self.get_new_unit_id();
                    let unit_type = self.db.unit_type(type_id);
                    Event::CreateUnit {
                        unit_info: Unit {
                            id: id,
                            player_id: self.current_player_id,
                            pos: pos,
                            type_id: type_id,
                            passenger_id: None,
                            attached_unit_id: None,
                            move_points: Some(MovePoints{n: 0}),
                            attack_points: Some(AttackPoints{n: 0}),
                            reactive_attack_points: Some(AttackPoints{n: 0}),
                            reaction_fire_mode: event::ReactionFireMode::Normal,
                            count: unit_type.count,
                            morale: 100,
                            is_alive: true,
                            is_loaded: false,
                            is_attached: false,
                        },
                    }.to_core_event()
                };
                self.do_core_event(&event);
            },
            Command::Move{unit_id, path, mode} => {
                let player_id = self.state.unit(unit_id).player_id;
                for window in path.windows(2) {
                    let from = window[0];
                    let to = window[1];
                    let show_event = self.state.unit_at_opt(to).and_then(|unit| {
                        Some(Event::Reveal {
                            unit_info: unit.clone(),
                        }.to_core_event())
                    });
                    if let Some(event) = show_event {
                        self.do_core_event(&event);
                        continue;
                    }
                    let move_event = {
                        let unit = self.state.unit(unit_id);
                        let cost = MovePoints {
                            n: tile_cost(&self.db, &self.state, unit, from, to).n
                                * move_cost_modifier(mode)
                        };
                        Event::Move {
                            unit_id: unit_id,
                            from: from,
                            to: to,
                            mode: mode,
                            cost: cost,
                        }.to_core_event()
                    };
                    let pre_visible_enemies = self.players_info[&player_id]
                        .visible_enemies().clone();
                    self.do_core_event(&move_event);
                    //
                    // TODO: при движении техники по сложным участкам
                    // создавать CoreEvent::Effect(Застрял\Замедился)
                    //
                    let reaction_fire_result = self.reaction_fire_internal(
                        unit_id, mode == event::MoveMode::Fast);
                    if reaction_fire_result != ReactionFireResult::None {
                        break;
                    }
                    let i = &self.players_info[&player_id];
                    if &pre_visible_enemies != i.visible_enemies() {
                        break;
                    }
                }
            },
            Command::AttackUnit{attacker_id, defender_id} => {
                if let Some(ref event) = self.command_attack_unit_to_event(
                    attacker_id, defender_id, event::FireMode::Active)
                {
                    self.do_core_event(event);
                    self.reaction_fire(attacker_id);
                }
            },
            Command::LoadUnit{transporter_id, passenger_id} => {
                let from = self.state.unit(passenger_id).pos;
                let to = self.state.unit(transporter_id).pos;
                self.do_core_event(&Event::LoadUnit {
                    transporter_id: Some(transporter_id),
                    passenger_id: passenger_id,
                    from: from,
                    to: to,
                }.to_core_event());
            },
            Command::UnloadUnit{transporter_id, passenger_id, pos} => {
                let event = Event::UnloadUnit {
                    unit_info: Unit {
                        pos: pos,
                        .. self.state.unit(passenger_id).clone()
                    },
                    transporter_id: Some(transporter_id),
                    from: self.state.unit(transporter_id).pos,
                    to: pos,
                }.to_core_event();
                self.do_core_event(&event);
                self.reaction_fire(passenger_id);
            },
            Command::Attach{transporter_id, attached_unit_id} => {
                let from = self.state.unit(transporter_id).pos;
                let to = self.state.unit(attached_unit_id).pos;
                self.do_core_event(&Event::Attach {
                    transporter_id: transporter_id,
                    attached_unit_id: attached_unit_id,
                    from: from,
                    to: to,
                }.to_core_event());
                self.reaction_fire(transporter_id);
            },
            Command::Detach{transporter_id, pos} => {
                let from = self.state.unit(transporter_id).pos;
                self.do_core_event(&Event::Detach {
                    transporter_id: transporter_id,
                    from: from,
                    to: pos,
                }.to_core_event());
                self.reaction_fire(transporter_id);
            },
            Command::SetReactionFireMode{unit_id, mode} => {
                self.do_core_event(&Event::SetReactionFireMode {
                    unit_id: unit_id,
                    mode: mode,
                }.to_core_event());
            },
            Command::Smoke{unit_id, pos} => {
                let id = self.get_new_object_id();
                self.do_core_event(&Event::Smoke {
                    id: id,
                    unit_id: Some(unit_id),
                    pos: pos,
                }.to_core_event());
                let mut dir = Dir::from_int(thread_rng().gen_range(0, 5));
                let additional_smoke_count = {
                    let unit = self.state.unit(unit_id);
                    let unit_type = self.db.unit_type(unit.type_id);
                    let weapon_type = self.db.weapon_type(unit_type.weapon_type_id);
                    weapon_type.smoke.unwrap()
                };
                assert!(additional_smoke_count <= 3);
                for _ in 0..additional_smoke_count {
                    let mut dir_index = dir.to_int() + thread_rng().gen_range(1, 3);
                    if dir_index > 5 {
                        dir_index -= 6;
                    }
                    dir = Dir::from_int(dir_index);
                    let id = self.get_new_object_id();
                    self.do_core_event(&Event::Smoke {
                        id: id,
                        unit_id: Some(unit_id),
                        pos: Dir::get_neighbour_pos(pos, dir),
                    }.to_core_event());
                }
                self.reaction_fire(unit_id);
            },
        };
        let sector_events = check_sectors(&self.db, &self.state);
        for event in sector_events {
            self.do_core_event(&event);
        }
    }

    pub fn do_command(&mut self, command: Command) {
        self.check_command(&command);
        self.simulation_step(command);
    }

    fn do_ai(&mut self) {
        loop {
            while let Some(event) = self.get_event() {
                self.ai.apply_event(&event);
            }
            let command = self.ai.get_command();
            self.do_command(command.clone());
            if command == Command::EndTurn {
                return;
            }
        }
    }

    fn handle_end_turn_event(&mut self, old_id: PlayerId, new_id: PlayerId) {
        for player in &self.players {
            if player.id == new_id {
                if self.current_player_id == old_id {
                    self.current_player_id = player.id;
                }
                break;
            }
        }
        if self.player().class == PlayerClass::Ai
            && new_id == self.player_id()
        {
            self.do_ai();
        }
    }

    fn filter_event(&mut self, player_id: PlayerId, event: &CoreEvent) {
        let mut i = self.players_info.get_mut(&player_id).unwrap();
        i.filter_event(&self.state, event);
    }

    fn do_core_event(&mut self, event: &CoreEvent) {
        self.state.apply_event(event);
        let player_ids: Vec<_> = self.players.iter()
            .map(|player| player.id).collect();
        for player_id in player_ids {
            self.filter_event(player_id, event);
        }
        if let Event::EndTurn{old_id, new_id} = event.event {
            self.handle_end_turn_event(old_id, new_id);
        }
    }
}
