// See LICENSE file for copyright and license details.

extern crate visualizer;

use visualizer::{Visualizer};

pub fn main() {
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
