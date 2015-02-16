// See LICENSE file for copyright and license details.

use rand::{thread_rng, Rng};
use std::collections::HashMap;
use cgmath::{Vector2};
use core::types::{Size2, ZInt, UnitId, PlayerId, MapPos};
use core::game_state::GameState;
use core::map::{distance};
use core::pathfinder::{MapPath, Pathfinder};
use core::dir::{Dir};

#[derive(Clone)]
pub enum Command {
    Move{unit_id: UnitId, path: MapPath},
    EndTurn,
    CreateUnit{pos: MapPos},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
}

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
        killed: bool,
    },
}

pub struct Player {
    pub id: PlayerId,
    pub is_ai: bool,
}

#[derive(Clone)]
pub enum UnitClass {
    Infantry,
    Vehicle,
}

pub struct WeaponType {
    pub name: String,
    pub damage: ZInt,
    pub ap: ZInt,
    pub accuracy: ZInt,
    pub max_distance: ZInt,
}

#[derive(Clone)]
pub struct WeaponTypeId{pub id: ZInt}

#[derive(Clone)]
pub struct UnitType {
    pub name: String,
    pub class: UnitClass,
    pub count: ZInt,
    pub size: ZInt,
    pub armor: ZInt,
    pub toughness: ZInt,
    pub weapon_skill: ZInt,
    pub weapon_type_id: WeaponTypeId,
    pub move_points: ZInt,
    pub attack_points: ZInt,
}

#[derive(Clone)]
pub struct UnitTypeId{pub id: ZInt}

pub struct Unit {
    pub id: UnitId,
    pub pos: MapPos,
    pub player_id: PlayerId,
    pub type_id: UnitTypeId,
    pub move_points: ZInt,
    pub attack_points: ZInt,
}

// TODO: Rename?
pub struct ObjectTypes {
    unit_types: Vec<UnitType>,
    weapon_types: Vec<WeaponType>,
}

impl ObjectTypes {
    pub fn new() -> ObjectTypes {
        let mut object_types = ObjectTypes {
            unit_types: vec![],
            weapon_types: vec![],
        };
        object_types.get_weapon_types();
        object_types.get_unit_types();
        object_types
    }

    // TODO: read from json/toml config
    fn get_weapon_types(&mut self) {
        self.weapon_types.push(WeaponType {
            name: "cannon".to_string(),
            damage: 9,
            ap: 9,
            accuracy: 5,
            max_distance: 5,
        });
        self.weapon_types.push(WeaponType {
            name: "rifle".to_string(),
            damage: 2,
            ap: 1,
            accuracy: 5,
            max_distance: 3,
        });
    }

    // TODO: read from json/toml config
    fn get_unit_types(&mut self) {
        let cannon_id = self.get_weapon_type_id("cannon");
        let rifle_id = self.get_weapon_type_id("rifle");
        self.unit_types.push(UnitType {
            name: "tank".to_string(),
            class: UnitClass::Vehicle,
            size: 6,
            count: 1,
            armor: 11,
            toughness: 9,
            weapon_skill: 5,
            weapon_type_id: cannon_id,
            move_points: 5,
            attack_points: 2,
        });
        self.unit_types.push(UnitType {
            name: "soldier".to_string(),
            class: UnitClass::Infantry,
            size: 4,
            count: 4,
            armor: 1,
            toughness: 2,
            weapon_skill: 5,
            weapon_type_id: rifle_id,
            move_points: 3,
            attack_points: 2,
        });
    }

    fn get_unit_type_id_opt(&self, name: &str) -> Option<UnitTypeId> {
        for (id, unit_type) in self.unit_types.iter().enumerate() {
            if unit_type.name.as_slice() == name {
                return Some(UnitTypeId{id: id as ZInt});
            }
        }
        None
    }

    pub fn get_unit_type<'a>(&'a self, unit_type_id: &UnitTypeId) -> &'a UnitType {
        &self.unit_types[unit_type_id.id as usize]
    }

    fn get_unit_type_id(&self, name: &str) -> UnitTypeId {
        match self.get_unit_type_id_opt(name) {
            Some(id) => id,
            None => panic!("No unit type with name: \"{}\"", name),
        }
    }

    fn get_weapon_type_id(&self, name: &str) -> WeaponTypeId {
        for (id, weapon_type) in self.weapon_types.iter().enumerate() {
            if weapon_type.name.as_slice() == name {
                return WeaponTypeId{id: id as ZInt};
            }
        }
        panic!("No weapon type with name \"{}\"", name);
    }

    pub fn get_unit_max_attack_dist(&self, unit: &Unit) -> ZInt {
        let attacker_type = self.get_unit_type(&unit.type_id);
        let weapon_type = &self
            .weapon_types[attacker_type.weapon_type_id.id as usize];
        weapon_type.max_distance
    }
}

fn is_target_dead(event: &CoreEvent) -> bool {
    match event {
        &CoreEvent::AttackUnit{ref killed, ..} => *killed,
        _ => panic!("wrong event type"),
    }
}

struct Ai {
    id: PlayerId,
    state: GameState,
    pathfinder: Pathfinder,
}

impl Ai {
    fn new(id: &PlayerId, map_size: &Size2<ZInt>) -> Ai {
        Ai {
            id: id.clone(),
            state: GameState::new(map_size, Some(id)),
            pathfinder: Pathfinder::new(map_size),
        }
    }

    // TODO: move fill_map here
    fn get_best_pos(&self) -> Option<MapPos> {
        let mut best_pos = None;
        let mut best_cost: Option<ZInt> = None;
        for (_, enemy) in self.state.units.iter() {
            if enemy.player_id == self.id {
                continue;
            }
            for i in range(0, 6) {
                let dir = Dir::from_int(i);
                let destination = Dir::get_neighbour_pos(&enemy.pos, &dir);
                if !self.state.map.is_inboard(&destination) {
                    continue;
                }
                if self.state.is_tile_occupied(&destination) {
                    continue;
                }
                let path = match self.pathfinder.get_path(&destination) {
                    Some(path) => path,
                    None => continue,
                };
                let cost = path.total_cost().n;
                if best_cost.is_some() {
                    if best_cost.unwrap() > cost {
                        best_cost = Some(cost);
                        best_pos = Some(destination.clone());
                    }
                } else {
                    best_cost = Some(cost);
                    best_pos = Some(destination.clone());
                }
            }
        }
        best_pos
    }

    fn truncate_path(&self, path: MapPath, move_points: ZInt) -> MapPath {
        if path.total_cost().n <= move_points {
            return path;
        }
        let len = path.nodes().len();
        for i in range(1, len) {
            let (ref cost, _) = path.nodes()[i];
            if cost.n > move_points {
                let mut new_nodes = path.nodes().clone();
                new_nodes.truncate(i);
                return MapPath::new(new_nodes);
            }
        }
        assert!(false); // TODO
        return path;
    }

    fn is_close_to_enemies(&self, object_types: &ObjectTypes, unit: &Unit) -> bool {
        for (_, target) in self.state.units.iter() {
            if target.player_id == self.id {
                continue;
            }
            let max_distance = object_types.get_unit_max_attack_dist(unit);
            if distance(&unit.pos, &target.pos) <= max_distance {
                return true;
            }
        }
        false
    }

    fn get_command(&mut self, object_types: &ObjectTypes) -> Command {
        {
            for (_, unit) in self.state.units.iter() {
                if unit.player_id != self.id {
                    continue;
                }
                // println!("id: {}, ap: {}", unit.id.id, unit.attack_points);
                if unit.attack_points <= 0 {
                    continue;
                }
                for (_, target) in self.state.units.iter() {
                    if target.player_id == self.id {
                        continue;
                    }
                    let max_distance = object_types.get_unit_max_attack_dist(unit);
                    if distance(&unit.pos, &target.pos) > max_distance {
                        continue;
                    }
                    return Command::AttackUnit {
                        attacker_id: unit.id.clone(),
                        defender_id: target.id.clone(),
                    };
                }
            }
        }
        {
            for (_, unit) in self.state.units.iter() {
                if unit.player_id != self.id {
                    continue;
                }
                if self.is_close_to_enemies(object_types, unit) {
                    continue;
                }
                self.pathfinder.fill_map(object_types, &self.state, unit);
                let destination = match self.get_best_pos() {
                    Some(destination) => destination,
                    None => continue,
                };
                let path = match self.pathfinder.get_path(&destination) {
                    Some(path) => path,
                    None => continue,
                };
                if unit.move_points == 0 {
                    continue;
                }
                let path = self.truncate_path(path, unit.move_points);
                return Command::Move{unit_id: unit.id.clone(), path: path};
            }
        }
        return Command::EndTurn;
    }
}

pub struct Core {
    game_state: GameState,
    players: Vec<Player>,
    current_player_id: PlayerId,
    core_event_list: Vec<CoreEvent>,
    event_lists: HashMap<PlayerId, Vec<CoreEvent>>,
    pub object_types: ObjectTypes, // TODO: remove 'pub'
    ai: Ai,
}

fn get_event_lists() -> HashMap<PlayerId, Vec<CoreEvent>> {
    let mut map = HashMap::new();
    map.insert(PlayerId{id: 0}, Vec::new());
    map.insert(PlayerId{id: 1}, Vec::new());
    map
}

fn get_players_list() -> Vec<Player> {
    vec!(
        Player{id: PlayerId{id: 0}, is_ai: false},
        // Player{id: PlayerId{id: 1}, is_ai: true},
        Player{id: PlayerId{id: 1}, is_ai: false},
    )
}

impl Core {
    pub fn new() -> Core {
        let map_size = Size2{w: 10, h: 8};
        let mut core = Core {
            game_state: GameState::new(&map_size, None),
            players: get_players_list(),
            current_player_id: PlayerId{id: 0},
            core_event_list: Vec::new(),
            event_lists: get_event_lists(),
            object_types: ObjectTypes::new(),
            ai: Ai::new(&PlayerId{id:1}, &map_size),
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
        let id = match self.game_state.units.keys().max_by(|&n| n) {
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
        self.game_state.map.size()
    }

    fn get_unit<'a>(&'a self, id: &UnitId) -> &'a Unit {
        match self.game_state.units.get(id) {
            Some(unit) => unit,
            None => panic!("No unit with id = {}", id.id),
        }
    }

    pub fn get_weapon_type(&self, weapon_type_id: &WeaponTypeId) -> &WeaponType {
        &self.object_types.weapon_types[weapon_type_id.id as usize]
    }

    fn hit_test(&self, attacker_id: &UnitId, defender_id: &UnitId) -> bool {
        fn test(needed: ZInt) -> bool {
            let real = thread_rng().gen_range(-5i32, 5i32);
            let result = real < needed;
            println!("real:{} < needed:{} = {}", real, needed, result);
            result
        }
        println!("");
        let attacker = self.get_unit(attacker_id);
        let defender = self.get_unit(defender_id);
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
        println!("hit_test = {}, pierce_test = {}, wound_test_v = {}",
            hit_test_v, pierce_test_v, wound_test_v);
        print!("hit test: ");
        if !test(hit_test_v) {
            return false;
        }
        print!("pierce test: ");
        if !test(pierce_test_v) {
            return false;
        }
        print!("wound test: ");
        if !test(wound_test_v) {
            return false;
        }
        println!("HIT!");
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
        let list = self.event_lists.get_mut(&self.current_player_id).unwrap();
        if list.len() == 0 {
            None
        } else {
            Some(list.remove(0))
        }
    }

    fn command_attack_unit_to_event(
        &self,
        attacker_id: UnitId,
        defender_id: UnitId,
        pos: &MapPos, // TODO: get pos from unit
        fire_mode: FireMode,
        // (apply coreevent to state after adding CoreEvent)
    ) -> Vec<CoreEvent> {
        let attacker = &self.game_state.units[attacker_id];
        // let defender = &self.game_state.units[defender_id];
        let attacker_type = self.object_types.get_unit_type(&attacker.type_id);
        let weapon_type = self.get_weapon_type(&attacker_type.weapon_type_id);
        // if distance(&attacker.pos, &defender.pos) < weapon_type.max_distance {
        if distance(&attacker.pos, pos) <= weapon_type.max_distance {
            let hit = self.hit_test(&attacker_id, &defender_id);
            vec![CoreEvent::AttackUnit {
                attacker_id: attacker_id,
                defender_id: defender_id,
                killed: hit,
                mode: fire_mode,
            }]
        } else {
            Vec::new()
        }
    }

    fn reaction_fire(&self, unit_id: &UnitId, pos: &MapPos) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        for (_, enemy_unit) in self.game_state.units.iter() {
            // TODO: check if unit is still alive
            if enemy_unit.player_id == self.current_player_id {
                continue;
            }
            if enemy_unit.attack_points <= 0 {
                continue;
            }
            let max_distance = self.object_types
                .get_unit_max_attack_dist(enemy_unit);
            if distance(&enemy_unit.pos, pos) > max_distance {
                continue;
            }
            let e = self.command_attack_unit_to_event(
                enemy_unit.id.clone(), unit_id.clone(), pos, FireMode::Reactive);
            events.push_all(e.as_slice());
            if e.is_empty() {
                continue;
            }
            if is_target_dead(&e[0]) {
                break;
            }
        }
        events
    }

    fn reaction_fire_move(&self, path: &MapPath, unit_id: &UnitId) -> Vec<CoreEvent> {
        let mut events = Vec::new();
        let len = path.nodes().len();
        'path_loop: for i in range(1, len) {
            let (_, ref pos) = path.nodes()[i];
            let e = self.reaction_fire(unit_id, pos);
            if !e.is_empty() {
                let mut new_nodes = path.nodes().clone();
                new_nodes.truncate(i + 1);
                events.push(CoreEvent::Move {
                    unit_id: unit_id.clone(),
                    path: MapPath::new(new_nodes),
                });
                events.push_all(e.as_slice());
                break 'path_loop;
            }
        }
        events
    }

    // TODO: rename: simulation_step?
    // Apply events immidietly after adding event to array.
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
                    events.push_all(e.as_slice());
                }
            },
            Command::AttackUnit{attacker_id, defender_id} => {
                // TODO: do some checks?
                let defender_pos = &self.game_state.units[defender_id].pos;
                let e = self.command_attack_unit_to_event(
                    attacker_id.clone(), defender_id, defender_pos, FireMode::Active);
                events.push_all(e.as_slice());
                if !e.is_empty() && !is_target_dead(&e[0]) {
                    let pos = &self.game_state.units[attacker_id].pos;
                    events.push_all(
                        self.reaction_fire(&attacker_id, pos).as_slice());
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
        self.make_events();
    }

    fn do_ai(&mut self) {
        loop {
            while let Some(event) = self.get_event() {
                self.ai.state.apply_event(&self.object_types, &event);
            }
            let command = self.ai.get_command(&self.object_types);
            self.do_command(command.clone());
            match command {
                Command::EndTurn => return,
                _ => {},
            }
        }
    }

    fn apply_event(&mut self, event: &CoreEvent) {
        match *event {
            CoreEvent::EndTurn{ref old_id, ref new_id} => {
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
            },
            _ => {},
        };
    }

    fn make_events(&mut self) {
        while self.core_event_list.len() != 0 {
            let event = self.core_event_list.pop().unwrap();
            self.apply_event(&event);
            self.game_state.apply_event(&self.object_types, &event);
            for player in self.players.iter() {
                let event_list = self.event_lists.get_mut(&player.id).unwrap();
                // TODO: per player event filter
                event_list.push(event.clone());
            }
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
