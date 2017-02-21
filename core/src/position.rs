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
    let slot_id = match get_free_slot_id(state, unit_type, pos) {
        Some(id) => id,
        None => return None,
    };
    Some(ExactPos{map_pos: pos, slot_id: slot_id})
}

pub fn get_free_slot_id(
    state: &State,
    unit_type: &UnitType,
    pos: MapPos,
) -> Option<SlotId> {
    let objects_at = state.objects_at(pos);
    let units_at = state.units_at(pos);
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

pub fn is_exact_pos_free(
    state: &State,
    unit_type: &UnitType,
    pos: ExactPos,
) -> bool {
    let units_at = state.units_at(pos.map_pos);
    if unit_type.is_big && !unit_type.is_air {
        return units_at.count() == 0;
    }
    for unit in units_at {
        if unit.pos == pos {
            return false;
        }
        match unit.pos.slot_id {
            SlotId::WholeTile | SlotId::TwoTiles(_) => {
                if !unit_type.is_air {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}
