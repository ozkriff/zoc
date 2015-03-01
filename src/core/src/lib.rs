// See LICENSE file for copyright and license details.

#![feature(core, collections, box_syntax)] // TODO

extern crate cgmath;
extern crate rand;
extern crate common;

pub mod geom;
pub mod map;
pub mod ai;
pub mod command;
pub mod player;
pub mod unit;
pub mod object;
pub mod dir;
pub mod game_state;
pub mod core;
pub mod pathfinder;
pub mod fov;

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
