// See LICENSE file for copyright and license details.

use std::iter::{repeat};
use std::num::{Float, SignedInt};
use cgmath::{Vector};
use common::types::{Size2, ZInt, MapPos};
use dir::{Dir, DirIter, dirs};

#[derive(Clone)]
pub enum Terrain {
    Plain,
    Trees,
}

pub struct Map<T> {
    tiles: Vec<T>,
    size: Size2<ZInt>,
}

impl<T: Clone> Map<T> {
    // TODO: remove 'empty'
    pub fn new(size: &Size2<ZInt>, empty: T) -> Map<T> {
        let tiles_count = size.w * size.h;
        let tiles = repeat(empty).take(tiles_count as usize).collect();
        Map {
            tiles: tiles,
            size: size.clone(),
        }
    }

    pub fn size(&self) -> &Size2<ZInt> {
        &self.size
    }

    pub fn tile_mut(&mut self, pos: &MapPos) -> &mut T {
        assert!(self.is_inboard(pos));
        let index = self.size.w * pos.v.y + pos.v.x;
        &mut self.tiles[index as usize]
    }

    pub fn tile(&self, pos: &MapPos) -> &T {
        assert!(self.is_inboard(pos));
        let index = self.size.w * pos.v.y + pos.v.x;
        &self.tiles[index as usize]
    }

    pub fn is_inboard(&self, pos: &MapPos) -> bool {
        let x = pos.v.x;
        let y = pos.v.y;
        x >= 0 && y >= 0 && x < self.size.w && y < self.size.h
    }

    pub fn get_iter(&self) -> MapPosIter {
        MapPosIter::new(self.size())
    }
}

pub struct MapPosIter {
    cursor: MapPos,
    map_size: Size2<ZInt>,
}

impl MapPosIter {
    fn new(map_size: &Size2<ZInt>) -> MapPosIter {
        MapPosIter {
            cursor: MapPos{v: Vector::from_value(0)},
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

pub struct RingIter {
    cursor: MapPos,
    segment_index: ZInt,
    dir_iter: DirIter,
    radius: ZInt,
    dir: Dir,
}

pub fn ring_iter(pos: &MapPos, radius: ZInt) -> RingIter {
    let mut pos = pos.clone();
    pos.v.x -= radius;
    let mut dir_iter = dirs();
    let dir = dir_iter.next()
        .expect("Can`t get first direction");
    assert_eq!(dir, Dir::SouthEast);
    RingIter {
        cursor: pos.clone(),
        radius: radius,
        segment_index: 0,
        dir_iter: dir_iter,
        dir: dir,
    }
}

impl RingIter {
    fn simple_step(&mut self) -> Option<MapPos> {
        self.cursor = Dir::get_neighbour_pos(
            &self.cursor, &self.dir);
        self.segment_index += 1;
        Some(self.cursor.clone())
    }

    fn rotate(&mut self, dir: Dir) -> Option<MapPos> {
        self.segment_index = 0;
        self.cursor = Dir::get_neighbour_pos(&self.cursor, &self.dir);
        self.dir = dir;
        Some(self.cursor.clone())
    }
}

impl Iterator for RingIter {
    type Item = MapPos;

    fn next(&mut self) -> Option<MapPos> {
        if self.segment_index >= self.radius - 1 {
            if let Some(dir) = self.dir_iter.next() {
                self.rotate(dir)
            } else {
                if self.segment_index == self.radius {
                    None
                } else {
                    // last pos
                    self.simple_step()
                }
            }
        } else {
            self.simple_step()
        }
    }
}

pub struct SpiralIter {
    ring_iter: RingIter,
    radius: ZInt,
    last_radius: ZInt,
    origin: MapPos,
}

pub fn spiral_iter(pos: &MapPos, radius: ZInt) -> SpiralIter {
    assert!(radius >= 1);
    SpiralIter {
        ring_iter: ring_iter(pos, 1),
        radius: 1,
        last_radius: radius,
        origin: pos.clone(),
    }
}

impl Iterator for SpiralIter {
    type Item = MapPos;

    fn next(&mut self) -> Option<MapPos> {
        let pos = self.ring_iter.next();
        if pos.is_some() {
            pos
        } else {
            self.radius += 1;
            if self.radius > self.last_radius {
                None
            } else {
                self.ring_iter = ring_iter(
                    &self.origin, self.radius);
                self.ring_iter.next()
            }
        }
    }
}

pub fn distance(from: &MapPos, to: &MapPos) -> ZInt {
    let to = to.v;
    let from = from.v;
    let dx = (to.x + to.y / 2) - (from.x + from.y / 2);
    let dy = to.y - from.y;
    (dx.abs() + dy.abs() + (dx - dy).abs()) / 2
}

#[cfg(test)]
mod tests {
    use cgmath::{Vector2};
    use common::types::{MapPos};
    use super::{ring_iter, spiral_iter};

    #[test]
    fn test_ring_1() {
        let radius = 1;
        let start_pos = MapPos{v: Vector2{x: 0, y: 0}};
        let expected = [
            (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 0) ];
        let mut expected = expected;
        for p in ring_iter(&start_pos, radius) {
            let expected = expected.next().expect(
                "Can not get next element from expected vector");
            assert_eq!(*expected, (p.v.x, p.v.y));
        }
        assert!(expected.next().is_none());
    }

    #[test]
    fn test_ring_2() {
        let radius = 2;
        let start_pos = MapPos{v: Vector2{x: 0, y: 0}};
        let expected = [
            (-1, -1),
            (-1, -2),
            (0, -2),
            (1, -2),
            (2, -1),
            (2, 0),
            (2, 1),
            (1, 2),
            (0, 2),
            (-1, 2),
            (-1, 1),
            (-2, 0),
        ];
        let mut expected = expected;
        for p in ring_iter(&start_pos, radius) {
            let expected = expected.next().expect(
                "Can not get next element from expected vector");
            assert_eq!(*expected, (p.v.x, p.v.y));
        }
        assert!(expected.next().is_none());
    }

    #[test]
    fn test_spiral_1() {
        let radius = 2;
        let start_pos = MapPos{v: Vector2{x: 0, y: 0}};
        let expected = [
            // ring 1
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 0),
            // ring 2
            (-1, -1),
            (-1, -2),
            (0, -2),
            (1, -2),
            (2, -1),
            (2, 0),
            (2, 1),
            (1, 2),
            (0, 2),
            (-1, 2),
            (-1, 1),
            (-2, 0),
        ];
        let mut expected = expected;
        for p in spiral_iter(&start_pos, radius) {
            let expected = expected.next().expect(
                "Can not get next element from expected vector");
            assert_eq!(*expected, (p.v.x, p.v.y));
        }
        assert!(expected.next().is_none());
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
