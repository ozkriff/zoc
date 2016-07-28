// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
use types::{ZInt};
use ::{MapPos};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    SouthEast,
    East,
    NorthEast,
    NorthWest,
    West,
    SouthWest,
}

const DIR_TO_POS_DIFF: [[Vector2<ZInt>; 6]; 2] = [
    [
        Vector2{x: 1, y: -1},
        Vector2{x: 1, y: 0},
        Vector2{x: 1, y: 1},
        Vector2{x: 0, y: 1},
        Vector2{x: -1, y: 0},
        Vector2{x: 0, y: -1},
    ],
    [
        Vector2{x: 0, y: -1},
        Vector2{x: 1, y: 0},
        Vector2{x: 0, y: 1},
        Vector2{x: -1, y: 1},
        Vector2{x: -1, y: 0},
        Vector2{x: -1, y: -1},
    ]
];

impl Dir {
    pub fn from_int(n: ZInt) -> Dir {
        assert!(n >= 0 && n < 6);
        let dirs = [
            Dir::SouthEast,
            Dir::East,
            Dir::NorthEast,
            Dir::NorthWest,
            Dir::West,
            Dir::SouthWest,
        ];
        dirs[n as usize].clone()
    }

    pub fn to_int(&self) -> ZInt {
        match *self {
            Dir::SouthEast => 0,
            Dir::East => 1,
            Dir::NorthEast => 2,
            Dir::NorthWest => 3,
            Dir::West => 4,
            Dir::SouthWest => 5,
        }
    }

    pub fn get_dir_from_to(from: &MapPos, to: &MapPos) -> Dir {
        // assert!(from.distance(to) == 1);
        let diff = to.v - from.v;
        let is_odd_row = from.v.y % 2 != 0;
        let subtable_index = if is_odd_row { 1 } else { 0 };
        for dir in dirs() {
            if diff == DIR_TO_POS_DIFF[subtable_index][dir.to_int() as usize] {
                return dir;
            }
        }
        panic!("impossible positions: {}, {}", from, to);
    }

    pub fn get_neighbour_pos(pos: &MapPos, dir: &Dir) -> MapPos {
        let is_odd_row = pos.v.y % 2 != 0;
        let subtable_index = if is_odd_row { 1 } else { 0 };
        let direction_index = dir.to_int();
        assert!(direction_index >= 0 && direction_index < 6);
        let difference = DIR_TO_POS_DIFF[subtable_index][direction_index as usize];
        MapPos{v: pos.v + difference}
    }
}

pub struct DirIter {
    index: ZInt,
}

pub fn dirs() -> DirIter {
    DirIter{index: 0}
}

impl Iterator for DirIter {
    type Item = Dir;

    fn next(&mut self) -> Option<Dir> {
        let next_dir = if self.index > 5 {
            None
        } else {
            Some(Dir::from_int(self.index))
        };
        self.index += 1;
        next_dir
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
