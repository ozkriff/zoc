// See LICENSE file for copyright and license details.

use std::default::{Default};
use common::types::{ZInt, MapPos, Size2};
use db::{Db};
use unit::{Unit, UnitClass};
use map::{Map, Terrain};
use partial_state::{PartialState};
use game_state::{GameState};
use dir::{Dir};
use ::{MovePoints};

#[derive(Clone)]
pub struct Tile {
    cost: MovePoints,
    parent: Option<Dir>,
}

impl Tile {
    pub fn parent(&self) -> &Option<Dir> { &self.parent }
    pub fn cost(&self) -> &MovePoints { &self.cost }
}

impl Default for Tile {
    fn default() -> Tile {
        Tile {
            cost: MovePoints{n: 0},
            parent: None,
        }
    }
}

pub fn truncate_path(db: &Db, state: &PartialState, path: &[MapPos], unit: &Unit) -> Vec<MapPos> {
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

pub fn path_cost<S: GameState>(db: &Db, state: &S, unit: &Unit, path: &[MapPos])
    -> MovePoints
{
    let mut cost = MovePoints{n: 0};
    for node in path {
        cost.n += tile_cost(db, state, unit, node).n;
    }
    cost
}

const MAX_COST: MovePoints = MovePoints{n: 30000};

pub fn tile_cost<S: GameState>(db: &Db, state: &S, unit: &Unit, pos: &MapPos)
    -> MovePoints
{
    let unit_type = db.unit_type(&unit.type_id);
    let tile = state.map().tile(pos);
    let n = match unit_type.class {
        UnitClass::Infantry => match tile {
            &Terrain::Plain => 1,
            &Terrain::Trees => 2,
        },
        UnitClass::Vehicle => match tile {
            &Terrain::Plain => 1,
            &Terrain::Trees => 5,
        },
    };
    MovePoints{n: n}
}

pub struct Pathfinder {
    queue: Vec<MapPos>,
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
        original_pos: &MapPos,
        neighbour_pos: &MapPos
    ) {
        let old_cost = self.map.tile(original_pos).cost.clone();
        let tile_cost = tile_cost(db, state, unit, neighbour_pos);
        let tile = self.map.tile_mut(neighbour_pos);
        let new_cost = MovePoints{n: old_cost.n + tile_cost.n};
        let units_count = state.units_at(neighbour_pos).len();
        if tile.cost.n > new_cost.n && units_count == 0 {
            tile.cost = new_cost;
            tile.parent = Some(Dir::get_dir_from_to(
                neighbour_pos, original_pos));
            self.queue.push(neighbour_pos.clone());
        }
    }

    fn clean_map(&mut self) {
        for pos in self.map.get_iter() {
            let tile = self.map.tile_mut(&pos);
            tile.cost = MAX_COST;
            tile.parent = None;
        }
    }

    fn try_to_push_neighbours(
        &mut self,
        db: &Db,
        state: &PartialState,
        unit: &Unit,
        pos: MapPos,
    ) {
        assert!(self.map.is_inboard(&pos));
        for i in 0 .. 6 {
            let dir = Dir::from_int(i as ZInt);
            let neighbour_pos = Dir::get_neighbour_pos(&pos, &dir);
            if self.map.is_inboard(&neighbour_pos) {
                self.process_neighbour_pos(
                    db, state, unit, &pos, &neighbour_pos);
            }
        }
    }

    fn push_start_pos_to_queue(&mut self, start_pos: MapPos) {
        let start_tile = self.map.tile_mut(&start_pos);
        start_tile.cost = MovePoints{n: 0};
        start_tile.parent = None;
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
    pub fn is_reachable(&self, pos: &MapPos) -> bool {
        self.map.tile(pos).cost.n != MAX_COST.n
    }
    */

    pub fn get_path(&self, destination: &MapPos) -> Option<Vec<MapPos>> {
        let mut path = Vec::new();
        let mut pos = destination.clone();
        if self.map.tile(&pos).cost.n == MAX_COST.n {
            return None;
        }
        while self.map.tile(&pos).cost.n != 0 {
            assert!(self.map.is_inboard(&pos));
            path.push(pos.clone());
            let parent_dir = match self.map.tile(&pos).parent() {
                &Some(ref dir) => dir,
                &None => return None,
            };
            pos = Dir::get_neighbour_pos(&pos, parent_dir);
        }
        path.reverse();
        Some(path)
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
