use std::{fmt};
use std::collections::{HashMap};
use cgmath::{Vector2};
use dir::{Dir};
use game_state::{State, ObjectsAtIter};
use map::{Map, Terrain};
use unit::{self, UnitId, Unit, UnitType};
use object::{Object, ObjectId, ObjectClass};
use player::{PlayerId};

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

// TODO: return iterator?
impl From<ExactPos> for MapPos {
    fn from(pos: ExactPos) -> MapPos {
        pos.map_pos
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
        |&(_, unit)| unit::is_commandable(player_id, unit));
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
        |&(_, unit)| unit::is_commandable(player_id, unit)).peekable();
    while let Some((&id, _)) = i.next() {
        let &(&next_id, _) = i.peek().unwrap();
        if next_id == unit_id {
            return id;
        }
    }
    unreachable!()
}

pub fn get_unit_ids_at(state: &State, pos: MapPos) -> Vec<UnitId> {
    let mut ids = Vec::new();
    for unit in state.units_at(pos) {
        if !unit::is_loaded_or_attached(unit) {
            ids.push(unit.id)
        }
    }
    ids
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
    state: &State,
    unit_type: &UnitType,
    pos: MapPos,
) -> Option<ExactPos> {
    let slot_ids = [
        SlotId::Id(0),
        SlotId::Id(1),
        SlotId::Id(2),
        SlotId::WholeTile,
        SlotId::Air,
    ];
    for &slot_id in &slot_ids {
        let exact_pos = ExactPos{map_pos: pos, slot_id: slot_id};
        if can_place_unit(state, unit_type, exact_pos) {
            return Some(exact_pos);
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

fn can_place_air_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    assert!(unit_type.is_air);
    if pos.slot_id != SlotId::Air {
        return false;
    }
    for unit in state.units_at(pos.map_pos) {
        if unit.pos.slot_id == SlotId::Air {
            return false;
        }
    }
    true
}

fn can_place_big_ground_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    // TODO: forbid placing on bridge tile
    assert!(unit_type.is_big);
    if pos.slot_id != SlotId::WholeTile {
        return false;
    }
    for object in state.objects_at(pos.map_pos) {
        if object.class == ObjectClass::Building {
            return false;
        }
    }
    // check if there're any other ground units
    for unit in state.units_at(pos.map_pos) {
        if unit.pos.slot_id != SlotId::Air {
            return false;
        }
    }
    true
}

fn can_place_small_ground_vehicle_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    assert!(!unit_type.is_infantry);
    // TODO: add assert that it's actually a small ground vehicle
    let objects_at = state.objects_at(pos.map_pos);
    for object in objects_at {
        match object.pos.slot_id {
            SlotId::Id(_) => {
                if object.pos == pos {
                    return false;
                }
            },
            SlotId::WholeTile => {
                if object.class == ObjectClass::Building {
                    return false;
                }
            }
            SlotId::TwoTiles(_) | SlotId::Air => {},
        }
    }
    true
}

fn can_place_small_ground_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    match pos.slot_id {
        SlotId::Id(_) => {},
        _ => return false, // TODO: convert this match to assert?
    }
    let slots_count = get_slots_count(state.map(), pos.map_pos);
    let units_at = state.units_at(pos.map_pos);
    let ground_units_count = units_at.clone()
        .filter(|unit| unit.pos.slot_id != SlotId::Air)
        .count();
    if slots_count == 1 && ground_units_count > 0 {
        return false;
    }
    if !unit_type.is_infantry
        && !can_place_small_ground_vehicle_unit(state, unit_type, pos)
    {
        return false;
    }
    for unit in units_at {
        if unit.pos == pos || unit.pos.slot_id == SlotId::WholeTile {
            return false;
        }
    }
    true
}

fn can_place_ground_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    // TODO: forbid placing on water tiles without bridge
    // TODO: check max move points
    if pos.slot_id == SlotId::Air {
        return false;
    }
    if unit_type.is_big {
        can_place_big_ground_unit(state, unit_type, pos)
    } else {
        can_place_small_ground_unit(state, unit_type, pos)
    }
}

pub fn can_place_unit(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    if unit_type.is_air {
        can_place_air_unit(state, unit_type, pos)
    } else {
        can_place_ground_unit(state, unit_type, pos)
    }
}
