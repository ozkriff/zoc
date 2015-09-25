// See LICENSE file for copyright and license details.

use rand::{thread_rng, Rng};
use std::cmp;
use std::collections::{HashMap, HashSet, LinkedList};
use cgmath::{Vector2};
use common::types::{Size2, ZInt, UnitId, PlayerId, MapPos};
use common::misc::{clamp};
use internal_state::{InternalState};
use map::{Map, Terrain, distance};
use pathfinder::{MapPath};
use unit::{Unit, UnitType, UnitTypeId, UnitClass};
use db::{Db};
use player::{Player};
use ai::{Ai};
use fow::{Fow};
use fov::{fov};
use filter;

#[derive(Clone)]
pub enum FireMode {
    Active,
    Reactive,
}

#[derive(Clone)]
pub enum ReactionFireMode {
    Normal,
    HoldFire,
}

#[derive(Clone, PartialEq, Eq)]
pub enum MoveMode {
    Fast,
    Hunt,
}

#[derive(Clone)]
pub enum Command {
    Move{unit_id: UnitId, path: MapPath, mode: MoveMode},
    EndTurn,
    CreateUnit{pos: MapPos},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
    LoadUnit{transporter_id: UnitId, passanger_id: UnitId},
    UnloadUnit{transporter_id: UnitId, passanger_id: UnitId, pos: MapPos},
    SetReactionFireMode{unit_id: UnitId, mode: ReactionFireMode},
}

#[derive(Clone)]
pub struct UnitInfo {
    pub unit_id: UnitId,
    pub pos: MapPos,
    pub type_id: UnitTypeId,
    pub player_id: PlayerId,
    pub passanger_id: Option<UnitId>,
}

#[derive(Clone)]
pub struct AttackInfo {
    pub attacker_id: Option<UnitId>,
    pub defender_id: UnitId,
    pub mode: FireMode,
    pub killed: ZInt,
    pub suppression: ZInt,
    pub remove_move_points: bool,
    pub is_ambush: bool,
}

#[derive(Clone)]
pub enum CoreEvent {
    Move {
        unit_id: UnitId,
        path: MapPath,
        mode: MoveMode,
    },
    EndTurn {
        old_id: PlayerId,
        new_id: PlayerId,
    },
    CreateUnit {
        unit_info: UnitInfo,
    },
    AttackUnit {
        attack_info: AttackInfo,
    },
    ShowUnit {
        unit_info: UnitInfo,
    },
    HideUnit {
        unit_id: UnitId,
    },
    LoadUnit {
        transporter_id: UnitId,
        passanger_id: UnitId,
    },
    UnloadUnit {
        unit_info: UnitInfo,
        transporter_id: UnitId,
    },
    SetReactionFireMode {
        unit_id: UnitId,
        mode: ReactionFireMode,
    },
}

pub fn unit_to_info(unit: &Unit) -> UnitInfo {
    UnitInfo {
        unit_id: unit.id.clone(),
        pos: unit.pos.clone(),
        type_id: unit.type_id.clone(),
        player_id: unit.player_id.clone(),
        passanger_id: unit.passanger_id.clone(),
    }
}

struct PlayerInfo {
    events: LinkedList<CoreEvent>,
    fow: Fow,
    visible_enemies: HashSet<UnitId>,
}

pub struct Core {
    state: InternalState,
    players: Vec<Player>,
    current_player_id: PlayerId,
    db: Db,
    ai: Ai,
    players_info: HashMap<PlayerId, PlayerInfo>,
    next_unit_id: UnitId,
}

fn get_players_list() -> Vec<Player> {
    vec!(
        Player{id: PlayerId{id: 0}, is_ai: false},
        // Player{id: PlayerId{id: 1}, is_ai: true},
        Player{id: PlayerId{id: 1}, is_ai: false},
    )
}

fn get_player_info_lists(map_size: &Size2) -> HashMap<PlayerId, PlayerInfo> {
    let mut map = HashMap::new();
    map.insert(PlayerId{id: 0}, PlayerInfo {
        fow: Fow::new(map_size, &PlayerId{id: 0}),
        events: LinkedList::new(),
        visible_enemies: HashSet::new(),
    });
    map.insert(PlayerId{id: 1}, PlayerInfo {
        fow: Fow::new(map_size, &PlayerId{id: 1}),
        events: LinkedList::new(),
        visible_enemies: HashSet::new(),
    });
    map
}

pub fn los(
    map: &Map<Terrain>,
    unit_type: &UnitType,
    from: &MapPos,
    to: &MapPos,
) -> bool {
    // TODO: profile and optimize!
    let mut v = false;
    let range = unit_type.los_range;
    fov(map, from, range, &mut |p| if *p == *to { v = true });
    v
}

impl Core {
    pub fn new() -> Core {
        let map_size = Size2{w: 10, h: 8};
        let mut core = Core {
            state: InternalState::new(&map_size),
            players: get_players_list(),
            current_player_id: PlayerId{id: 0},
            db: Db::new(),
            ai: Ai::new(&PlayerId{id:1}, &map_size),
            players_info: get_player_info_lists(&map_size),
            next_unit_id: UnitId{id: 0},
        };
        core.get_units();
        core
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    // TODO: Move to scenario.json
    fn get_units(&mut self) {
        let tank_id = self.db.unit_type_id("tank");
        let truck_id = self.db.unit_type_id("truck");
        let soldier_id = self.db.unit_type_id("soldier");
        let scout_id = self.db.unit_type_id("scout");
        let p_id_0 = PlayerId{id: 0};
        let p_id_1 = PlayerId{id: 1};
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 1}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 2}}, &soldier_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 3}}, &scout_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 1, y: 3}}, &truck_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 4}}, &soldier_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 5}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 6}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 1}}, &tank_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 2}}, &soldier_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 3}}, &scout_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 4}}, &soldier_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 8, y: 4}}, &truck_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 5}}, &tank_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 6}}, &tank_id, &p_id_1);
    }

    fn get_new_unit_id(&mut self) -> UnitId {
        let new_unit_id = self.next_unit_id.clone();
        self.next_unit_id.id += 1;
        new_unit_id
    }

    fn add_unit(&mut self, pos: &MapPos, type_id: &UnitTypeId, player_id: &PlayerId) {
        if self.state.is_tile_occupied(pos) {
            println!("Sorry, tile is occupied");
            return;
        }
        let new_unit_id = self.get_new_unit_id();
        let event = CoreEvent::CreateUnit {
            unit_info: UnitInfo {
                unit_id: new_unit_id,
                pos: pos.clone(),
                type_id: type_id.clone(),
                player_id: player_id.clone(),
                passanger_id: None,
            },
        };
        self.do_core_event(&event);
    }

    pub fn map_size(&self) -> &Size2 {
        self.state.map().size()
    }

    fn get_killed_count(&self, attacker: &Unit, defender: &Unit) -> ZInt {
        let hit = self.hit_test(attacker, defender);
        if !hit {
            return 0;
        }
        let defender_type = self.db.unit_type(&defender.type_id);
        match defender_type.class {
            UnitClass::Infantry => {
                clamp(thread_rng().gen_range(1, 5), 1, defender.count)
            },
            UnitClass::Vehicle => 1,
        }
    }

    fn hit_test(&self, attacker: &Unit, defender: &Unit) -> bool {
        fn test(needed: ZInt) -> bool {
            let real = thread_rng().gen_range(-5i32, 5i32);
            let result = real < needed;
            // println!("real:{} < needed:{} = {}", real, needed, result);
            result
        }
        // println!("");
        let attacker_type = self.db.unit_type(&attacker.type_id);
        let defender_type = self.db.unit_type(&defender.type_id);
        let weapon_type = self.db.weapon_type(&attacker_type.weapon_type_id);
        if distance(&attacker.pos, &defender.pos) > weapon_type.max_distance {
            return false;
        }
        let hit_test_v = -15 + defender_type.size
            + weapon_type.accuracy + attacker_type.weapon_skill;
        let pierce_test_v = 5 + -defender_type.armor + weapon_type.ap;
        let wound_test_v = -defender_type.toughness + weapon_type.damage;
        // println!("hit_test = {}, pierce_test = {}, wound_test_v = {}",
        //     hit_test_v, pierce_test_v, wound_test_v);
        // print!("hit test: ");
        if !test(hit_test_v) {
            return false;
        }
        // print!("pierce test: ");
        if !test(pierce_test_v) {
            return false;
        }
        // print!("wound test: ");
        if !test(wound_test_v) {
            return false;
        }
        // println!("HIT!");
        true
        // false
    }

    pub fn player(&self) -> &Player {
        &self.players[self.player_id().id as usize]
    }

    pub fn player_id(&self) -> &PlayerId {
        &self.current_player_id
    }

    pub fn get_event(&mut self) -> Option<CoreEvent> {
        let mut i = self.players_info.get_mut(&self.current_player_id)
            .expect("core: Can`t get current player`s info");
        i.events.pop_front()
    }

    // TODO: &UnitType -> &UnitId? &Unit?
    fn los(&self, unit_type: &UnitType, from: &MapPos, to: &MapPos) -> bool {
        los(self.state.map(), unit_type, from, to)
    }

    fn command_attack_unit_to_event(
        &self,
        attacker_id: &UnitId,
        defender_id: &UnitId,
        defender_pos: &MapPos,
        fire_mode: &FireMode,
    ) -> Option<CoreEvent> {
        let attacker = self.state.unit(&attacker_id);
        let defender = self.state.unit(&defender_id);
        let attacker_type = self.db.unit_type(&attacker.type_id);
        let weapon_type = self.db.weapon_type(&attacker_type.weapon_type_id);
        if distance(&attacker.pos, defender_pos) > weapon_type.max_distance {
            return None;
        }
        if !self.los(attacker_type, &attacker.pos, defender_pos) {
            return None;
        }
        if attacker.morale < 50 {
            return None;
        }
        let killed = cmp::min(
            defender.count, self.get_killed_count(attacker, defender));
        let fow = &self.players_info[&defender.player_id].fow;
        let is_visible = fow.is_visible(
            &self.db, &self.state, attacker, &attacker.pos);
        let is_ambush = !is_visible && thread_rng().gen_range(1, 10) > 3;
        let attack_info = AttackInfo {
            attacker_id: Some(attacker_id.clone()),
            defender_id: defender_id.clone(),
            killed: killed,
            mode: fire_mode.clone(),
            suppression: 10 + 20 * killed,
            remove_move_points: false,
            is_ambush: is_ambush,
        };
        Some(CoreEvent::AttackUnit{attack_info: attack_info})
    }

    fn can_unit_make_reaction_attack(
        &self,
        defender: &Unit,
        defender_pos: &MapPos,
        attacker: &Unit,
    ) -> bool {
        assert!(attacker.player_id != defender.player_id);
        let enemy_reactive_attack_points = attacker.reactive_attack_points
            .expect("Core must know about everything").clone();
        if enemy_reactive_attack_points <= 0 {
            return false;
        }
        if attacker.morale < 50 {
            return false;
        }
        let fow = &self.players_info[&attacker.player_id].fow;
        if !fow.is_visible(&self.db, &self.state, defender, defender_pos) {
            return false;
        }
        let max_distance = self.db.unit_max_attack_dist(attacker);
        if distance(&attacker.pos, defender_pos) > max_distance {
            return false;
        }
        let enemy_type = self.db.unit_type(&attacker.type_id);
        if !self.los(enemy_type, &attacker.pos, defender_pos) {
            return false;
        }
        if let ReactionFireMode::HoldFire = attacker.reaction_fire_mode {
            return false;
        }
        true
    }

    fn reaction_fire_check(&self, unit_id: &UnitId, pos: &MapPos) -> bool {
        let unit = self.state.unit(unit_id);
        for (_, attacker) in self.state.units() {
            if attacker.player_id == unit.player_id {
                continue; // This is not enemy
            }
            if self.can_unit_make_reaction_attack(unit, pos, attacker) {
                return true;
            }
        }
        false
    }

    fn reaction_fire_internal<F>(&mut self, unit_id: &UnitId, pos: &MapPos, f: F)
        where F: Fn(AttackInfo) -> AttackInfo
    {
        let unit_ids: Vec<_> = self.state.units().keys()
            .map(|id| id.clone()).collect();
        for enemy_unit_id in unit_ids {
            let event = {
                let enemy_unit = self.state.unit(&enemy_unit_id);
                let unit = self.state.unit(unit_id);
                if enemy_unit.player_id == unit.player_id {
                    continue;
                }
                if !self.can_unit_make_reaction_attack(unit, pos, enemy_unit) {
                    continue;
                }
                let event = self.command_attack_unit_to_event(
                    &enemy_unit.id, &unit_id, pos, &FireMode::Reactive);
                if let Some(CoreEvent::AttackUnit{attack_info}) = event {
                    CoreEvent::AttackUnit{attack_info: f(attack_info)}
                } else {
                    continue;
                }
            };
            self.do_core_event(&event);
            if self.state.units().get(unit_id).is_none() {
                // unit is killed
                return;
            }
        }
    }

    fn reaction_fire(&mut self, unit_id: &UnitId, pos: &MapPos) {
        self.reaction_fire_internal(unit_id, pos, |i| i);
    }

    fn reaction_fire_move(
        &mut self,
        path: &MapPath,
        unit_id: &UnitId,
        move_mode: &MoveMode,
    ) {
        let len = path.nodes().len();
        for i in 1 .. len {
            let pos = &path.nodes()[i].pos;
            if self.reaction_fire_check(unit_id, pos) {
                let mut new_nodes = path.nodes().clone();
                new_nodes.truncate(i + 1);
                self.do_core_event(&CoreEvent::Move {
                    unit_id: unit_id.clone(),
                    path: MapPath::new(new_nodes),
                    mode: move_mode.clone(),
                });
                self.reaction_fire_internal(unit_id, pos, |attack_info| {
                    AttackInfo {
                        remove_move_points: *move_mode == MoveMode::Fast,
                        .. attack_info
                    }
                });
                return;
            }
        }
        self.do_core_event(&CoreEvent::Move {
            unit_id: unit_id.clone(),
            path: path.clone(),
            mode: move_mode.clone(),
        });
    }

    fn simulation_step(&mut self, command: Command) {
        match command {
            Command::EndTurn => {
                let old_id = self.current_player_id.id;
                let max_id = self.players.len() as ZInt;
                let new_id = if old_id + 1 == max_id {
                    0
                } else {
                    old_id + 1
                };
                self.do_core_event(&CoreEvent::EndTurn {
                    old_id: PlayerId{id: old_id},
                    new_id: PlayerId{id: new_id},
                });
            },
            Command::CreateUnit{pos} => {
                let event = CoreEvent::CreateUnit {
                    unit_info: UnitInfo {
                        unit_id: self.get_new_unit_id(),
                        pos: pos,
                        type_id: self.db.unit_type_id("soldier"),
                        player_id: self.current_player_id.clone(),
                        passanger_id: None,
                    },
                };
                self.do_core_event(&event);
            },
            Command::Move{ref unit_id, ref path, ref mode} => {
                // TODO: do some checks?
                self.reaction_fire_move(path, unit_id, mode);
            },
            Command::AttackUnit{ref attacker_id, ref defender_id} => {
                // TODO: do some checks?
                let defender_pos = self.state.unit(&defender_id).pos.clone();
                let event = self.command_attack_unit_to_event(
                    attacker_id, defender_id, &defender_pos, &FireMode::Active);
                if let Some(ref event) = event {
                    self.do_core_event(event);
                    let is_target_alive = self.state.units().get(&defender_id).is_some();
                    if is_target_alive {
                        let pos = &self.state.unit(&attacker_id).pos.clone();
                        self.reaction_fire(&attacker_id, pos);
                    }
                }
            },
            Command::LoadUnit{transporter_id, passanger_id} => {
                self.do_core_event(&CoreEvent::LoadUnit {
                    transporter_id: transporter_id,
                    passanger_id: passanger_id,
                });
            },
            Command::UnloadUnit{transporter_id, passanger_id, pos} => {
                let event = {
                    let passanger = self.state.unit(&passanger_id);
                    CoreEvent::UnloadUnit {
                        transporter_id: transporter_id,
                        unit_info: UnitInfo {
                            pos: pos.clone(),
                            .. unit_to_info(passanger)
                        },
                    }
                };
                self.do_core_event(&event);
                self.reaction_fire(&passanger_id, &pos);
            },
            Command::SetReactionFireMode{unit_id, mode} => {
                self.do_core_event(&CoreEvent::SetReactionFireMode {
                    unit_id: unit_id,
                    mode: mode,
                });
            },
        };
    }

    pub fn do_command(&mut self, command: Command) {
        self.simulation_step(command);
    }

    fn do_ai(&mut self) {
        loop {
            while let Some(event) = self.get_event() {
                self.ai.apply_event(&self.db, &event);
            }
            let command = self.ai.get_command(&self.db);
            self.do_command(command.clone());
            if let Command::EndTurn = command {
                return;
            }
        }
    }

    fn handle_end_turn_event(&mut self, old_id: &PlayerId, new_id: &PlayerId) {
        for player in &self.players {
            if player.id == *new_id {
                if self.current_player_id == *old_id {
                    self.current_player_id = player.id.clone();
                }
                break;
            }
        }
        if self.player().is_ai && *new_id == *self.player_id() {
            self.do_ai();
        }
    }

    fn do_core_event(&mut self, event: &CoreEvent) {
        if let CoreEvent::EndTurn{ref old_id, ref new_id} = *event {
            self.handle_end_turn_event(old_id, new_id);
        }
        self.state.apply_event(&self.db, &event);
        for player in &self.players {
            let (filtered_events, active_unit_ids) = filter::filter_events(
                &self.db,
                &self.state,
                &player.id,
                &self.players_info[&player.id].fow,
                &event,
            );
            let mut i = self.players_info.get_mut(&player.id)
                .expect("core: Can`t get player`s info");
            for event in filtered_events {
                i.fow.apply_event(&self.db, &self.state, &event);
                i.events.push_back(event);
                let new_visible_enemies = filter::get_visible_enemies(
                    &self.db,
                    &self.state,
                    &i.fow,
                    &player.id,
                );
                let show_hide_events = filter::show_or_hide_passive_enemies(
                    self.state.units(),
                    &active_unit_ids,
                    &i.visible_enemies,
                    &new_visible_enemies,
                );
                i.events.extend(show_hide_events);
                i.visible_enemies = new_visible_enemies;
            }
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
