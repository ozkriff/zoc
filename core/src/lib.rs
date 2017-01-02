extern crate cgmath;
extern crate rand;

pub mod geom;
pub mod map;
pub mod db;
pub mod unit;
pub mod dir;
pub mod game_state;
pub mod pathfinder;
pub mod misc;
pub mod types;
pub mod check;

mod ai;
mod fov;
mod fow;
mod filter;

use std::{cmp, fmt};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc};
use rand::{thread_rng, Rng};
use cgmath::{Vector2};
use types::{Size2};
use misc::{clamp};
use game_state::{State, ObjectsAtIter};
use map::{Map, Terrain};
use pathfinder::{tile_cost};
use unit::{Unit, UnitTypeId};
use db::{Db};
use ai::{Ai};
use fow::{Fow};
use dir::{Dir};
use check::{check_command, check_attack};

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct HitChance{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MovePoints{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttackPoints{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Distance{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReinforcementPoints{pub n: i32}

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerId{pub id: i32}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct UnitId{pub id: i32}

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct SectorId{pub id: i32}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MapPos{pub v: Vector2<i32>}

impl fmt::Display for MapPos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MapPos({}, {})", self.v.x, self.v.y)
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SlotId {
    Id(u8),
    WholeTile,
    TwoTiles(Dir),
    Air,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct ExactPos {
    pub map_pos: MapPos,
    pub slot_id: SlotId,
}

#[derive(Clone, Copy, Debug)]
pub struct ExactPosIter {
    p: ExactPos,
    i: u8,
}

impl ExactPos {
    pub fn map_pos_iter(self) -> ExactPosIter {
        ExactPosIter {
            p: self,
            i: 0,
        }
    }
}

impl Iterator for ExactPosIter {
    type Item = MapPos;

    fn next(&mut self) -> Option<Self::Item> {
        let next_pos = match self.p.slot_id {
            SlotId::Air | SlotId::Id(_) | SlotId::WholeTile => {
                if self.i == 0 {
                    Some(self.p.map_pos)
                } else {
                    None
                }
            }
            SlotId::TwoTiles(dir) => {
                if self.i == 0 {
                    Some(self.p.map_pos)
                } else if self.i == 1 {
                    Some(Dir::get_neighbour_pos(self.p.map_pos, dir))
                } else {
                    None
                }
            }
        };
        self.i += 1;
        next_pos
    }
}

fn check_sectors(db: &Db, state: &State) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    for (&sector_id, sector) in state.sectors() {
        let mut claimers = HashSet::new();
        for &pos in &sector.positions {
            for unit in state.units_at(pos) {
                let unit_type = db.unit_type(unit.type_id);
                if !unit_type.is_air && unit.is_alive {
                    claimers.insert(unit.player_id);
                }
            }
        }
        let owner_id = if claimers.len() != 1 {
            None
        } else {
            Some(claimers.into_iter().next().unwrap())
        };
        if sector.owner_id != owner_id {
            events.push(CoreEvent::SectorOwnerChanged {
                sector_id: sector_id,
                new_owner_id: owner_id,
            });
        }
    }
    events
}

// TODO: return iterator?
impl From<ExactPos> for MapPos {
    fn from(pos: ExactPos) -> MapPos {
        pos.map_pos
    }
}

#[derive(Clone, Debug)]
pub struct Sector {
    pub owner_id: Option<PlayerId>,
    pub positions: Vec<MapPos>,
}

impl Sector {
    pub fn center(&self) -> MapPos {
        let mut pos = Vector2{x: 0.0, y: 0.0};
        for sector_pos in &self.positions {
            pos.x += sector_pos.v.x as f32;
            pos.y += sector_pos.v.y as f32;
        }
        pos /= self.positions.len() as f32;
        let pos = MapPos{v: Vector2{
            x: (pos.x + 0.5) as i32,
            y: (pos.y + 0.5) as i32,
        }};
        assert!(self.positions.contains(&pos));
        pos
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjectClass {
    Building,
    Road,
    Smoke,
    ReinforcementSector,
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ObjectId {
    pub id: i32,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub pos: ExactPos,
    pub class: ObjectClass,
    pub timer: Option<i32>,
    pub owner_id: Option<PlayerId>,
}

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FireMode {
    Active,
    Reactive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ReactionFireMode {
    Normal,
    HoldFire,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MoveMode {
    Fast,
    Hunt,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Command {
    Move{unit_id: UnitId, path: Vec<ExactPos>, mode: MoveMode},
    EndTurn,
    CreateUnit{pos: ExactPos, type_id: UnitTypeId},
    AttackUnit{attacker_id: UnitId, defender_id: UnitId},
    LoadUnit{transporter_id: UnitId, passenger_id: UnitId},
    UnloadUnit{transporter_id: UnitId, passenger_id: UnitId, pos: ExactPos},
    Attach{transporter_id: UnitId, attached_unit_id: UnitId},
    Detach{transporter_id: UnitId, pos: ExactPos},
    SetReactionFireMode{unit_id: UnitId, mode: ReactionFireMode},
    Smoke{unit_id: UnitId, pos: MapPos},
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitInfo {
    pub unit_id: UnitId,
    pub pos: ExactPos,
    pub type_id: UnitTypeId,
    pub player_id: PlayerId,
    pub passenger_id: Option<UnitId>,
    pub attached_unit_id: Option<UnitId>,
    pub is_loaded: bool,
    pub is_attached: bool,
    pub is_alive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttackInfo {
    pub attacker_id: Option<UnitId>,
    pub defender_id: UnitId,
    pub mode: FireMode,
    pub killed: i32,
    pub suppression: i32,
    pub remove_move_points: bool,
    pub is_ambush: bool,
    pub is_inderect: bool,
    pub leave_wrecks: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoreEvent {
    Move {
        unit_id: UnitId,
        from: ExactPos,
        to: ExactPos,
        mode: MoveMode,
        cost: MovePoints,
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
    // Reveal is like ShowUnit but is generated directly by Core
    Reveal {
        unit_info: UnitInfo,
    },
    ShowUnit {
        unit_info: UnitInfo,
    },
    HideUnit {
        unit_id: UnitId,
    },
    LoadUnit {
        transporter_id: Option<UnitId>,
        passenger_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    UnloadUnit {
        unit_info: UnitInfo,
        transporter_id: Option<UnitId>,
        from: ExactPos,
        to: ExactPos,
    },
    Attach {
        transporter_id: UnitId,
        attached_unit_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    Detach {
        transporter_id: UnitId,
        from: ExactPos,
        to: ExactPos,
    },
    SetReactionFireMode {
        unit_id: UnitId,
        mode: ReactionFireMode,
    },
    SectorOwnerChanged {
        sector_id: SectorId,
        new_owner_id: Option<PlayerId>,
    },
    VictoryPoint {
        player_id: PlayerId,
        pos: MapPos,
        count: i32,
    },
    // TODO: CreateObject
    Smoke {
        id: ObjectId,
        pos: MapPos,
        unit_id: Option<UnitId>,
    },
    // TODO: RemoveObject
    RemoveSmoke {
        id: ObjectId,
    },
}

pub fn move_cost_modifier(mode: MoveMode) -> i32 {
    match mode {
        MoveMode::Fast => 1,
        MoveMode::Hunt => 2,
    }
}

pub fn is_unit_in_object(unit: &Unit, object: &Object) -> bool {
    if unit.pos == object.pos {
        return true;
    }
    let is_object_big = object.pos.slot_id == SlotId::WholeTile;
    is_object_big && unit.pos.map_pos == object.pos.map_pos
}

// TODO: simplify/optimize
pub fn find_next_player_unit_id(
    state: &State,
    player_id: PlayerId,
    unit_id: UnitId,
) -> UnitId {
    let mut i = state.units().cycle().filter(
        |&(_, unit)| unit.is_alive && unit.player_id == player_id);
    while let Some((&id, _)) = i.next() {
        if id == unit_id {
            let (&id, _) = i.next().unwrap();
            return id;
        }
    }
    unreachable!()
}

// TODO: simplify/optimize
pub fn find_prev_player_unit_id(
    state: &State,
    player_id: PlayerId,
    unit_id: UnitId,
) -> UnitId {
    let mut i = state.units().cycle().filter(
        |&(_, unit)| unit.is_alive && unit.player_id == player_id).peekable();
    while let Some((&id, _)) = i.next() {
        let &(&next_id, _) = i.peek().unwrap();
        if next_id == unit_id {
            return id;
        }
    }
    unreachable!()
}

pub fn is_loaded_or_attached(unit: &Unit) -> bool {
    unit.is_loaded || unit.is_attached
}

pub fn get_unit_ids_at(state: &State, pos: MapPos) -> Vec<UnitId> {
    let mut ids = Vec::new();
    for unit in state.units_at(pos) {
        if !is_loaded_or_attached(unit) {
            ids.push(unit.id)
        }
    }
    ids
}

pub fn unit_to_info(unit: &Unit) -> UnitInfo {
    UnitInfo {
        unit_id: unit.id,
        pos: unit.pos,
        type_id: unit.type_id,
        player_id: unit.player_id,
        passenger_id: unit.passenger_id,
        attached_unit_id: unit.attached_unit_id,
        is_alive: unit.is_alive,
        is_loaded: unit.is_loaded,
        is_attached: unit.is_attached,
    }
}

#[derive(Clone, Debug)]
struct PlayerInfo {
    events: VecDeque<CoreEvent>,
    visible_enemies: HashSet<UnitId>,

    // This filed is optional because we need to temporary
    // put its Fow into Core's State for filtering events.
    //
    // See State::to_full, State:to_partial
    //
    fow: Option<Fow>,
}

impl PlayerInfo {
    fn new(db: Rc<Db>, player_id: PlayerId, map_size: Size2) -> PlayerInfo {
        let fow = Fow::new(db, map_size, player_id);
        PlayerInfo {
            fow: Some(fow),
            events: VecDeque::new(),
            visible_enemies: HashSet::new(),
        }
    }

    fn fow(&self) -> &Fow {
        self.fow.as_ref().unwrap()
    }

    fn fow_mut(&mut self) -> &mut Fow {
        self.fow.as_mut().unwrap()
    }
}

pub fn print_unit_info(db: &Db, unit: &Unit) {
    let unit_type = db.unit_type(unit.type_id);
    let weapon_type = db.weapon_type(unit_type.weapon_type_id);
    println!("unit:");
    println!("  player_id: {}", unit.player_id.id);
    if let Some(move_points) = unit.move_points {
        println!("  move_points: {}", move_points.n);
    } else {
        println!("  move_points: ?");
    }
    if let Some(attack_points) = unit.attack_points {
        println!("  attack_points: {}", attack_points.n);
    } else {
        println!("  attack_points: ?");
    }
    if let Some(reactive_attack_points) = unit.reactive_attack_points {
        println!("  reactive_attack_points: {}", reactive_attack_points.n);
    } else {
        println!("  reactive_attack_points: ?");
    }
    println!("  count: {}", unit.count);
    println!("  morale: {}", unit.morale);
    println!("  passenger_id: {:?}", unit.passenger_id);
    println!("  attached_unit_id: {:?}", unit.attached_unit_id);
    println!("  is_alive: {:?}", unit.is_alive);
    println!("type:");
    println!("  name: {}", unit_type.name);
    println!("  is_infantry: {}", unit_type.is_infantry);
    println!("  count: {}", unit_type.count);
    println!("  size: {}", unit_type.size);
    println!("  armor: {}", unit_type.armor);
    println!("  toughness: {}", unit_type.toughness);
    println!("  weapon_skill: {}", unit_type.weapon_skill);
    println!("  mp: {}", unit_type.move_points.n);
    println!("  ap: {}", unit_type.attack_points.n);
    println!("  reactive_ap: {}", unit_type.reactive_attack_points.n);
    println!("  los_range: {}", unit_type.los_range.n);
    println!("  cover_los_range: {}", unit_type.cover_los_range.n);
    println!("weapon:");
    println!("  name: {}", weapon_type.name);
    println!("  damage: {}", weapon_type.damage);
    println!("  ap: {}", weapon_type.ap);
    println!("  accuracy: {}", weapon_type.accuracy);
    println!("  min_distance: {}", weapon_type.min_distance.n);
    println!("  max_distance: {}", weapon_type.max_distance.n);
    println!("  smoke: {:?}", weapon_type.smoke);
}

pub fn print_terrain_info(state: &State, pos: MapPos) {
    match *state.map().tile(pos) {
        Terrain::City => println!("City"),
        Terrain::Trees => println!("Trees"),
        Terrain::Plain => println!("Plain"),
        Terrain::Water => println!("Water"),
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ReactionFireResult {
    Attacked,
    Killed,
    None,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GameType {
    Hotseat,
    SingleVsAi,
}

impl Default for GameType {
    fn default() -> GameType {
        GameType::Hotseat
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub game_type: GameType,
    pub map_name: String,
    pub players_count: i32, // TODO: must it be defined by map/scenario?
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
                GameType::SingleVsAi => PlayerClass::Ai,
                GameType::Hotseat => PlayerClass::Human,
            },
        },
    )
}

fn get_player_info_lists(db: &Rc<Db>, map_size: Size2) -> HashMap<PlayerId, PlayerInfo> {
    let mut map = HashMap::new();
    map.insert(PlayerId{id: 0}, PlayerInfo::new(
        db.clone(), PlayerId{id: 0}, map_size));
    map.insert(PlayerId{id: 1}, PlayerInfo::new(
        db.clone(), PlayerId{id: 1}, map_size));
    map
}

pub fn objects_at(objects: &HashMap<ObjectId, Object>, pos: MapPos) -> ObjectsAtIter {
    ObjectsAtIter::new(objects, pos)
}

pub fn get_free_slot_for_building(
    map: &Map<Terrain>,
    objects: &HashMap<ObjectId, Object>,
    pos: MapPos,
) -> Option<SlotId> {
    let mut slots = [false, false, false];
    for object in objects_at(objects, pos) {
        if let SlotId::Id(slot_id) = object.pos.slot_id {
            slots[slot_id as usize] = true;
        } else {
            return None;
        }
    }
    let slots_count = get_slots_count(map, pos) as usize;
    for (i, slot) in slots.iter().enumerate().take(slots_count) {
        if !slot {
            return Some(SlotId::Id(i as u8));
        }
    }
    None
}

pub fn get_free_exact_pos(
    db: &Db,
    state: &State,
    type_id: UnitTypeId,
    pos: MapPos,
) -> Option<ExactPos> {
    let slot_id = match get_free_slot_id(db, state, type_id, pos) {
        Some(id) => id,
        None => return None,
    };
    Some(ExactPos{map_pos: pos, slot_id: slot_id})
}

pub fn get_free_slot_id(
    db: &Db,
    state: &State,
    type_id: UnitTypeId,
    pos: MapPos,
) -> Option<SlotId> {
    let objects_at = state.objects_at(pos);
    let units_at = state.units_at(pos);
    let unit_type = db.unit_type(type_id);
    if unit_type.is_air {
        for unit in units_at.clone() {
            if unit.pos.slot_id == SlotId::Air {
                return None;
            }
        }
        return Some(SlotId::Air);
    }
    if unit_type.is_big {
        for object in objects_at {
            match object.class {
                ObjectClass::Building => return None,
                ObjectClass::Smoke |
                ObjectClass::ReinforcementSector |
                ObjectClass::Road => {},
            }
        }
        if units_at.count() == 0 {
            return Some(SlotId::WholeTile);
        } else {
            return None;
        }
    }
    let mut slots = [false, false, false];
    for unit in units_at {
        match unit.pos.slot_id {
            SlotId::Id(slot_id) => slots[slot_id as usize] = true,
            SlotId::WholeTile | SlotId::TwoTiles(_) => return None,
            SlotId::Air => {},
        }
    }
    if !unit_type.is_infantry {
        for object in objects_at {
            match object.pos.slot_id {
                SlotId::Id(slot_id) => {
                    slots[slot_id as usize] = true;
                },
                SlotId::WholeTile => {
                    match object.class {
                        ObjectClass::Building => return None,
                        ObjectClass::Smoke |
                        ObjectClass::ReinforcementSector |
                        ObjectClass::Road => {},
                    }
                }
                SlotId::TwoTiles(_) | SlotId::Air => {},
            }
        }
    }
    let slots_count = get_slots_count(state.map(), pos) as usize;
    for (i, slot) in slots.iter().enumerate().take(slots_count) {
        if !slot {
            return Some(SlotId::Id(i as u8));
        }
    }
    None
}

pub fn get_slots_count(map: &Map<Terrain>, pos: MapPos) -> i32 {
    match *map.tile(pos) {
        Terrain::Water => 1,
        Terrain::City |
        Terrain::Plain |
        Terrain::Trees => 3,
    }
}

// TODO: join logic with get_free_slot_id
pub fn is_exact_pos_free(
    db: &Db,
    state: &State,
    type_id: UnitTypeId,
    pos: ExactPos,
) -> bool {
    let units_at = state.units_at(pos.map_pos);
    let unit_type = db.unit_type(type_id);
    if unit_type.is_big && !unit_type.is_air {
        return units_at.count() == 0;
    }
    for unit in units_at {
        if unit.pos == pos {
            return false;
        }
        match unit.pos.slot_id {
            SlotId::WholeTile | SlotId::TwoTiles(_) => {
                return false;
            }
            _ => {}
        }
    }
    true
}

fn cover_bonus(db: &Db, state: &State, defender: &Unit) -> i32 {
    let defender_type = db.unit_type(defender.type_id);
    if defender_type.is_infantry {
        match *state.map().tile(defender.pos) {
            Terrain::Plain | Terrain::Water => 0,
            Terrain::Trees => 2,
            Terrain::City => 3,
        }
    } else {
        0
    }
}

pub fn hit_chance(
    db: &Db,
    state: &State,
    attacker: &Unit,
    defender: &Unit,
) -> HitChance {
    let attacker_type = db.unit_type(attacker.type_id);
    let defender_type = db.unit_type(defender.type_id);
    let weapon_type = db.weapon_type(attacker_type.weapon_type_id);
    let cover_bonus = cover_bonus(db, state, defender);
    let hit_test_v = -7 - cover_bonus + defender_type.size
        + weapon_type.accuracy + attacker_type.weapon_skill;
    let pierce_test_v = 10 + -defender_type.armor + weapon_type.ap;
    let wound_test_v = 5 -defender_type.toughness + weapon_type.damage;
    let hit_test_v = clamp(hit_test_v, 0, 10);
    let pierce_test_v = clamp(pierce_test_v, 0, 10);
    let wound_test_v = clamp(wound_test_v, 0, 10);
    let k = (hit_test_v * pierce_test_v * wound_test_v) / 10;
    HitChance{n: clamp(k, 0, 100)}
}

impl Core {
    pub fn new(options: &Options) -> Core {
        let db = Rc::new(Db::new());
        let state = State::new_full(db.clone(), options);
        let players_info = get_player_info_lists(&db, state.map().size());
        let ai = Ai::new(db.clone(), options, PlayerId{id:1});
        Core {
            state: state,
            players: get_players_list(options),
            current_player_id: PlayerId{id: 0},
            db: db,
            ai: ai,
            players_info: players_info,
            next_unit_id: UnitId{id: 0},
            next_object_id: ObjectId{id: 0},
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

    fn get_killed_count(&self, attacker: &Unit, defender: &Unit) -> i32 {
        let hit = self.attack_test(attacker, defender);
        if !hit {
            return 0;
        }
        let defender_type = self.db.unit_type(defender.type_id);
        if defender_type.is_infantry {
            clamp(thread_rng().gen_range(1, 5), 1, defender.count)
        } else {
            1
        }
    }

    fn attack_test(&self, attacker: &Unit, defender: &Unit) -> bool {
        let k = hit_chance(&self.db, &self.state, attacker, defender).n;
        let r = thread_rng().gen_range(0, 100);
        r < k
    }

    fn player(&self) -> &Player {
        &self.players[self.player_id().id as usize]
    }

    pub fn player_id(&self) -> PlayerId {
        self.current_player_id
    }

    pub fn get_event(&mut self) -> Option<CoreEvent> {
        let mut i = self.players_info.get_mut(&self.current_player_id)
            .expect("core: Can`t get current player`s info");
        i.events.pop_front()
    }

    fn command_attack_unit_to_event(
        &self,
        attacker_id: UnitId,
        defender_id: UnitId,
        fire_mode: FireMode,
    ) -> Option<CoreEvent> {
        let attacker = self.state.unit(attacker_id);
        let defender = self.state.unit(defender_id);
        let check_attack_result = check_attack(
            &self.db,
            &self.state,
            attacker,
            defender,
            fire_mode,
        );
        if check_attack_result.is_err() {
            return None;
        }
        let attacker_type = self.db.unit_type(attacker.type_id);
        let weapon_type = self.db.weapon_type(attacker_type.weapon_type_id);
        let hit_chance = hit_chance(&self.db, &self.state, attacker, defender);
        let suppression = hit_chance.n / 2;
        let killed = cmp::min(
            defender.count, self.get_killed_count(attacker, defender));
        let fow = self.players_info[&defender.player_id].fow();
        let is_visible = fow.is_visible(attacker);
        let ambush_chance = 70;
        let is_ambush = !is_visible
            && thread_rng().gen_range(1, 100) <= ambush_chance;
        let per_death_suppression = 20;
        let defender_type = self.db.unit_type(defender.type_id);
        // TODO: destroyed helicopters must kill everyone
        // on the ground in their tile
        let leave_wrecks = !defender_type.is_infantry && !defender_type.is_air;
        let attack_info = AttackInfo {
            attacker_id: Some(attacker_id),
            defender_id: defender_id,
            killed: killed,
            mode: fire_mode,
            suppression: suppression + per_death_suppression * killed,
            remove_move_points: false,
            is_ambush: is_ambush,
            is_inderect: weapon_type.is_inderect,
            leave_wrecks: leave_wrecks,
        };
        Some(CoreEvent::AttackUnit{attack_info: attack_info})
    }

    fn can_unit_make_reaction_attack(
        &self,
        defender: &Unit,
        attacker: &Unit,
    ) -> bool {
        assert!(attacker.player_id != defender.player_id);
        if attacker.reaction_fire_mode == ReactionFireMode::HoldFire {
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
            FireMode::Reactive,
        );
        check_attack_result.is_ok()
    }

    fn reaction_fire_internal(&mut self, unit_id: UnitId, stop_on_attack: bool) -> ReactionFireResult {
        let unit_ids: Vec<_> = self.state.units().map(|(&id, _)| id).collect();
        let mut result = ReactionFireResult::None;
        for enemy_unit_id in unit_ids {
            if is_loaded_or_attached(self.state.unit(enemy_unit_id)) {
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
                    enemy_unit.id, unit_id, FireMode::Reactive);
                if let Some(CoreEvent::AttackUnit{mut attack_info}) = event {
                    let hit_chance = hit_chance(&self.db, &self.state, enemy_unit, unit);
                    let unit_type = self.db.unit_type(unit.type_id);
                    if hit_chance.n > 15 && !unit_type.is_air && stop_on_attack {
                        attack_info.remove_move_points = true;
                    }
                    CoreEvent::AttackUnit{attack_info: attack_info}
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
        let id = self.current_player_id;
        let mut i = self.players_info.get_mut(&id).unwrap();
        self.state.to_partial(i.fow.take().unwrap());
        if let Err(err) = check_command(&self.db, id, &self.state, command) {
            panic!("Bad command: {:?} ({:?})", err, command);
        }
        i.fow = Some(self.state.to_full());
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
                        end_turn_events.push(CoreEvent::VictoryPoint {
                            player_id: player_id,
                            pos: sector.center(),
                            count: 1,
                        });
                    }
                }
                for (&object_id, object) in self.state.objects() {
                    if let Some(timer) = object.timer {
                        if timer <= 0 {
                            end_turn_events.push(CoreEvent::RemoveSmoke {
                                id: object_id,
                            });
                        }
                    }
                }
                for event in end_turn_events {
                    self.do_core_event(&event);
                }
                self.do_core_event(&CoreEvent::EndTurn {
                    old_id: old_id,
                    new_id: new_id,
                });
            },
            Command::CreateUnit{pos, type_id} => {
                let event = CoreEvent::CreateUnit {
                    unit_info: UnitInfo {
                        unit_id: self.get_new_unit_id(),
                        pos: pos,
                        type_id: type_id,
                        player_id: self.current_player_id,
                        passenger_id: None,
                        attached_unit_id: None,
                        is_alive: true,
                        is_loaded: false,
                        is_attached: false,
                    },
                };
                self.do_core_event(&event);
            },
            Command::Move{unit_id, path, mode} => {
                let player_id = self.state.unit(unit_id).player_id;
                for window in path.windows(2) {
                    let from = window[0];
                    let to = window[1];
                    let show_event = self.state.unit_at_opt(to).and_then(|unit| {
                        Some(CoreEvent::Reveal {
                            unit_info: unit_to_info(unit),
                        })
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
                        CoreEvent::Move {
                            unit_id: unit_id,
                            from: from,
                            to: to,
                            mode: mode,
                            cost: cost,
                        }
                    };
                    let pre_visible_enemies = self.players_info[&player_id]
                        .visible_enemies.clone();
                    self.do_core_event(&move_event);
                    let reaction_fire_result = self.reaction_fire_internal(
                        unit_id, mode == MoveMode::Fast);
                    if reaction_fire_result != ReactionFireResult::None {
                        break;
                    }
                    let i = &self.players_info[&player_id];
                    if pre_visible_enemies != i.visible_enemies {
                        break;
                    }
                }
            },
            Command::AttackUnit{attacker_id, defender_id} => {
                if let Some(ref event) = self.command_attack_unit_to_event(
                    attacker_id, defender_id, FireMode::Active)
                {
                    self.do_core_event(event);
                    self.reaction_fire(attacker_id);
                }
            },
            Command::LoadUnit{transporter_id, passenger_id} => {
                let from = self.state.unit(passenger_id).pos;
                let to = self.state.unit(transporter_id).pos;
                self.do_core_event(&CoreEvent::LoadUnit {
                    transporter_id: Some(transporter_id),
                    passenger_id: passenger_id,
                    from: from,
                    to: to,
                });
            },
            Command::UnloadUnit{transporter_id, passenger_id, pos} => {
                let event = {
                    let passenger = self.state.unit(passenger_id);
                    let from = self.state.unit(transporter_id).pos;
                    CoreEvent::UnloadUnit {
                        transporter_id: Some(transporter_id),
                        unit_info: UnitInfo {
                            pos: pos,
                            .. unit_to_info(passenger)
                        },
                        from: from,
                        to: pos,
                    }
                };
                self.do_core_event(&event);
                self.reaction_fire(passenger_id);
            },
            Command::Attach{transporter_id, attached_unit_id} => {
                let from = self.state.unit(transporter_id).pos;
                let to = self.state.unit(attached_unit_id).pos;
                self.do_core_event(&CoreEvent::Attach {
                    transporter_id: transporter_id,
                    attached_unit_id: attached_unit_id,
                    from: from,
                    to: to,
                });
                self.reaction_fire(transporter_id);
            },
            Command::Detach{transporter_id, pos} => {
                let from = self.state.unit(transporter_id).pos;
                self.do_core_event(&CoreEvent::Detach {
                    transporter_id: transporter_id,
                    from: from,
                    to: pos,
                });
                self.reaction_fire(transporter_id);
            },
            Command::SetReactionFireMode{unit_id, mode} => {
                self.do_core_event(&CoreEvent::SetReactionFireMode {
                    unit_id: unit_id,
                    mode: mode,
                });
            },
            Command::Smoke{unit_id, pos} => {
                let id = self.get_new_object_id();
                self.do_core_event(&CoreEvent::Smoke {
                    id: id,
                    unit_id: Some(unit_id),
                    pos: pos,
                });
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
                    self.do_core_event(&CoreEvent::Smoke {
                        id: id,
                        unit_id: Some(unit_id),
                        pos: Dir::get_neighbour_pos(pos, dir),
                    });
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
        let state = &self.state;
        let (filtered_events, active_unit_ids) = filter::filter_events(
            state, player_id, i.fow(), event);
        for filtered_event in filtered_events {
            i.fow_mut().apply_event(state, &filtered_event);
            i.events.push_back(filtered_event);
            let new_enemies = filter::get_visible_enemies(
                state, i.fow(), player_id);
            let show_hide_events = filter::show_or_hide_passive_enemies(
                state, &active_unit_ids, &i.visible_enemies, &new_enemies);
            i.events.extend(show_hide_events);
            i.visible_enemies = new_enemies;
        }
    }

    fn do_core_event(&mut self, event: &CoreEvent) {
        self.state.apply_event(event);
        let player_ids: Vec<_> = self.players.iter()
            .map(|player| player.id).collect();
        for player_id in player_ids {
            self.filter_event(player_id, &event);
        }
        if let CoreEvent::EndTurn{old_id, new_id} = *event {
            self.handle_end_turn_event(old_id, new_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Vector2};
    use ::{Sector, MapPos};

    #[test]
    fn test_center_1() {
        let real = Sector {
            positions: vec![
                MapPos{v: Vector2{x: 5, y: 0}},
                MapPos{v: Vector2{x: 6, y: 0}},
                MapPos{v: Vector2{x: 5, y: 1}},
                MapPos{v: Vector2{x: 6, y: 1}},
                MapPos{v: Vector2{x: 7, y: 1}},
                MapPos{v: Vector2{x: 5, y: 2}},
                MapPos{v: Vector2{x: 6, y: 2}},
            ],
            owner_id: None,
        }.center();
        let expected = MapPos{v: Vector2{x: 6, y: 1}};
        assert_eq!(expected, real);
    }

    #[test]
    fn test_center_2() {
        let real = Sector {
            positions: vec![
                MapPos{v: Vector2{x: 6, y: 0}},
                MapPos{v: Vector2{x: 6, y: 1}},
                MapPos{v: Vector2{x: 6, y: 2}},
            ],
            owner_id: None,
        }.center();
        let expected = MapPos{v: Vector2{x: 6, y: 1}};
        assert_eq!(expected, real);
    }
}
