// See LICENSE file for copyright and license details.

use std::default::{Default};
use types::{ZInt, Size2};
use db::{Db};
use unit::{Unit, UnitClass};
use map::{Map, Terrain};
use partial_state::{PartialState};
use game_state::{GameState};
use dir::{Dir, dirs};
use ::{MovePoints, MapPos, ExactPos, SlotId, get_free_exact_pos};

#[derive(Clone)]
pub struct Tile {
    cost: MovePoints,
    parent: Option<Dir>,
    slot_id: SlotId,
}

impl Tile {
    pub fn parent(&self) -> &Option<Dir> { &self.parent }
    pub fn cost(&self) -> &MovePoints { &self.cost }
    pub fn slot_id(&self) -> &SlotId { &self.slot_id }
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

pub fn truncate_path(db: &Db, state: &PartialState, path: &[ExactPos], unit: &Unit) -> Vec<ExactPos> {
    let mut new_path = Vec::new();
    let mut cost = MovePoints{n: 0};
    for pos in path {
        cost.n += tile_cost(db, state, unit, &pos).n;
        if cost.n > unit.move_points.n {
            break;
        }
        new_path.push(pos.clone());
    }
    new_path
}

pub fn path_cost<S: GameState>(db: &Db, state: &S, unit: &Unit, path: &[ExactPos])
    -> MovePoints
{
    let mut cost = MovePoints{n: 0};
    for node in path {
        cost.n += tile_cost(db, state, unit, node).n;
    }
    cost
}

// TODO: const (see https://github.com/rust-lang/rust/issues/24111 )
pub fn max_cost() -> MovePoints {
    MovePoints{n: ZInt::max_value()}
}

pub fn obstacles_count<S: GameState>(
    state: &S,
    pos: &MapPos,
) -> ZInt {
    let units = state.units_at(pos);
    let objects = state.objects_at(pos);
    let mut count = units.len() + objects.len();
    for unit in &units {
        for obj in &objects {
            if unit.pos == obj.pos {
                count -= 1;
            }
        }
    }
    count as ZInt
}

pub fn tile_cost<S: GameState>(db: &Db, state: &S, unit: &Unit, pos: &ExactPos)
    -> MovePoints
{
    let obstacles_count = obstacles_count(state, &pos.map_pos);
    let unit_type = db.unit_type(&unit.type_id);
    let tile = state.map().tile(&pos);
    let n = match unit_type.class {
        UnitClass::Infantry => match tile {
            &Terrain::Plain => 1,
            &Terrain::Trees => 2,
            &Terrain::City => 0,
        },
        UnitClass::Vehicle => match tile {
            &Terrain::Plain => 1,
            &Terrain::Trees => 5,
            &Terrain::City => 0,
        },
    };
    MovePoints{n: n + obstacles_count}
}

pub struct Pathfinder {
    queue: Vec<ExactPos>,
    map: Map<Tile>,
}

impl Pathfinder {
    pub fn new(map_size: &Size2) -> Pathfinder {
        Pathfinder {
            queue: Vec::new(),
            map: Map::new(map_size),
        }
    }

    pub fn get_map(&self) -> &Map<Tile> {
        &self.map
    }

    fn process_neighbour_pos(
        &mut self,
        db: &Db,
        state: &PartialState,
        unit: &Unit,
        original_pos: &ExactPos,
        neighbour_pos: &ExactPos
    ) {
        let old_cost = self.map.tile(&original_pos).cost.clone();
        let tile_cost = tile_cost(db, state, unit, neighbour_pos);
        let tile = self.map.tile_mut(&neighbour_pos);
        let new_cost = MovePoints{n: old_cost.n + tile_cost.n};
        if tile.cost.n > new_cost.n {
            tile.cost = new_cost;
            tile.parent = Some(Dir::get_dir_from_to(
                &neighbour_pos.map_pos, &original_pos.map_pos));
            tile.slot_id = neighbour_pos.slot_id.clone();
            self.queue.push(neighbour_pos.clone());
        }
    }

    fn clean_map(&mut self) {
        for pos in self.map.get_iter() {
            let tile = self.map.tile_mut(&pos);
            tile.cost = max_cost();
            tile.parent = None;
            tile.slot_id = SlotId::WholeTile;
        }
    }

    fn try_to_push_neighbours(
        &mut self,
        db: &Db,
        state: &PartialState,
        unit: &Unit,
        pos: ExactPos,
    ) {
        assert!(self.map.is_inboard(&pos));
        for dir in dirs() {
            let neighbour_pos = Dir::get_neighbour_pos(&pos.map_pos, &dir);
            if self.map.is_inboard(&neighbour_pos) {
                let exact_neighbour_pos = match get_free_exact_pos(
                    db, state, &unit.type_id, &neighbour_pos
                ) {
                    Some(pos) => pos,
                    None => continue,
                };
                self.process_neighbour_pos(
                    db, state, unit, &pos, &exact_neighbour_pos);
            }
        }
    }

    fn push_start_pos_to_queue(&mut self, start_pos: ExactPos) {
        let start_tile = self.map.tile_mut(&start_pos);
        start_tile.cost = MovePoints{n: 0};
        start_tile.parent = None;
        start_tile.slot_id = start_pos.slot_id.clone();
        self.queue.push(start_pos);
    }

    pub fn fill_map(&mut self, db: &Db, state: &PartialState, unit: &Unit) {
        assert!(self.queue.len() == 0);
        self.clean_map();
        self.push_start_pos_to_queue(unit.pos.clone());
        while self.queue.len() != 0 {
            let pos = self.queue.remove(0);
            self.try_to_push_neighbours(db, state, unit, pos);
        }
    }

    /*
    pub fn is_reachable(&self, pos: &ExactPos) -> bool {
        self.map.tile(pos).cost.n != max_cost().n
    }
    */

    pub fn get_path(&self, destination: &ExactPos) -> Option<Vec<ExactPos>> {
        let mut path = Vec::new();
        let mut pos = destination.clone();
        if self.map.tile(&pos).cost.n == max_cost().n {
            return None;
        }
        while self.map.tile(&pos).cost.n != 0 {
            assert!(self.map.is_inboard(&pos));
            path.push(pos.clone());
            let parent_dir = match self.map.tile(&pos).parent() {
                &Some(ref dir) => dir,
                &None => return None,
            };
            let neighbour_map_pos = Dir::get_neighbour_pos(&pos.map_pos, parent_dir);
            pos = ExactPos {
                map_pos: neighbour_map_pos.clone(),
                slot_id: self.map.tile(&neighbour_map_pos).slot_id.clone(),
            };
        }
        path.reverse();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
