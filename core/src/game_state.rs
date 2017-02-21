use std::collections::hash_map::{self, HashMap};
use std::collections::{HashSet};
use std::rc::{Rc};
use cgmath::{Vector2};
use types::{Size2};
use unit::{Unit, UnitId};
use db::{Db};
use map::{Map, Terrain};
use dir::{Dir};
use fow::{Fow};
use sector::{Sector, SectorId};
use position::{self, MapPos, ExactPos, SlotId};
use event::{CoreEvent, FireMode};
use player::{PlayerId};
use object::{ObjectId, Object, ObjectClass};
use movement::{MovePoints};
use attack::{AttackPoints};
use options::{Options};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReinforcementPoints{pub n: i32}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score{pub n: i32}

#[derive(Clone)]
pub struct ObjectsAtIter<'a> {
    it: hash_map::Iter<'a, ObjectId, Object>,
    pos: MapPos,
}

impl<'a> ObjectsAtIter<'a> {
    pub fn new(objects: &HashMap<ObjectId, Object>, pos: MapPos) -> ObjectsAtIter {
        ObjectsAtIter{it: objects.iter(), pos: pos}
    }
}

impl<'a> Iterator for ObjectsAtIter<'a> {
    type Item = &'a Object;

    fn next(&mut self) -> Option<Self::Item> {
        for (_, object) in &mut self.it {
            for map_pos in object.pos.map_pos_iter() {
                if self.pos == map_pos {
                    return Some(object);
                }
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct UnitsAtIter<'a> {
    it: UnitIter<'a>,
    pos: MapPos,
}

impl<'a> Iterator for UnitsAtIter<'a> {
    type Item = &'a Unit;

    fn next(&mut self) -> Option<Self::Item> {
        for (_, unit) in &mut self.it {
            if self.pos == unit.pos.map_pos {
                return Some(unit);
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct UnitIter<'a> {
    iter: hash_map::Iter<'a, UnitId, Unit>,
    state: &'a State,
}

impl<'a> Iterator for UnitIter<'a> {
    type Item = (&'a UnitId, &'a Unit);

    fn next(&mut self) -> Option<Self::Item> {
        for pair in &mut self.iter {
            let (_, unit) = pair;
            if self.state.is_unit_visible(unit) {
                return Some(pair);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct State {
    units: HashMap<UnitId, Unit>,
    objects: HashMap<ObjectId, Object>,
    map: Map<Terrain>,
    sectors: HashMap<SectorId, Sector>,
    score: HashMap<PlayerId, Score>,
    reinforcement_points: HashMap<PlayerId, ReinforcementPoints>,
    players_count: i32,
    db: Rc<Db>,

    // If this field is None then the State is considered "Full State"
    // (contains all information), otherwise the State is "Partial State"
    // (contains player-specific view on the game).
    //
    // When this field is Some there's no external access to units
    // in fogged tiles.
    //
    fow: Option<Fow>,

    /// Hack for not filtering fogged units from ShowUnit events
    shown_unit_ids: HashSet<UnitId>,
}

fn basic_state(db: Rc<Db>, options: &Options) -> State {
    let mut score = HashMap::new();
    score.insert(PlayerId{id: 0}, Score{n: 0});
    score.insert(PlayerId{id: 1}, Score{n: 0});
    let mut reinforcement_points = HashMap::new();
    reinforcement_points.insert(PlayerId{id: 0}, ReinforcementPoints{n: 10});
    reinforcement_points.insert(PlayerId{id: 1}, ReinforcementPoints{n: 10});
    let (map, objects, sectors) = load_map(&options.map_name);
    State {
        units: HashMap::new(),
        objects: objects,
        map: map,
        sectors: sectors,
        score: score,
        reinforcement_points: reinforcement_points,
        players_count: options.players_count,
        db: db,
        fow: None,
        shown_unit_ids: HashSet::new(),
    }
}

impl State {
    pub fn new_full(db: Rc<Db>, options: &Options) -> State {
        basic_state(db, options)
    }

    pub fn new_partial(db: Rc<Db>, options: &Options, id: PlayerId) -> State {
        let mut state = basic_state(db.clone(), options);
        let fow = Fow::new(&state, id);
        state.to_partial(fow);
        state
    }

    pub fn to_partial(&mut self, fow: Fow) {
        assert!(self.fow.is_none());
        self.fow = Some(fow);
    }

    pub fn to_full(&mut self) -> Fow {
        assert!(self.fow.is_some());
        self.fow.take().unwrap()
    }

    pub fn is_partial(&self) -> bool {
        self.fow.is_some()
    }

    pub fn db(&self) -> &Rc<Db> {
        &self.db
    }

    /// Converts active ap (attack points) to reactive
    fn convert_ap(&mut self, player_id: PlayerId) {
        for (_, unit) in &mut self.units {
            let unit_type = self.db.unit_type(unit.type_id);
            let weapon_type = self.db.weapon_type(unit_type.weapon_type_id);
            if unit.player_id != player_id || !weapon_type.reaction_fire {
                continue;
            }
            if let Some(ref mut reactive_attack_points)
                = unit.reactive_attack_points
            {
                reactive_attack_points.n += unit.attack_points.unwrap().n;
            }
            if let Some(ref mut attack_points) = unit.attack_points {
                attack_points.n = 0;
            }
        }
    }

    fn refresh_units(&mut self, player_id: PlayerId) {
        for (_, unit) in &mut self.units {
            if unit.player_id == player_id {
                let unit_type = self.db.unit_type(unit.type_id);
                if let Some(ref mut move_points) = unit.move_points {
                    *move_points = unit_type.move_points;
                }
                if let Some(ref mut attack_points) = unit.attack_points {
                    *attack_points = unit_type.attack_points;
                }
                if let Some(ref mut reactive_attack_points) = unit.reactive_attack_points {
                    *reactive_attack_points = unit_type.reactive_attack_points;
                }
                unit.morale += 10;
                let max_morale = 100; // TODO: get from UnitType
                if unit.morale > max_morale {
                    unit.morale = max_morale;
                }
            }
        }
    }

    fn add_unit(&mut self, unit: &Unit) {
        assert!(self.units.get(&unit.id).is_none());
        self.units.insert(unit.id, unit.clone());
    }

    pub fn units(&self) -> UnitIter {
        UnitIter {
            iter: self.units.iter(),
            state: self,
        }
    }

    fn is_unit_visible(&self, unit: &Unit) -> bool {
        let fow = match self.fow {
            Some(ref fow) => fow,
            None => return true,
        };
        fow.is_visible(unit) || self.shown_unit_ids.contains(&unit.id)
    }

    pub fn unit_opt(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id).and_then(|unit| {
            if self.is_unit_visible(unit) {
                Some(unit)
            } else {
                None
            }
        })
    }

    pub fn unit(&self, id: UnitId) -> &Unit {
        self.unit_opt(id).unwrap()
    }

    pub fn units_at(&self, pos: MapPos) -> UnitsAtIter {
        UnitsAtIter{it: self.units(), pos: pos}
    }

    pub fn objects_at(&self, pos: MapPos) -> ObjectsAtIter {
        ObjectsAtIter::new(self.objects(), pos)
    }

    pub fn unit_at_opt(&self, pos: ExactPos) -> Option<&Unit> {
        for unit in self.units_at(pos.map_pos) {
            if unit.pos == pos {
                return Some(unit);
            }
        }
        None
    }

    pub fn unit_at(&self, pos: ExactPos) -> &Unit {
        self.unit_at_opt(pos).unwrap()
    }

    pub fn objects(&self) -> &HashMap<ObjectId, Object> {
        &self.objects
    }

    pub fn map(&self) -> &Map<Terrain> {
        &self.map
    }

    pub fn sectors(&self) -> &HashMap<SectorId, Sector> {
        &self.sectors
    }

    pub fn score(&self) -> &HashMap<PlayerId, Score> {
        &self.score
    }

    pub fn reinforcement_points(&self) -> &HashMap<PlayerId, ReinforcementPoints> {
        &self.reinforcement_points
    }

    pub fn is_ground_tile_visible(&self, pos: MapPos) -> bool {
        if let Some(ref fow) = self.fow {
            fow.is_ground_tile_visible(pos)
        } else {
            true
        }
    }

    pub fn apply_event(&mut self, event: &CoreEvent) {
        match *event {
            CoreEvent::Move{unit_id, to, cost, ..} => {
                {
                    let unit = self.units.get_mut(&unit_id).unwrap();
                    unit.pos = to;
                    if let Some(ref mut move_points) = unit.move_points {
                        assert!(move_points.n > 0);
                        move_points.n -= cost.n;
                        assert!(move_points.n >= 0);
                    }
                }
                if let Some(passenger_id) = self.units[&unit_id].passenger_id {
                    let passenger = self.units.get_mut(&passenger_id).unwrap();
                    passenger.pos = to;
                }
                if let Some(attached_unit_id) = self.units[&unit_id].attached_unit_id {
                    let attached_unit = self.units.get_mut(&attached_unit_id).unwrap();
                    attached_unit.pos = to;
                }
            },
            CoreEvent::EndTurn{new_id, old_id} => {
                self.shown_unit_ids.clear();
                {
                    let reinforcement_points = self.reinforcement_points
                        .get_mut(&old_id).unwrap();
                    reinforcement_points.n += 10;
                }
                self.refresh_units(new_id);
                self.convert_ap(old_id);
                for (_, object) in &mut self.objects {
                    if let Some(ref mut timer) = object.timer {
                        *timer -= 1;
                        assert!(*timer >= 0);
                    }
                }
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                {
                    let unit_type = self.db.unit_type(unit_info.type_id);
                    let reinforcement_points = self.reinforcement_points
                        .get_mut(&unit_info.player_id).unwrap();
                    assert!(*reinforcement_points >= unit_type.cost);
                    reinforcement_points.n -= unit_type.cost.n;
                }
                self.add_unit(unit_info);
            },
            CoreEvent::AttackUnit{ref attack_info} => {
                let count;
                {
                    let unit = self.units.get_mut(&attack_info.defender_id)
                        .expect("Can`t find defender");
                    unit.count -= attack_info.killed;
                    unit.morale -= attack_info.suppression;
                    if attack_info.remove_move_points {
                        if let Some(ref mut move_points) = unit.move_points {
                            move_points.n = 0;
                        }
                    }
                    count = unit.count;
                }
                if count <= 0 {
                    if let Some(passenger_id)
                        = self.unit(attack_info.defender_id).passenger_id
                    {
                        self.units.remove(&passenger_id).unwrap();
                    }
                    if let Some(attached_unit_id)
                        = self.unit(attack_info.defender_id).attached_unit_id
                    {
                        let attached_unit = self.units.get_mut(&attached_unit_id).unwrap();
                        attached_unit.attack_points = Some(AttackPoints{n: 0});
                        attached_unit.reactive_attack_points = Some(AttackPoints{n: 0});
                        attached_unit.move_points = Some(MovePoints{n: 0});
                    }
                    if attack_info.leave_wrecks {
                        let unit = self.units.get_mut(&attack_info.defender_id).unwrap();
                        unit.attached_unit_id = None;
                        unit.passenger_id = None;
                        unit.is_alive = false;
                    } else {
                        assert!(self.units.get(&attack_info.defender_id).is_some());
                        self.units.remove(&attack_info.defender_id);
                    }
                }
                if let Some(attacker_id) = attack_info.attacker_id {
                    if let Some(unit) = self.units.get_mut(&attacker_id) {
                        match attack_info.mode {
                            FireMode::Active => {
                                if let Some(ref mut attack_points)
                                    = unit.attack_points
                                {
                                    assert!(attack_points.n >= 1);
                                    attack_points.n -= 1;
                                }
                            },
                            FireMode::Reactive => {
                                if let Some(ref mut reactive_attack_points)
                                    = unit.reactive_attack_points
                                {
                                    assert!(reactive_attack_points.n >= 1);
                                    reactive_attack_points.n -= 1;
                                }
                            },
                        }
                    }
                }
            },
            CoreEvent::Reveal{..} => (),
            CoreEvent::ShowUnit{ref unit_info} => {
                self.add_unit(unit_info);
                self.shown_unit_ids.insert(unit_info.id);
            },
            CoreEvent::HideUnit{unit_id} => {
                assert!(self.units.get(&unit_id).is_some());
                self.units.remove(&unit_id);
            },
            CoreEvent::LoadUnit{passenger_id, transporter_id, to, ..} => {
                // TODO: hide info about passenger from enemy player
                if let Some(transporter_id) = transporter_id {
                    self.units.get_mut(&transporter_id)
                        .expect("Bad transporter_id")
                        .passenger_id = Some(passenger_id);
                }
                let passenger = self.units.get_mut(&passenger_id)
                    .expect("Bad passenger_id");
                passenger.pos = to;
                passenger.is_loaded = true;
                if let Some(ref mut move_points) = passenger.move_points {
                    move_points.n = 0;
                }
            },
            CoreEvent::UnloadUnit{transporter_id, ref unit_info, ..} => {
                if let Some(transporter_id) = transporter_id {
                    self.units.get_mut(&transporter_id)
                        .expect("Bad transporter_id")
                        .passenger_id = None;
                }
                if self.unit_opt(unit_info.id).is_some() {
                    let unit = self.units.get_mut(&unit_info.id).unwrap();
                    unit.pos = unit_info.pos;
                    unit.is_loaded = false;
                } else {
                    self.add_unit(unit_info);
                }
            },
            CoreEvent::Attach{transporter_id, attached_unit_id, to, ..} => {
                if let Some(passenger_id) = self.unit(transporter_id).passenger_id {
                    let passenger = self.units.get_mut(&passenger_id).unwrap();
                    passenger.pos = to;
                }
                {
                    let attached_unit = self.units.get_mut(&attached_unit_id).unwrap();
                    attached_unit.is_attached = true;
                }
                let transporter = self.units.get_mut(&transporter_id).unwrap();
                transporter.pos = to;
                if let Some(ref mut move_points) = transporter.move_points {
                    move_points.n = 0;
                }
                transporter.attached_unit_id = Some(attached_unit_id);
            },
            CoreEvent::Detach{transporter_id, to, ..} => {
                if let Some(passenger_id) = self.unit(transporter_id).passenger_id {
                    let passenger = self.units.get_mut(&passenger_id).unwrap();
                    passenger.pos = to;
                }
                if let Some(attached_unit_id) = self.unit(transporter_id).attached_unit_id {
                    let attached_unit = self.units.get_mut(&attached_unit_id).unwrap();
                    attached_unit.is_attached = false;
                    if let Some(ref mut move_points) = attached_unit.move_points {
                        move_points.n = 0;
                    }
                }
                let transporter = self.units.get_mut(&transporter_id).unwrap();
                transporter.attached_unit_id = None;
                transporter.pos = to;
                if let Some(ref mut move_points) = transporter.move_points {
                    move_points.n = 0;
                }
            },
            CoreEvent::SetReactionFireMode{unit_id, mode} => {
                self.units.get_mut(&unit_id)
                    .expect("Bad unit id")
                    .reaction_fire_mode = mode;
            },
            CoreEvent::SectorOwnerChanged{sector_id, new_owner_id} => {
                let sector = self.sectors.get_mut(&sector_id).unwrap();
                sector.owner_id = new_owner_id;
            },
            CoreEvent::VictoryPoint{player_id, count, ..} => {
                self.score.get_mut(&player_id).unwrap().n += count;
            },
            CoreEvent::Smoke{pos, id, unit_id} => {
                if let Some(unit_id) = unit_id {
                    if let Some(unit) = self.units.get_mut(&unit_id) {
                        if let Some(ref mut attack_points) = unit.attack_points {
                            attack_points.n = 0;
                        }
                    }
                }
                let smoke_duration_in_turns = 3; // TODO: get from config
                let timer = smoke_duration_in_turns * self.players_count - 1;
                self.objects.insert(id, Object {
                    class: ObjectClass::Smoke,
                    pos: ExactPos {
                        map_pos: pos,
                        slot_id: SlotId::WholeTile,
                    },
                    timer: Some(timer),
                    owner_id: None,
                });
            },
            CoreEvent::RemoveSmoke{id} => {
                self.objects.remove(&id);
            },
        }
        if self.fow.is_some() {
            let mut fow = self.to_full();
            fow.apply_event(self, event);
            self.to_partial(fow);
        }
    }
}

// TODO: create trees, buildings and roads like units - using event system
fn add_object(objects: &mut HashMap<ObjectId, Object>, object: Object) {
    let id = ObjectId{id: objects.len() as i32 + 1};
    objects.insert(id, object);
}

fn add_road(objects: &mut HashMap<ObjectId, Object>, path: &[MapPos]) {
    for window in path.windows(2) {
        let from = window[0];
        let to = window[1];
        let dir = Dir::get_dir_from_to(from, to);
        let object = Object {
            class: ObjectClass::Road,
            pos: ExactPos {
                map_pos: from,
                slot_id: SlotId::TwoTiles(dir),
            },
            timer: None,
            owner_id: None,
        };
        add_object(objects, object);
    }
}

fn add_reinforcement_sector(
    objects: &mut HashMap<ObjectId, Object>,
    pos: MapPos,
    owner_id: Option<PlayerId>,
) {
    let object = Object {
        class: ObjectClass::ReinforcementSector,
        pos: ExactPos {
            map_pos: pos,
            slot_id: SlotId::WholeTile,
        },
        timer: None,
        owner_id: owner_id,
    };
    add_object(objects, object);
}

fn add_buildings(
    map: &mut Map<Terrain>,
    objects: &mut HashMap<ObjectId, Object>,
    pos: MapPos,
    count: i32,
) {
    *map.tile_mut(pos) = Terrain::City;
    for _ in 0 .. count {
        let slot_id = position::get_free_slot_for_building(map, objects, pos).unwrap();
        let obj_pos = ExactPos{map_pos: pos, slot_id: slot_id};
        let object = Object {
            class: ObjectClass::Building,
            pos: obj_pos,
            timer: None,
            owner_id: None,
        };
        add_object(objects, object);
    }
}

fn add_big_building(
    map: &mut Map<Terrain>,
    objects: &mut HashMap<ObjectId, Object>,
    pos: MapPos,
) {
    *map.tile_mut(pos) = Terrain::City;
    let object = Object {
        class: ObjectClass::Building,
        pos: ExactPos {
            map_pos: pos,
            slot_id: SlotId::WholeTile,
        },
        timer: None,
        owner_id: None,
    };
    add_object(objects, object);
}

type MapInfo = (Map<Terrain>, HashMap<ObjectId, Object>, HashMap<SectorId, Sector>);

// TODO: read from scenario.json?
fn load_map(map_name: &str) -> MapInfo {
    match map_name {
        "map01" => load_map_01(),
        "map02" => load_map_02(),
        "map03" => load_map_03(),
        "map04" => load_map_04(),
        "map05" => load_map_05(),
        "map_fov_bug_test" => load_map_fov_bug_test(),
        _ => unimplemented!(),
    }
}

fn load_map_01() -> MapInfo {
    let map_size = Size2{w: 10, h: 12};
    let mut objects = HashMap::new();
    let mut map = Map::new(map_size);
    let mut sectors = HashMap::new();
    for &((x, y), terrain) in &[
        ((6, 7), Terrain::Water),
        ((5, 8), Terrain::Water),
        ((5, 9), Terrain::Water),
        ((4, 10), Terrain::Water),
        ((5, 11), Terrain::Water),
        ((1, 2), Terrain::Trees),
        ((1, 6), Terrain::Trees),
        ((2, 6), Terrain::Trees),
        ((4, 3), Terrain::Trees),
        ((4, 4), Terrain::Trees),
        ((4, 5), Terrain::Trees),
        ((5, 1), Terrain::Trees),
        ((5, 10), Terrain::Trees),
        ((6, 0), Terrain::Trees),
        ((6, 1), Terrain::Trees),
        ((6, 2), Terrain::Trees),
    ] {
        *map.tile_mut(MapPos{v: Vector2{x: x, y: y}}) = terrain;
    }
    for &((x, y), count) in &[
        ((5, 4), 2),
        ((5, 5), 2),
        ((5, 6), 1),
        ((6, 5), 3),
        ((6, 6), 1),
        ((8, 11), 2),
        ((8, 10), 2),
        ((9, 11), 1),
    ] {
        let pos = MapPos{v: Vector2{x: x, y: y}};
        add_buildings(&mut map, &mut objects, pos, count);
    }
    for &(x, y) in &[
        (6, 4),
    ] {
        let pos = MapPos{v: Vector2{x: x, y: y}};
        add_big_building(&mut map, &mut objects, pos);
    }
    add_road(&mut objects, &[
        MapPos{v: Vector2{x: 0, y: 1}},
        MapPos{v: Vector2{x: 1, y: 1}},
        MapPos{v: Vector2{x: 2, y: 1}},
        MapPos{v: Vector2{x: 2, y: 2}},
        MapPos{v: Vector2{x: 3, y: 2}},
        MapPos{v: Vector2{x: 4, y: 2}},
        MapPos{v: Vector2{x: 5, y: 2}},
        MapPos{v: Vector2{x: 6, y: 3}},
        MapPos{v: Vector2{x: 7, y: 3}},
        MapPos{v: Vector2{x: 8, y: 3}},
        MapPos{v: Vector2{x: 9, y: 3}},
    ]);
    add_road(&mut objects, &[
        MapPos{v: Vector2{x: 2, y: 2}},
        MapPos{v: Vector2{x: 3, y: 3}},
        MapPos{v: Vector2{x: 3, y: 4}},
        MapPos{v: Vector2{x: 3, y: 5}},
        MapPos{v: Vector2{x: 3, y: 6}},
        MapPos{v: Vector2{x: 4, y: 6}},
        MapPos{v: Vector2{x: 5, y: 7}},
        MapPos{v: Vector2{x: 5, y: 8}},
        MapPos{v: Vector2{x: 6, y: 9}},
        MapPos{v: Vector2{x: 6, y: 10}},
        MapPos{v: Vector2{x: 7, y: 11}},
    ]);
    for &((x, y), player_index) in &[
        ((0, 1), 0),
        ((0, 7), 0),
        ((9, 3), 1),
        ((9, 8), 1),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    sectors.insert(
        SectorId{id: 0},
        Sector {
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
        },
    );
    sectors.insert(
        SectorId{id: 1},
        Sector {
            positions: vec![
                MapPos{v: Vector2{x: 5, y: 4}},
                MapPos{v: Vector2{x: 6, y: 4}},
                MapPos{v: Vector2{x: 5, y: 5}},
                MapPos{v: Vector2{x: 6, y: 5}},
                MapPos{v: Vector2{x: 7, y: 5}},
                MapPos{v: Vector2{x: 5, y: 6}},
                MapPos{v: Vector2{x: 6, y: 6}},
            ],
            owner_id: None,
        },
    );
    (map, objects, sectors)
}

fn load_map_02() -> MapInfo {
    let map_size = Size2{w: 9, h: 12};
    let mut objects = HashMap::new();
    let mut map = Map::new(map_size);
    let mut sectors = HashMap::new();
    for &((x, y), terrain) in &[
        ((3, 6), Terrain::Trees),
        ((3, 7), Terrain::Trees),
    ] {
        *map.tile_mut(MapPos{v: Vector2{x: x, y: y}}) = terrain;
    }
    for &((x, y), player_index) in &[
        ((0, 4), 0),
        ((0, 10), 0),
        ((8, 4), 1),
        ((8, 10), 1),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    sectors.insert(
        SectorId{id: 0},
        Sector {
            positions: vec![
                MapPos{v: Vector2{x: 4, y: 3}},
            ],
            owner_id: None,
        },
    );
    sectors.insert(
        SectorId{id: 1},
        Sector {
            positions: vec![
                MapPos{v: Vector2{x: 5, y: 8}},
            ],
            owner_id: None,
        },
    );
    (map, objects, sectors)
}

fn load_map_03() -> MapInfo {
    let map_size = Size2{w: 3, h: 1};
    let mut objects = HashMap::new();
    let mut map = Map::new(map_size);
    let sectors = HashMap::new();
    for &((x, y), terrain) in &[
        ((1, 0), Terrain::Trees),
    ] {
        *map.tile_mut(MapPos{v: Vector2{x: x, y: y}}) = terrain;
    }
    for &((x, y), player_index) in &[
        ((0, 0), 0),
        ((2, 0), 1),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    (map, objects, sectors)
}

fn load_map_04() -> MapInfo {
    let map_size = Size2{w: 2, h: 1};
    let mut objects = HashMap::new();
    let mut map = Map::new(map_size);
    let sectors = HashMap::new();
    for &((x, y), terrain) in &[
        ((1, 0), Terrain::Trees),
    ] {
        *map.tile_mut(MapPos{v: Vector2{x: x, y: y}}) = terrain;
    }
    for &((x, y), player_index) in &[
        ((0, 0), 0),
        ((1, 0), 1),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    (map, objects, sectors)
}

fn load_map_05() -> MapInfo {
    let map_size = Size2{w: 3, h: 1};
    let mut objects = HashMap::new();
    let map = Map::new(map_size);
    let sectors = HashMap::new();
    for &((x, y), player_index) in &[
        ((0, 0), 0),
        ((2, 0), 1),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    (map, objects, sectors)
}

/// Map for repoducing of https://github.com/ozkriff/zoc/issues/149
fn load_map_fov_bug_test() -> MapInfo {
    let map_size = Size2{w: 20, h: 20};
    let mut objects = HashMap::new();
    let mut map = Map::new(map_size);
    let sectors = HashMap::new();
    for &((x, y), terrain) in &[
        ((9, 10), Terrain::Trees),
        ((10, 9), Terrain::Trees),
    ] {
        *map.tile_mut(MapPos{v: Vector2{x: x, y: y}}) = terrain;
    }
    for &((x, y), player_index) in &[
        ((10, 10), 0),
    ] {
        add_reinforcement_sector(
            &mut objects,
            MapPos{v: Vector2{x: x, y: y}},
            Some(PlayerId{id: player_index}),
        );
    }
    (map, objects, sectors)
}
