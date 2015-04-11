// See LICENSE file for copyright and license details.

use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet, LinkedList};
use cgmath::{Vector2};
use common::types::{Size2, ZInt, UnitId, PlayerId, MapPos};
use common::misc::{clamp};
use internal_state::{InternalState};
use map::{Map, Terrain, distance};
use pathfinder::{MapPath, PathNode, MoveCost};
use command::{Command};
use unit::{Unit, UnitTypeId, WeaponTypeId, WeaponType, UnitClass};
use object::{ObjectTypes};
use player::{Player};
use ai::{Ai};
use fow::{Fow};
use fov::{fov_closure};

#[derive(Clone)]
pub enum FireMode {
    Active,
    Reactive,
}

#[derive(Clone)]
pub enum CoreEvent {
    Move{unit_id: UnitId, path: MapPath},
    EndTurn{old_id: PlayerId, new_id: PlayerId},
    CreateUnit {
        unit_id: UnitId,
        pos: MapPos,
        type_id: UnitTypeId,
        player_id: PlayerId,
    },
    AttackUnit {
        attacker_id: UnitId,
        defender_id: UnitId,
        mode: FireMode,
        killed: ZInt,
    },
    ShowUnit {
        unit_id: UnitId,
        pos: MapPos,
        type_id: UnitTypeId,
        player_id: PlayerId,
    },
    HideUnit {
        unit_id: UnitId,
    },
}

fn is_target_dead(state: &InternalState, event: &CoreEvent) -> bool {
    match event {
        &CoreEvent::AttackUnit{ref defender_id, ref killed, ..} => {
            state.units[defender_id].count - *killed <= 0
        },
        _ => panic!("wrong event type"),
    }
}

fn get_visible_enemies(
    fow: &Fow,
    units: &HashMap<UnitId, Unit>,
    player_id: &PlayerId,
) -> HashSet<UnitId> {
    let mut visible_enemies = HashSet::new();
    for (id, unit) in units.iter() {
        if unit.player_id != *player_id && fow.is_visible(&unit.pos) {
            visible_enemies.insert(id.clone());
        }
    }
    visible_enemies
}

fn show_or_hide_passive_enemies(
    units: &HashMap<UnitId, Unit>,
    active_unit_ids: &HashSet<UnitId>,
    old: &HashSet<UnitId>,
    new: &HashSet<UnitId>,
) -> LinkedList<CoreEvent> {
    let mut events = LinkedList::new();
    let located_units = new.difference(old);
    for id in located_units {
        if active_unit_ids.contains(id) {
            continue;
        }
        let unit = units.get(&id).expect("Can`t find unit");
        events.push_back(CoreEvent::ShowUnit {
            unit_id: id.clone(),
            pos: unit.pos.clone(),
            type_id: unit.type_id.clone(),
            player_id: unit.player_id.clone(),
        });
    }
    let lost_units = old.difference(new);
    for id in lost_units {
        if active_unit_ids.contains(id) {
            continue;
        }
        events.push_back(CoreEvent::HideUnit{unit_id: id.clone()});
    }
    events
}

pub struct PlayerInfo {
    events: LinkedList<CoreEvent>,
    fow: Fow,
    visible_enemies: HashSet<UnitId>,
}

pub struct Core {
    state: InternalState,
    players: Vec<Player>,
    current_player_id: PlayerId,
    core_event_list: Vec<CoreEvent>,
    pub object_types: ObjectTypes, // TODO: remove 'pub'
    ai: Ai,
    players_info: HashMap<PlayerId, PlayerInfo>,
}

fn get_players_list() -> Vec<Player> {
    vec!(
        Player{id: PlayerId{id: 0}, is_ai: false},
        // Player{id: PlayerId{id: 1}, is_ai: true},
        Player{id: PlayerId{id: 1}, is_ai: false},
    )
}

fn get_player_info_lists(map_size: &Size2<ZInt>) -> HashMap<PlayerId, PlayerInfo> {
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

pub fn los(map: &Map<Terrain>, from: &MapPos, to: &MapPos) -> bool {
    // TODO: profile and optimize!
    let mut v = false;
    fov_closure(map, &mut |p| if *p == *to { v = true }, from);
    v
}

impl Core {
    pub fn new() -> Core {
        let map_size = Size2{w: 10, h: 8};
        let mut core = Core {
            state: InternalState::new(&map_size),
            players: get_players_list(),
            current_player_id: PlayerId{id: 0},
            core_event_list: Vec::new(),
            object_types: ObjectTypes::new(),
            ai: Ai::new(&PlayerId{id:1}, &map_size),
            players_info: get_player_info_lists(&map_size),
        };
        core.get_units();
        core
    }

    pub fn object_types(&self) -> &ObjectTypes {
        &self.object_types
    }

    // TODO: Move to scenario.json
    fn get_units(&mut self) {
        let tank_id = self.object_types.get_unit_type_id("tank");
        let soldier_id = self.object_types.get_unit_type_id("soldier");
        let p_id_0 = PlayerId{id: 0};
        let p_id_1 = PlayerId{id: 1};
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 1}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 2}}, &soldier_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 3}}, &soldier_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 4}}, &soldier_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 5}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 0, y: 6}}, &tank_id, &p_id_0);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 1}}, &tank_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 2}}, &soldier_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 3}}, &soldier_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 4}}, &soldier_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 5}}, &tank_id, &p_id_1);
        self.add_unit(&MapPos{v: Vector2{x: 9, y: 6}}, &tank_id, &p_id_1);
    }

    fn get_new_unit_id(&self) -> UnitId {
        // TODO: check max id
        let id = match self.state.units.keys().max_by(|&n| n) {
            Some(n) => n.id + 1,
            None => 0,
        };
        UnitId{id: id}
    }

    fn add_unit(&mut self, pos: &MapPos, type_id: &UnitTypeId, player_id: &PlayerId) {
        let event = CoreEvent::CreateUnit{
            unit_id: self.get_new_unit_id(),
            pos: pos.clone(),
            type_id: type_id.clone(),
            player_id: player_id.clone(),
        };
        self.do_core_event(event);
    }

    pub fn map_size(&self) -> &Size2<ZInt> {
        self.state.map.size()
    }

    pub fn get_weapon_type(&self, weapon_type_id: &WeaponTypeId) -> &WeaponType {
        self.object_types.get_weapon_type(weapon_type_id)
    }

    fn get_killed_count(&self, attacker: &Unit, defender: &Unit) -> ZInt {
        let hit = self.hit_test(attacker, defender);
        if !hit {
            return 0;
        }
        let defender_type = self.object_types.get_unit_type(&defender.type_id);
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
        let attacker_type = self.object_types.get_unit_type(&attacker.type_id);
        let defender_type = self.object_types.get_unit_type(&defender.type_id);
        let weapon_type = self.get_weapon_type(&attacker_type.weapon_type_id);
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

    pub fn los(&self, from: &MapPos, to: &MapPos) -> bool {
        los(&self.state.map, from, to)
    }

    fn command_attack_unit_to_event(
        &self,
        attacker_id: UnitId,
        defender_id: UnitId,
        pos: &MapPos, // TODO: get pos from unit
        fire_mode: FireMode,
        // (apply coreevent to state after adding CoreEvent)
    ) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        let attacker = &self.state.units[&attacker_id];
        let defender = &self.state.units[&defender_id];
        let attacker_type = self.object_types.get_unit_type(&attacker.type_id);
        let weapon_type = self.get_weapon_type(&attacker_type.weapon_type_id);
        if distance(&attacker.pos, pos) > weapon_type.max_distance {
            return events;
        }
        if !self.los(&attacker.pos, pos) {
            return events;
        }
        let killed = self.get_killed_count(attacker, defender);
        events.push(CoreEvent::AttackUnit {
            attacker_id: attacker_id,
            defender_id: defender_id,
            killed: killed,
            mode: fire_mode,
        });
        events
    }

    fn reaction_fire(&self, unit_id: &UnitId, pos: &MapPos) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        for (_, enemy_unit) in self.state.units.iter() {
            // TODO: check if unit is still alive
            if enemy_unit.player_id == self.current_player_id {
                continue;
            }
            if enemy_unit.attack_points <= 0 {
                continue;
            }
            let fow = &self.players_info[&enemy_unit.player_id].fow;
            if !fow.is_visible(pos) {
                continue;
            }
            let max_distance = self.object_types
                .get_unit_max_attack_dist(enemy_unit);
            if distance(&enemy_unit.pos, pos) > max_distance {
                continue;
            }
            if !self.los(&enemy_unit.pos, pos) {
                continue;
            }
            let e = self.command_attack_unit_to_event(
                enemy_unit.id.clone(), unit_id.clone(), pos, FireMode::Reactive);
            events.push_all(&e);
            if e.is_empty() {
                continue;
            }
            if is_target_dead(&self.state, &e[0]) {
                break;
            }
        }
        events
    }

    fn reaction_fire_move(&self, path: &MapPath, unit_id: &UnitId) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        let len = path.nodes().len();
        for i in 1 .. len {
            let pos = &path.nodes()[i].pos;
            let e = self.reaction_fire(unit_id, pos);
            if !e.is_empty() {
                let mut new_nodes = path.nodes().clone();
                new_nodes.truncate(i + 1);
                events.push(CoreEvent::Move {
                    unit_id: unit_id.clone(),
                    path: MapPath::new(new_nodes),
                });
                events.push_all(&e);
                break;
            }
        }
        events
    }

    // TODO: rename: simulation_step?
    // Apply events immediately after adding event to array.
    fn command_to_event(&self, command: Command) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        match command {
            Command::EndTurn => {
                let old_id = self.current_player_id.id;
                let max_id = self.players.len() as ZInt;
                let new_id = if old_id + 1 == max_id {
                    0
                } else {
                    old_id + 1
                };
                events.push(CoreEvent::EndTurn {
                    old_id: PlayerId{id: old_id},
                    new_id: PlayerId{id: new_id},
                });
            },
            Command::CreateUnit{pos} => {
                events.push(CoreEvent::CreateUnit {
                    unit_id: self.get_new_unit_id(),
                    pos: pos,
                    type_id: self.object_types.get_unit_type_id("soldier"),
                    player_id: self.current_player_id.clone(),
                });
            },
            Command::Move{ref unit_id, ref path} => {
                // TODO: do some checks?
                let e = self.reaction_fire_move(path, unit_id);
                if e.is_empty() {
                    events.push(CoreEvent::Move {
                        unit_id: unit_id.clone(),
                        path: path.clone(),
                    });
                } else {
                    events.push_all(&e);
                }
            },
            Command::AttackUnit{attacker_id, defender_id} => {
                // TODO: do some checks?
                let defender_pos = &self.state.units[&defender_id].pos;
                let e = self.command_attack_unit_to_event(
                    attacker_id.clone(), defender_id, defender_pos, FireMode::Active);
                events.push_all(&e);
                if !e.is_empty() && !is_target_dead(&self.state, &e[0]) {
                    let pos = &self.state.units[&attacker_id].pos;
                    events.push_all(&self.reaction_fire(&attacker_id, pos));
                }
            },
        };
        events
    }

    pub fn do_command(&mut self, command: Command) {
        let events = self.command_to_event(command);
        if events.is_empty() {
            println!("BAD COMMAND!");
        }
        for event in events.into_iter() {
            self.do_core_event(event);
        }
    }

    fn do_core_event(&mut self, core_event: CoreEvent) {
        self.core_event_list.push(core_event);
        self.make_player_events();
    }

    fn do_ai(&mut self) {
        loop {
            while let Some(event) = self.get_event() {
                self.ai.apply_event(&self.object_types, &event);
            }
            let command = self.ai.get_command(&self.object_types);
            self.do_command(command.clone());
            // TODO: use 'if let'
            match command {
                Command::EndTurn => return,
                _ => {},
            }
        }
    }

    fn handle_end_turn_event(&mut self, old_id: &PlayerId, new_id: &PlayerId) {
        for player in self.players.iter() {
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

    fn create_show_unit_event(&self, unit: &Unit) -> CoreEvent {
        CoreEvent::ShowUnit {
            unit_id: unit.id.clone(),
            pos: unit.pos.clone(),
            type_id: unit.type_id.clone(),
            player_id: unit.player_id.clone(),
        }
    }

    // TODO: add unit/functional tests
    fn filter_events(
        &self,
        player_id: &PlayerId,
        event: &CoreEvent,
    ) -> (Vec<CoreEvent>, HashSet<UnitId>) {
        let mut active_unit_ids = HashSet::new();
        let mut events = vec![];
        let fow = &self.players_info[player_id].fow;
        match event {
            &CoreEvent::Move{ref unit_id, ref path, ..} => {
                let unit = self.state.units.get(unit_id)
                    .expect("Can`t find moving unit");
                if unit.player_id == *player_id {
                    events.push(event.clone())
                } else {
                    let len = path.nodes().len();
                    for i in 1 .. len {
                        let prev_node = path.nodes()[i - 1].clone();
                        let next_node = path.nodes()[i].clone();
                        let prev_vis = fow.is_visible(&prev_node.pos);
                        let next_vis = fow.is_visible(&next_node.pos);
                        active_unit_ids.insert(unit.id.clone());
                        if !prev_vis && next_vis {
                            events.push(CoreEvent::ShowUnit {
                                unit_id: unit.id.clone(),
                                pos: prev_node.pos.clone(),
                                type_id: unit.type_id.clone(),
                                player_id: unit.player_id.clone(),
                            });
                        }
                        if prev_vis || next_vis {
                            // TODO: concatenate move events
                            let new_nodes = vec![
                                // TODO: fix move cost hack
                                PathNode{cost: MoveCost{n: 0}, pos: prev_node.pos.clone()},
                                PathNode{cost: MoveCost{n: 0}, pos: next_node.pos.clone()},
                            ];
                            events.push(CoreEvent::Move {
                                unit_id: unit.id.clone(),
                                path: MapPath::new(new_nodes),
                            });
                        }
                        if prev_vis && !next_vis {
                            events.push(CoreEvent::HideUnit {
                                unit_id: unit.id.clone(),
                            });
                        }
                    }
                }
            },
            &CoreEvent::EndTurn{..} => {
                events.push(event.clone());
            },
            &CoreEvent::CreateUnit {
                ref pos,
                ref unit_id,
                player_id: ref new_unit_player_id,
                ..
            } => {
                if *player_id == *new_unit_player_id || fow.is_visible(pos) {
                    events.push(event.clone());
                    active_unit_ids.insert(unit_id.clone());
                }
            },
            &CoreEvent::AttackUnit{ref attacker_id, ref defender_id, ..} => {
                let attacker = self.state.units.get(attacker_id)
                    .expect("Can`t find attacker");
                if !fow.is_visible(&attacker.pos) {
                    events.push(self.create_show_unit_event(&attacker));
                }
                // if defender is not dead...
                if let Some(defender) = self.state.units.get(defender_id) {
                    if !fow.is_visible(&defender.pos) {
                        events.push(self.create_show_unit_event(&defender));
                    }
                }
                active_unit_ids.insert(attacker_id.clone());
                active_unit_ids.insert(defender_id.clone());
                events.push(event.clone());
            },
            &CoreEvent::ShowUnit{..} => panic!(),
            &CoreEvent::HideUnit{..} => panic!(),
        }
        (events, active_unit_ids)
    }

    fn make_player_events(&mut self) {
        while !self.core_event_list.is_empty() {
            let event = self.core_event_list.pop()
                .expect("core_event_list is empty");
            if let CoreEvent::EndTurn{ref old_id, ref new_id} = event {
                self.handle_end_turn_event(old_id, new_id);
            }
            self.state.apply_event(&self.object_types, &event);
            for player in self.players.iter() {
                let (filtered_events, active_unit_ids)
                    = self.filter_events(&player.id, &event);
                for event in filtered_events {
                    let mut i = self.players_info.get_mut(&player.id)
                        .expect("core: Can`t get player`s info");
                    i.fow.apply_event(&self.state, &event);
                    i.events.push_back(event);
                    let new_visible_enemies = get_visible_enemies(
                        &i.fow, &self.state.units, &player.id);
                    let mut show_hide_events = show_or_hide_passive_enemies(
                        &self.state.units,
                        &active_unit_ids,
                        &i.visible_enemies,
                        &new_visible_enemies,
                    );
                    i.events.append(&mut show_hide_events);
                    i.visible_enemies = new_visible_enemies;
                }
            }
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
