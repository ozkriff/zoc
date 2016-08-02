extern crate visualizer;

use visualizer::{Visualizer};

pub fn main() {
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}
