#[cfg(target_os = "android")]
#[macro_use]
extern crate android_glue;

#[macro_use]
extern crate gfx;

extern crate gfx_window_glutin as gfx_glutin;
extern crate gfx_device_gl as gfx_gl;
extern crate rand;
extern crate time;
extern crate cgmath;
extern crate collision;
extern crate glutin;
extern crate core;
extern crate image;
extern crate rusttype;

mod visualizer;
mod pick;
mod gen;
mod gui;
mod obj;
mod scene;
mod event_visualizer;
mod unit_type_visual_info;
mod mesh_manager;
mod player_info;
mod selection;
mod types;
mod pipeline;
mod map_text;
mod move_helper;
mod camera;
mod geom;
mod screen;
mod texture;
mod tactical_screen;
mod context_menu_popup;
mod reinforcements_popup;
mod main_menu_screen;
mod end_turn_screen;
mod game_results_screen;
mod context;
mod text;
mod mesh;
mod fs;

use std::env;
use std::thread;
use std::time as stdtime;
use visualizer::{Visualizer};

const MAX_FPS: u64 = 60;

pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    let mut visualizer = Visualizer::new();

    let max_frame_time = stdtime::Duration::from_millis(1000 / MAX_FPS);

    while visualizer.is_running() {
        let start = stdtime::Instant::now();
        visualizer.tick();

        let delta = start.elapsed();
        if max_frame_time > delta {
            println!("Sleeping for: {:?}", max_frame_time - delta);
            thread::sleep(max_frame_time - delta);
        }
    }
}
