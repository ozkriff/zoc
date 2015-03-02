// See LICENSE file for copyright and license details.

use cgmath::{Vector3, Vector2};
// use std::cmp::Ord;

#[derive(Clone)]
pub struct Size2<T>{
    pub w: T,
    pub h: T,
}

pub type ZInt = i32;
pub type ZFloat = f32;

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone)]
pub struct PlayerId{pub id: ZInt}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct UnitId{pub id: ZInt}

#[derive(PartialEq, Clone, Debug)]
pub struct MapPos{pub v: Vector2<ZInt>}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
