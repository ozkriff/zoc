use std::default::{Default};
use std::rc::{Rc};
use types::{Size2};
use db::{Db};
use unit::{Unit};
use map::{Map, Terrain};
use game_state::{State};
use dir::{Dir, dirs};
use ::{MovePoints, ExactPos, SlotId, ObjectClass, get_free_exact_pos};

#[derive(Clone, Debug)]
pub struct Tile {
    cost: MovePoints,
    parent: Option<Dir>,
    slot_id: SlotId,
}

impl Tile {
    pub fn parent(&self) -> Option<Dir> { self.parent }
    pub fn cost(&self) -> MovePoints { self.cost }
    pub fn slot_id(&self) -> SlotId { self.slot_id }
}

impl Default for Tile {
    fn default() -> Tile {
        Tile {
            cost: MovePoints{n: 0},
            parent: None,
            slot_id: SlotId::WholeTile,
        }
    }
}

pub fn truncate_path(db: &Db, state: &State, path: &[ExactPos], unit: &Unit) -> Option<Vec<ExactPos>> {
    let mut new_path = Vec::new();
    let mut cost = MovePoints{n: 0};
    new_path.push(path[0]);
    let move_points = unit.move_points.unwrap();
    for window in path.windows(2) {
        let from = window[0];
        let to = window[1];
        cost.n += tile_cost(db, state, unit, from, to).n;
        if cost.n > move_points.n {
            break;
        }
        new_path.push(to);
    }
    if new_path.len() < 2 {
        None
    } else {
        Some(new_path)
    }
}

pub fn path_cost(db: &Db, state: &State, unit: &Unit, path: &[ExactPos])
    -> MovePoints
{
    let mut cost = MovePoints{n: 0};
    for window in path.windows(2) {
        let from = window[0];
        let to = window[1];
        cost.n += tile_cost(db, state, unit, from, to).n;
    }
    cost
}

// TODO: const (see https://github.com/rust-lang/rust/issues/24111 )
pub fn max_cost() -> MovePoints {
    MovePoints{n: i32::max_value()}
}

// TODO: increase cost for attached units
pub fn tile_cost(db: &Db, state: &State, unit: &Unit, from: ExactPos, pos: ExactPos)
    -> MovePoints
{
    let map_pos = pos.map_pos;
    let objects_at = state.objects_at(map_pos);
    let units_at = state.units_at(map_pos);
    let mut unit_cost = 0;
    let mut object_cost = 0;
    let unit_type = db.unit_type(unit.type_id);
    if unit_type.is_air {
        return MovePoints{n: 2};
    }
    'unit_loop: for unit in units_at {
        for object in objects_at.clone() {
            match object.pos.slot_id {
                SlotId::Id(_) => if unit.pos == object.pos {
                    assert!(db.unit_type(unit.type_id).is_infantry);
                    break 'unit_loop;
                },
                SlotId::TwoTiles(_) | SlotId::WholeTile => {
                    break 'unit_loop;
                },
                SlotId::Air => {},
            }
        }
        unit_cost += 1;
    }
    let tile = state.map().tile(pos);
    let mut terrain_cost = if unit_type.is_infantry {
        match *tile {
            Terrain::Plain | Terrain::City => 4,
            Terrain::Trees => 5,
            Terrain::Water => 99,
        }
    } else {
        match *tile {
            Terrain::Plain | Terrain::City => 4,
            Terrain::Trees => 8,
            Terrain::Water => 99,
        }
    };
    for object in objects_at.clone() {
        if object.class != ObjectClass::Road {
            continue;
        }
        let mut i = object.pos.map_pos_iter();
        let road_from = i.next().unwrap();
        let road_to = i.next().unwrap();
        assert!(road_from != road_to);
        let is_road_pos_ok = road_from == from.map_pos && road_to == pos.map_pos;
        let is_road_pos_rev_ok = road_to == from.map_pos && road_from == pos.map_pos;
        if (is_road_pos_ok || is_road_pos_rev_ok) && !unit_type.is_big {
            // TODO: ultrahardcoded value :(
            terrain_cost = if unit_type.is_infantry { 4 } else { 2 };
        }
    }
    for object in objects_at {
        let cost = if unit_type.is_infantry {
            match object.class {
                ObjectClass::Building => 1,
                ObjectClass::ReinforcementSector |
                ObjectClass::Road |
                ObjectClass::Smoke => 0,
            }
        } else {
            match object.class {
                ObjectClass::Building => 2,
                ObjectClass::ReinforcementSector |
                ObjectClass::Road |
                ObjectClass::Smoke => 0,
            }
        };
        object_cost += cost;
    }
    MovePoints{n: terrain_cost + object_cost + unit_cost}
}

#[derive(Clone, Debug)]
pub struct Pathfinder {
    queue: Vec<ExactPos>,
    map: Map<Tile>,
    db: Rc<Db>,
}

impl Pathfinder {
    pub fn new(db: Rc<Db>, map_size: Size2) -> Pathfinder {
        Pathfinder {
            queue: Vec::new(),
            map: Map::new(map_size),
            db: db,
        }
    }

    pub fn get_map(&self) -> &Map<Tile> {
        &self.map
    }

    fn process_neighbour_pos(
        &mut self,
        state: &State,
        unit: &Unit,
        original_pos: ExactPos,
        neighbour_pos: ExactPos
    ) {
        let old_cost = self.map.tile(original_pos).cost;
        let tile_cost = tile_cost(&self.db, state, unit, original_pos, neighbour_pos);
        let tile = self.map.tile_mut(neighbour_pos);
        let new_cost = MovePoints{n: old_cost.n + tile_cost.n};
        if tile.cost.n > new_cost.n {
            tile.cost = new_cost;
            tile.parent = Some(Dir::get_dir_from_to(
                neighbour_pos.map_pos, original_pos.map_pos));
            tile.slot_id = neighbour_pos.slot_id;
            self.queue.push(neighbour_pos);
        }
    }

    fn clean_map(&mut self) {
        for pos in self.map.get_iter() {
            let tile = self.map.tile_mut(pos);
            tile.cost = max_cost();
            tile.parent = None;
            tile.slot_id = SlotId::WholeTile;
        }
    }

    fn try_to_push_neighbours(
        &mut self,
        state: &State,
        unit: &Unit,
        pos: ExactPos,
    ) {
        assert!(self.map.is_inboard(pos));
        for dir in dirs() {
            let neighbour_pos = Dir::get_neighbour_pos(pos.map_pos, dir);
            if self.map.is_inboard(neighbour_pos) {
                let exact_neighbour_pos = match get_free_exact_pos(
                    &self.db, state, unit.type_id, neighbour_pos
                ) {
                    Some(pos) => pos,
                    None => continue,
                };
                self.process_neighbour_pos(
                    state, unit, pos, exact_neighbour_pos);
            }
        }
    }

    fn push_start_pos_to_queue(&mut self, start_pos: ExactPos) {
        let start_tile = self.map.tile_mut(start_pos);
        start_tile.cost = MovePoints{n: 0};
        start_tile.parent = None;
        start_tile.slot_id = start_pos.slot_id;
        self.queue.push(start_pos);
    }

    pub fn fill_map(&mut self, state: &State, unit: &Unit) {
        assert!(self.queue.len() == 0);
        self.clean_map();
        self.push_start_pos_to_queue(unit.pos);
        while !self.queue.is_empty() {
            let pos = self.queue.remove(0);
            self.try_to_push_neighbours(state, unit, pos);
        }
    }

    /*
    pub fn is_reachable(&self, pos: ExactPos) -> bool {
        self.map.tile(pos).cost.n != max_cost().n
    }
    */

    pub fn get_path(&self, destination: ExactPos) -> Option<Vec<ExactPos>> {
        let mut path = vec![destination];
        let mut pos = destination;
        if self.map.tile(pos).cost.n == max_cost().n {
            return None;
        }
        while self.map.tile(pos).cost.n != 0 {
            assert!(self.map.is_inboard(pos));
            let parent_dir = match self.map.tile(pos).parent() {
                Some(dir) => dir,
                None => return None,
            };
            let neighbour_map_pos = Dir::get_neighbour_pos(pos.map_pos, parent_dir);
            pos = ExactPos {
                map_pos: neighbour_map_pos,
                slot_id: self.map.tile(neighbour_map_pos).slot_id,
            };
            path.push(pos);
        }
        path.reverse();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }
}
