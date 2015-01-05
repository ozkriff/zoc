// See LICENSE file for copyright and license details.

// use cgmath::{Vector2};
use core_types::{ZInt/*, MapPos*/};

pub enum Dir {
  NorthEast,
  East,
  SouthEast,
  SouthWest,
  West,
  NorthWest,
}

/*
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
*/

impl Dir {
    pub fn from_int(n: ZInt) -> Dir {
        assert!(n >= 0 && n < 6);
        let dirs = [
            Dir::NorthEast,
            Dir::East,
            Dir::SouthEast,
            Dir::SouthWest,
            Dir::West,
            Dir::NorthWest,
        ];
        dirs[n as uint]
    }

    pub fn to_int(&self) -> ZInt {
        match *self {
            Dir::NorthEast => 0,
            Dir::East => 1,
            Dir::SouthEast => 2,
            Dir::SouthWest => 3,
            Dir::West => 4,
            Dir::NorthWest => 5,
        }
    }

    /*
    pub fn get_dir_from_to(from: MapPos, to: MapPos) -> Dir {
        // assert!(from.distance(to) == 1);
        let diff = to.v - from.v;
        for i in range(0u, 6) {
            if diff == DIR_TO_POS_DIFF[(from.v.y % 2) as uint][i] {
                return Dir::from_int(i as ZInt);
            }
        }
        panic!("impossible positions: {}, {}", from, to);
    }

    pub fn get_neighbour_pos(pos: MapPos, dir: Dir) -> MapPos {
        let is_odd_row = pos.v.y % 2 == 1;
        let subtable_index = if is_odd_row { 1 } else { 0 };
        let direction_index = dir.to_int();
        assert!(direction_index >= 0 && direction_index < 6);
        let difference = DIR_TO_POS_DIFF[subtable_index][direction_index as uint];
        MapPos{v: pos.v + difference}
    }
    */
}

pub struct DirIter {
    index: ZInt,
}

impl DirIter {
    pub fn new() -> Self {
        DirIter{index: 0}
    }
}

impl Iterator for DirIter {
    type Item = Dir;

    fn next(&mut self) -> Option<Dir> {
        let next_dir = if self.index == 6 {
            None
        } else {
            Some(Dir::from_int(self.index))
        };
        self.index += 1;
        next_dir
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
