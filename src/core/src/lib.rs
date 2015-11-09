// See LICENSE file for copyright and license details.

extern crate num;
extern crate cgmath;
extern crate rand;
extern crate common;

pub mod geom;
pub mod map;
pub mod db;
pub mod unit;
pub mod dir;
pub mod game_state;
pub mod state;
pub mod pathfinder;

mod ai;
mod player;
mod fov;
mod fow;
mod internal_state;
mod filter;

use rand::{thread_rng, Rng};
use std::cmp;
use std::collections::{HashMap, HashSet, LinkedList};
use cgmath::{Vector2};
use common::types::{Size2, ZInt, UnitId, PlayerId, MapPos};
use common::misc::{clamp};
use internal_state::{InternalState};
use game_state::{GameState};
use state::{State};
use map::{Map, Terrain, distance};
use pathfinder::{MapPath};
use unit::{Unit, UnitType, UnitTypeId, UnitClass};
use db::{Db};
use player::{Player};
use ai::{Ai};
use fow::{Fow};
use fov::{fov};

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

fn find_transporter_id(db: &Db, units: &[&Unit]) -> Option<UnitId> {
    let mut transporter_id = None;
    for unit in units {
        let unit_type = db.unit_type(&unit.type_id);
        if unit_type.is_transporter {
            transporter_id = Some(unit.id.clone());
        }
    }
    transporter_id
}

pub fn get_unit_id_at(db: &Db, state: &GameState, pos: &MapPos) -> Option<UnitId> {
    let units_at = state.units_at(pos);
    if units_at.len() == 1 {
        let unit_id = units_at[0].id.clone();
        Some(unit_id)
    } else if units_at.len() > 1 {
        let transporter_id = find_transporter_id(db, &units_at)
            .expect("Multiple units in tile, but no transporter");
        for unit in &units_at {
            if unit.id == transporter_id {
                continue;
            }
            let transporter = state.unit(&transporter_id);
            if let Some(ref passanger_id) = transporter.passanger_id {
                if *passanger_id != unit.id {
                    panic!("Non-passanger unit in multiunit tile");
                }
            } else {
                panic!("Multiple units in tile, but transporter is empty");
            }
        }
        Some(transporter_id)
    } else {
        None
    }
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

#[derive(Debug)]
pub enum CommandError {
    TileIsOccupied,
    NotEnoughMovePoints,
    NotEnoughAttackPoints,
    NotEnoughReactiveAttackPoints,
    BadMorale,
    OutOfRange,
    TooClose,
    NoLos,
    BadTransporterClass,
    BadPassangerClass,
    TransporterIsNotEmpty,
    TransporterIsEmpty,
    TransporterIsTooFarAway,
    PassangerHasNotEnoughMovePoints,
    UnloadDistanceIsTooBig,
    DestinationTileIsNotEmpty,
    BadUnitId,
    BadTransporterId,
    BadPassangerId,
    BadAttackerId,
    BadDefenderId,
    BadPath,
}

impl CommandError {
    fn to_str(&self) -> &str {
        match *self {
            CommandError::TileIsOccupied => "Tile is occupied",
            CommandError::NotEnoughMovePoints => "Not enough move points",
            CommandError::NotEnoughAttackPoints => "No attack points",
            CommandError::NotEnoughReactiveAttackPoints => "No reactive attack points",
            CommandError::BadMorale => "Can`t attack when suppresset",
            CommandError::OutOfRange => "Out of range",
            CommandError::TooClose => "Too close",
            CommandError::NoLos => "No Line of Sight",
            CommandError::BadTransporterClass => "Bad transporter class",
            CommandError::BadPassangerClass => "Bad passanger class",
            CommandError::TransporterIsNotEmpty => "Transporter is not empty",
            CommandError::TransporterIsEmpty => "Transporter is empty",
            CommandError::TransporterIsTooFarAway => "Transporter is too far away",
            CommandError::PassangerHasNotEnoughMovePoints => "Passanger has not enough move points",
            CommandError::UnloadDistanceIsTooBig => "Unload pos it too far away",
            CommandError::DestinationTileIsNotEmpty => "Destination tile is not empty",
            CommandError::BadUnitId => "Bad unit id",
            CommandError::BadTransporterId => "Bad transporter id",
            CommandError::BadPassangerId => "Bad passanger id",
            CommandError::BadAttackerId => "Bad attacker id",
            CommandError::BadDefenderId => "Bad defender id",
            CommandError::BadPath => "Bad path",
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl std::error::Error for CommandError {
    fn description(&self) -> &str {
        self.to_str()
    }
}

fn check_attack_at<'a, S: State<'a>>(
    db: &Db,
    state: &'a S,
    attacker_id: &UnitId,
    defender_id: &UnitId,
    defender_pos: &MapPos,
    fire_mode: &FireMode,
) -> Result<(), CommandError> {
    if state.units().get(attacker_id).is_none() {
        return Err(CommandError::BadAttackerId);
    }
    if state.units().get(defender_id).is_none() {
        return Err(CommandError::BadDefenderId);
    }
    let attacker = state.unit(attacker_id);
    let reactive_attack_points = attacker.reactive_attack_points.unwrap();
    match *fire_mode {
        FireMode::Active => if attacker.attack_points <= 0 {
            return Err(CommandError::NotEnoughAttackPoints);
        },
        FireMode::Reactive => if reactive_attack_points <= 0 {
            return Err(CommandError::NotEnoughReactiveAttackPoints);
        },
    }
    // TODO: magic number
    if attacker.morale < 50 {
        return Err(CommandError::BadMorale);
    }
    let attacker_type = db.unit_type(&attacker.type_id);
    let weapon_type = db.weapon_type(&attacker_type.weapon_type_id);
    if distance(&attacker.pos, defender_pos) > weapon_type.max_distance {
        return Err(CommandError::OutOfRange);
    }
    if distance(&attacker.pos, defender_pos) < weapon_type.min_distance {
        return Err(CommandError::TooClose);
    }
    if !los(state.map(), attacker_type, &attacker.pos, defender_pos) {
        return Err(CommandError::NoLos);
    }
    Ok(())
}

fn check_attack<'a, S: State<'a>>(
    db: &Db,
    state: &'a S,
    attacker_id: &UnitId,
    defender_id: &UnitId,
) -> Result<(), CommandError> {
    let defender = state.unit(defender_id);
    check_attack_at(db, state, attacker_id, defender_id, &defender.pos, &FireMode::Active)
}

pub fn check_command<'a, S: State<'a>>(
    db: &Db,
    state: &'a S,
    command: &Command,
) -> Result<(), CommandError> {
    match command {
        &Command::EndTurn => Ok(()),
        &Command::CreateUnit{ref pos} => {
            if state.is_tile_occupied(pos) {
                Err(CommandError::TileIsOccupied)
            } else {
                Ok(())
            }
        },
        &Command::Move{ref unit_id, ref path, ref mode} => {
            if state.units().get(unit_id).is_none() {
                return Err(CommandError::BadUnitId);
            }
            // unit stands at first pos
            for i in 1 .. path.nodes().len() {
                let pos = &path.nodes()[i].pos;
                if state.is_tile_occupied(pos) {
                    return Err(CommandError::BadPath);
                }
            }
            let unit = state.unit(&unit_id);
            let cost_modifier = match mode {
                &MoveMode::Fast => 1,
                &MoveMode::Hunt => 2,
            };
            // TODO: we believe that path's cost is correct. this is bad.
            let cost = path.total_cost().n * cost_modifier;
            if cost > unit.move_points {
                return Err(CommandError::NotEnoughMovePoints);
            }
            Ok(())
        },
        &Command::AttackUnit{ref attacker_id, ref defender_id} => {
            check_attack(db, state, attacker_id, defender_id)
        },
        &Command::LoadUnit{ref transporter_id, ref passanger_id} => {
            if state.units().get(transporter_id).is_none() {
                return Err(CommandError::BadTransporterId);
            }
            if state.units().get(passanger_id).is_none() {
                return Err(CommandError::BadPassangerId);
            }
            let passanger = state.unit(&passanger_id);
            let pos = passanger.pos.clone();
            let transporter = state.unit(&transporter_id);
            if !db.unit_type(&transporter.type_id).is_transporter {
                return Err(CommandError::BadTransporterClass);
            }
            match db.unit_type(&passanger.type_id).class {
                UnitClass::Infantry => {},
                _ => {
                    return Err(CommandError::BadPassangerClass);
                }
            }
            if transporter.passanger_id.is_some() {
                return Err(CommandError::TransporterIsNotEmpty);
            }
            if distance(&transporter.pos, &pos) > 1 {
                return Err(CommandError::TransporterIsTooFarAway);
            }
            // TODO: 0 -> real move cost of transport tile for passanger
            if passanger.move_points == 0 {
                return Err(CommandError::PassangerHasNotEnoughMovePoints);
            }
            Ok(())
        },
        &Command::UnloadUnit{ref transporter_id, ref passanger_id, ref pos} => {
            if state.units().get(transporter_id).is_none() {
                return Err(CommandError::BadTransporterId);
            }
            if state.units().get(passanger_id).is_none() {
                return Err(CommandError::BadPassangerId);
            }
            let transporter = state.unit(&transporter_id);
            if !db.unit_type(&transporter.type_id).is_transporter {
                return Err(CommandError::BadTransporterClass);
            }
            if distance(&transporter.pos, &pos) > 1 {
                return Err(CommandError::UnloadDistanceIsTooBig);
            }
            if let None = transporter.passanger_id {
                return Err(CommandError::TransporterIsEmpty);
            }
            if state.is_tile_occupied(pos) {
                return Err(CommandError::DestinationTileIsNotEmpty);
            }
            // TODO: check that tile is walkable for passanger
            Ok(())
        },
        &Command::SetReactionFireMode{ref unit_id, ..} => {
            if state.units().get(unit_id).is_none() {
                Err(CommandError::BadUnitId)
            } else {
                Ok(())
            }
        },
    }
}

#[derive(PartialEq, Eq)]
pub enum GameType {
    Hotseat,
    SingleVsAi,
}

impl Default for GameType {
    fn default() -> GameType {
        GameType::Hotseat
    }
}

#[derive(Default)]
pub struct Options {
    pub game_type: GameType,
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

fn get_players_list(game_type: &GameType) -> Vec<Player> {
    vec!(
        Player{id: PlayerId{id: 0}, is_ai: false},
        Player{id: PlayerId{id: 1}, is_ai: GameType::SingleVsAi == *game_type},
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
    pub fn new(options: &Options) -> Core {
        let map_size = Size2{w: 10, h: 8}; // TODO: read from config file
        let mut core = Core {
            state: InternalState::new(&map_size),
            players: get_players_list(&options.game_type),
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
            println!("Sorry, tile is occupied"); // TODO: ?
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
            let real = thread_rng().gen_range(-5, 5);
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

    fn command_attack_unit_to_event(
        &self,
        attacker_id: &UnitId,
        defender_id: &UnitId,
        defender_pos: &MapPos,
        fire_mode: &FireMode,
    ) -> Option<CoreEvent> {
        let attacker = self.state.unit(&attacker_id);
        let defender = self.state.unit(&defender_id);
        let check_attack_result = check_attack_at(
            &self.db,
            &self.state,
            attacker_id,
            defender_id,
            defender_pos,
            fire_mode,
        );
        if let Err(..) = check_attack_result {
            return None;
        }
        let killed = cmp::min(
            defender.count, self.get_killed_count(attacker, defender));
        let fow = &self.players_info[&defender.player_id].fow;
        let is_visible = fow.is_visible(
            &self.db, &self.state, attacker, &attacker.pos);
        let is_ambush = !is_visible && thread_rng().gen_range(1, 10) > 3; // TODO: remove magic
        let attack_info = AttackInfo {
            attacker_id: Some(attacker_id.clone()),
            defender_id: defender_id.clone(),
            killed: killed,
            mode: fire_mode.clone(),
            suppression: 10 + 20 * killed, // TODO: remove magic
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
        if let ReactionFireMode::HoldFire = attacker.reaction_fire_mode {
            return false;
        }
        let check_attack_result = check_attack_at(
            &self.db,
            &self.state,
            &attacker.id,
            &defender.id,
            defender_pos,
            &FireMode::Reactive,
        );
        check_attack_result.is_ok()
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
                let mut new_nodes = path.nodes().to_vec();
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
        if let Err(err) = check_command(&self.db, &self.state, &command) {
            println!("Bad command: {:?}", err);
            return;
        }
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
                    let attacker_pos = self.state.unit(&attacker_id).pos.clone();
                    self.reaction_fire(&attacker_id, &attacker_pos);
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
        if let CoreEvent::EndTurn{ref old_id, ref new_id} = *event {
            self.handle_end_turn_event(old_id, new_id);
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
