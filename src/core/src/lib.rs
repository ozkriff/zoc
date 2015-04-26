// See LICENSE file for copyright and license details.

#![feature(core, collections)] // TODO

extern crate num;
extern crate cgmath;
extern crate rand;
extern crate common;

pub mod geom;
pub mod map;
pub mod command;
pub mod db;
pub mod unit;
pub mod dir;
pub mod game_state;
pub mod core;
pub mod pathfinder;

mod ai;
mod player;
mod fov;
mod fow;
mod internal_state;

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
