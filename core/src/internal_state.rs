use std::collections::{HashMap};
use cgmath::{Vector2};
use types::{Size2};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use game_state::{GameState, GameStateMut};
use dir::{Dir};
use ::{
    CoreEvent,
    FireMode,
    UnitInfo,
    ReactionFireMode,
    PlayerId,
    UnitId,
    MapPos,
    ExactPos,
    SlotId,
    Object,
    ObjectId,
    ObjectClass,
    Sector,
    SectorId,
    Score,
    MovePoints,
    AttackPoints,
    Options,
    get_free_slot_for_building,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InfoLevel {
    Full,
    Partial,
}

#[derive(Clone, Debug)]
pub struct InternalState {
    units: HashMap<UnitId, Unit>,
    objects: HashMap<ObjectId, Object>,
    map: Map<Terrain>,
    sectors: HashMap<SectorId, Sector>,
    score: HashMap<PlayerId, Score>,
    reinforcement_points: HashMap<PlayerId, i32>, // TODO: i32 -> ???
    players_count: i32,
}

impl InternalState {
    pub fn new(options: &Options) -> InternalState {
        let mut score = HashMap::new();
        score.insert(PlayerId{id: 0}, Score{n: 0});
        score.insert(PlayerId{id: 1}, Score{n: 0});
        let mut reinforcement_points = HashMap::new();
        reinforcement_points.insert(PlayerId{id: 0}, 10);
        reinforcement_points.insert(PlayerId{id: 1}, 10);
        let (map, objects, sectors) = load_map(&options.map_name);
        InternalState {
            units: HashMap::new(),
            objects: objects,
            map: map,
            sectors: sectors,
            score: score,
            reinforcement_points: reinforcement_points,
            players_count: options.players_count,
        }
    }

    /// Converts active ap (attack points) to reactive
    fn convert_ap(&mut self, db: &Db, player_id: PlayerId) {
        for (_, unit) in &mut self.units {
            let unit_type = db.unit_type(unit.type_id);
            let weapon_type = db.weapon_type(unit_type.weapon_type_id);
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

    fn refresh_units(&mut self, db: &Db, player_id: PlayerId) {
        for (_, unit) in &mut self.units {
            if unit.player_id == player_id {
                let unit_type = db.unit_type(unit.type_id);
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

    fn add_unit(&mut self, db: &Db, unit_info: &UnitInfo, info_level: InfoLevel) {
        assert!(self.units.get(&unit_info.unit_id).is_none());
        let unit_type = db.unit_type(unit_info.type_id);
        let cost = unit_type.cost;
        let reinforcement_points = self.reinforcement_points
            .get_mut(&unit_info.player_id).unwrap();
        if *reinforcement_points < cost {
            return;
        }
        *reinforcement_points -= cost;
        self.units.insert(unit_info.unit_id, Unit {
            is_alive: unit_info.is_alive,
            id: unit_info.unit_id,
            pos: unit_info.pos,
            player_id: unit_info.player_id,
            type_id: unit_info.type_id,
            move_points: if info_level == InfoLevel::Full {
                Some(MovePoints{n: 0})
            } else {
                None
            },
            attack_points: if info_level == InfoLevel::Full {
                Some(AttackPoints{n: 0})
            } else {
                None
            },
            reactive_attack_points: if info_level == InfoLevel::Full {
                Some(AttackPoints{n: 0})
            } else {
                None
            },
            reaction_fire_mode: ReactionFireMode::Normal,
            count: unit_type.count,
            morale: 100,
            passenger_id: if info_level == InfoLevel::Full {
                unit_info.passenger_id
            } else {
                None
            },
        });
    }
}

impl GameState for InternalState {
    fn units(&self) -> &HashMap<UnitId, Unit> {
        &self.units
    }

    fn objects(&self) -> &HashMap<ObjectId, Object> {
        &self.objects
    }

    fn map(&self) -> &Map<Terrain> {
        &self.map
    }

    fn sectors(&self) -> &HashMap<SectorId, Sector> {
        &self.sectors
    }

    fn score(&self) -> &HashMap<PlayerId, Score> {
        &self.score
    }

    fn reinforcement_points(&self) -> &HashMap<PlayerId, i32> {
        &self.reinforcement_points
    }
}

impl GameStateMut for InternalState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
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
            },
            CoreEvent::EndTurn{new_id, old_id} => {
                {
                    let reinforcement_points = self.reinforcement_points
                        .get_mut(&old_id).unwrap();
                    *reinforcement_points += 10;
                }
                self.refresh_units(db, new_id);
                self.convert_ap(db, old_id);
                // TODO: timer ticks on every player's turn! O.o
                for (_, object) in &mut self.objects {
                    if let Some(ref mut timer) = object.timer {
                        *timer -= 1;
                        assert!(*timer >= 0);
                    }
                }
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                self.add_unit(db, unit_info, InfoLevel::Full);
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
                    if attack_info.leave_wrecks {
                        // TODO: kill\unload passengers
                        let unit = self.units.get_mut(&attack_info.defender_id).unwrap();
                        unit.is_alive = false;
                    } else {
                        assert!(self.units.get(&attack_info.defender_id).is_some());
                        self.units.remove(&attack_info.defender_id);
                    }
                }
                let attacker_id = match attack_info.attacker_id {
                    Some(attacker_id) => attacker_id,
                    None => return,
                };
                if let Some(unit) = self.units.get_mut(&attacker_id) {
                    match attack_info.mode {
                        FireMode::Active => {
                            if let Some(ref mut attack_points) = unit.attack_points {
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
            },
            CoreEvent::ShowUnit{ref unit_info} => {
                self.add_unit(db, unit_info, InfoLevel::Partial);
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
                if let Some(unit) = self.units.get_mut(&unit_info.unit_id) {
                    unit.pos = unit_info.pos;
                    return;
                }
                self.add_unit(db, unit_info, InfoLevel::Partial);
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
        let slot_id = get_free_slot_for_building(map, objects, pos).unwrap();
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
        ((0, 0), 0),
        ((0, 1), 0),
        ((9, 2), 1),
        ((9, 3), 1),
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
