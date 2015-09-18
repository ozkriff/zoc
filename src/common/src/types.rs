// See LICENSE file for copyright and license details.

use std::fmt;
use cgmath::{Vector2};

pub type ZInt = i32;
pub type ZFloat = f32;

#[derive(Clone)]
pub struct Size2 {
    pub w: ZInt,
    pub h: ZInt,
}

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone)]
pub struct PlayerId{pub id: ZInt}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct UnitId{pub id: ZInt}

#[derive(PartialEq, Clone, Debug)]
pub struct MapPos{pub v: Vector2<ZInt>}

impl fmt::Display for MapPos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MapPos({}, {})", self.v.x, self.v.y)
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
