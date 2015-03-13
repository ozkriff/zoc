// See LICENSE file for copyright and license details.

#![feature(start)]

#[cfg(target_os = "android")]
#[macro_use]
extern crate android_glue;

extern crate visualizer;

use visualizer::{Visualizer};

#[cfg(target_os = "android")]
android_start!(main);

pub fn main() {
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
