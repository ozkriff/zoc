// See LICENSE file for copyright and license details.

#![feature(core, collections, box_syntax)] // TODO

extern crate cgmath;
extern crate rand;
extern crate common;

pub mod geom;
pub mod map;
pub mod command;
pub mod object;
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
