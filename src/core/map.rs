// See LICENSE file for copyright and license details.

use std::iter::{repeat};
use std::num::{Float, SignedInt};
use cgmath::{Vector2};
use core::types::{Size2, ZInt, MapPos};

#[derive(Clone)]
pub enum Tile {
    Plain,
    Trees,
    // Swamp,
    Building,
}

pub struct Map {
    tiles: Vec<Tile>,
    size: Size2<ZInt>,
}

impl Map {
    pub fn new(size: &Size2<ZInt>) -> Map {
        let tiles_count = size.w * size.h;
        let tiles = repeat(Tile::Plain).take(tiles_count as usize).collect();
        let mut map = Map {
            tiles: tiles,
            size: size.clone(),
        };
        *map.tile_mut(&MapPos{v: Vector2{x: 1, y: 3}}) = Tile::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 2, y: 3}}) = Tile::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 3, y: 3}}) = Tile::Trees;
        *map.tile_mut(&MapPos{v: Vector2{x: 3, y: 4}}) = Tile::Trees;
        map
    }

    pub fn size(&self) -> &Size2<ZInt> {
        &self.size
    }

    pub fn tile_mut(&mut self, pos: &MapPos) -> &mut Tile {
        let index = self.size.w * pos.v.y + pos.v.x;
        &mut self.tiles[index as usize]
    }

    pub fn tile(&self, pos: &MapPos) -> &Tile {
        let index = self.size.w * pos.v.y + pos.v.x;
        &self.tiles[index as usize]
    }
}

pub struct MapPosIter {
    cursor: MapPos,
    map_size: Size2<ZInt>,
}

impl MapPosIter {
    pub fn new(map_size: &Size2<ZInt>) -> MapPosIter {
        MapPosIter {
            cursor: MapPos{v: Vector2::from_value(0)},
            map_size: map_size.clone(),
        }
    }
}

impl Iterator for MapPosIter {
    type Item = MapPos;

    fn next(&mut self) -> Option<MapPos> {
        let current_pos = if self.cursor.v.y >= self.map_size.h {
            None
        } else {
            Some(self.cursor.clone())
        };
        self.cursor.v.x += 1;
        if self.cursor.v.x >= self.map_size.w {
            self.cursor.v.x = 0;
            self.cursor.v.y += 1;
        }
        current_pos
    }
}

pub fn distance(from: &MapPos, to: &MapPos) -> ZInt {
    let to = to.v;
    let from = from.v;
    let dx = (to.x + to.y / 2) - (from.x + from.y / 2);
    let dy = to.y - from.y;
    (dx.abs() + dy.abs() + (dx - dy).abs()) / 2
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
