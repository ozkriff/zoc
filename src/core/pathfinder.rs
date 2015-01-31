// See LICENSE file for copyright and license details.

use core::types::{ZInt, MapPos, Size2};
use core::core::{ObjectTypes, Unit, UnitClass};
use core::map;
use core::game_state::{GameState};
use core::dir::{Dir};

#[derive(Clone)]
pub struct MoveCost{pub n: ZInt}

#[derive(Clone)]
pub struct MapPath {
    nodes: Vec<(MoveCost, MapPos)>,
}

impl MapPath {
    pub fn new(nodes: Vec<(MoveCost, MapPos)>) -> MapPath {
        MapPath{nodes: nodes}
    }

    // pub fn len(&self) -> ZInt {
    //     self.nodes.len() as ZInt
    // }

    pub fn destination(&self) -> &MapPos {
        let &(_, ref pos) = self.nodes.last().unwrap();
        pos
    }

    pub fn nodes(&self) -> &Vec<(MoveCost, MapPos)> {
        &self.nodes
    }

    pub fn total_cost(&self) -> &MoveCost {
        let &(ref total_cost, _) = self.nodes.last().unwrap();
        total_cost
    }
}

struct Tile {
    cost: MoveCost,
    parent: Option<Dir>,
}

impl Tile {
    pub fn parent(&self) -> &Option<Dir> { &self.parent }
    pub fn cost(&self) -> &MoveCost { &self.cost }
}

struct Map {
    size: Size2<ZInt>,
    tiles: Vec<Tile>,
}

const MAX_COST: MoveCost = MoveCost{n: 30000};

impl<'a> Map {
    pub fn tile_mut(&'a mut self, pos: &MapPos) -> &'a mut Tile {
        self.tiles.get_mut((pos.v.x + pos.v.y * self.size.w) as usize)
            .expect("bad tile index")
    }

    pub fn tile(&'a self, pos: &MapPos) -> &'a Tile {
        assert!(self.is_inboard(pos));
        &self.tiles[(pos.v.x + pos.v.y * self.size.w) as usize]
    }

    pub fn is_inboard(&self, pos: &MapPos) -> bool {
        let x = pos.v.x;
        let y = pos.v.y;
        x >= 0 && y >= 0 && x < self.size.w && y < self.size.h
    }

    /*
    pub fn get_size(&self) -> &Size2<ZInt> {
        &self.size
    }
    */
}

pub struct Pathfinder {
    queue: Vec<MapPos>,
    map: Map,
}

fn create_tiles(tiles_count: ZInt) -> Vec<Tile> {
    let mut tiles = Vec::new();
    for _ in range(0, tiles_count) {
        tiles.push(Tile {
            cost: MoveCost{n: 0},
            parent: None,
        });
    }
    tiles
}

impl Pathfinder {
    pub fn new(map_size: &Size2<ZInt>) -> Pathfinder {
        let tiles_count = map_size.w * map_size.h;
        Pathfinder {
            queue: Vec::new(),
            map: Map {
                size: map_size.clone(),
                tiles: create_tiles(tiles_count),
            },
        }
    }

    pub fn get_map(&self) -> &Map {
        &self.map
    }

    fn tile_cost(
        &self,
        object_types: &ObjectTypes,
        state: &GameState,
        unit: &Unit,
        pos: &MapPos,
    ) -> ZInt { // TODO: ZInt -> MoveCost
        let unit_type = object_types.get_unit_type(&unit.type_id);
        let tile = state.map.tile(pos);
        match unit_type.class {
            UnitClass::Infantry => match tile {
                &map::Tile::Plain => 1,
                &map::Tile::Trees => 2,
                &map::Tile::Building => 2,
            },
            UnitClass::Vehicle => match tile {
                &map::Tile::Plain => 1,
                &map::Tile::Trees => 5,
                &map::Tile::Building => 10,
            },
        }
    }

    fn process_neighbour_pos(
        &mut self,
        object_types: &ObjectTypes,
        state: &GameState,
        unit: &Unit,
        original_pos: &MapPos,
        neighbour_pos: &MapPos
    ) {
        let old_cost = self.map.tile(original_pos).cost.clone();
        let tile_cost = self.tile_cost(object_types, state, unit, neighbour_pos);
        let tile = self.map.tile_mut(neighbour_pos);
        let new_cost = MoveCost{n: old_cost.n + tile_cost};
        let units_count = state.units_at(neighbour_pos).len();
        if tile.cost.n > new_cost.n && units_count == 0 {
            tile.cost = new_cost;
            tile.parent = Some(Dir::get_dir_from_to(
                neighbour_pos, original_pos));
            self.queue.push(neighbour_pos.clone());
        }
    }

    fn clean_map(&mut self) {
        for tile in self.map.tiles.iter_mut() {
            tile.cost = MAX_COST;
            tile.parent = None;
        }
    }

    fn try_to_push_neighbours(
        &mut self,
        object_types: &ObjectTypes,
        state: &GameState,
        unit: &Unit,
        pos: MapPos,
    ) {
        assert!(self.map.is_inboard(&pos));
        for i in range(0, 6) {
            let dir = Dir::from_int(i as ZInt);
            let neighbour_pos = Dir::get_neighbour_pos(&pos, &dir);
            if self.map.is_inboard(&neighbour_pos) {
                self.process_neighbour_pos(
                    object_types, state, unit, &pos, &neighbour_pos);
            }
        }
    }

    fn push_start_pos_to_queue(&mut self, start_pos: MapPos) {
        let start_tile = self.map.tile_mut(&start_pos);
        start_tile.cost = MoveCost{n: 0};
        start_tile.parent = None;
        self.queue.push(start_pos);
    }

    pub fn fill_map(&mut self, object_types: &ObjectTypes, state: &GameState, unit: &Unit) {
        assert!(self.queue.len() == 0);
        self.clean_map();
        self.push_start_pos_to_queue(unit.pos.clone());
        while self.queue.len() != 0 {
            let pos = self.queue.remove(0);
            self.try_to_push_neighbours(object_types, state, unit, pos);
        }
    }

    /*
    pub fn is_reachable(&self, pos: &MapPos) -> bool {
        self.map.tile(pos).cost.n != MAX_COST.n
    }
    */

    pub fn get_path(&self, destination: &MapPos) -> Option<MapPath> {
        let mut path = Vec::new();
        let mut pos = destination.clone();
        if self.map.tile(&pos).cost.n == MAX_COST.n {
            return None;
        }
        assert!(self.map.is_inboard(&pos));
        let start_cost = self.map.tile(&pos).cost.clone();
        path.push((start_cost, pos.clone()));
        while self.map.tile(&pos).cost.n != 0 {
            let parent_dir = match self.map.tile(&pos).parent() {
                &Some(ref dir) => dir,
                &None => return None,
            };
            pos = Dir::get_neighbour_pos(&pos, parent_dir);
            assert!(self.map.is_inboard(&pos));
            let cost = self.map.tile(&pos).cost.clone();
            path.push((cost, pos.clone()));
        }
        path.reverse();
        Some(MapPath {
            nodes: path,
        })
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
