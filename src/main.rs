#[cfg(target_os = "android")]
#[macro_use]
extern crate android_glue;

#[macro_use]
extern crate gfx;

extern crate gfx_window_glutin as gfx_glutin;
extern crate gfx_device_gl as gfx_gl;
extern crate rand;
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
mod action;
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

use visualizer::{Visualizer};

pub fn main() {
    // TODO: возможно, это выставлять это дело только если никто другой его не задал
    std::env::set_var("RUST_BACKTRACE", "1");
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}
