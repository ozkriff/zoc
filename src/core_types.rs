// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
// use std::cmp::Ord;

#[deriving(Clone)]
pub struct Size2<T>{
    pub w: T,
    pub h: T,
}

pub type ZInt = i32;

/*
#[deriving(PartialOrd, PartialEq, Eq, Hash, Clone)]
pub struct PlayerId{pub id: ZInt}

#[deriving(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct UnitId{pub id: ZInt}
*/

#[deriving(PartialEq, Clone, Show)]
pub struct MapPos{pub v: Vector2<ZInt>}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
