// See LICENSE file for copyright and license details.

#![feature(old_path, core, std_misc, box_syntax)] // TODO

extern crate rand;
extern crate time;
extern crate cgmath;
extern crate glutin;
extern crate common;
extern crate core;
extern crate zgl;

mod visualizer;
mod picker;
mod gui;
mod scene;
mod event_visualizer;
mod unit_type_visual_info;
mod selection;
mod map_text;
mod move_helper;
mod geom;

pub use visualizer::{Visualizer};

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
