// See LICENSE file for copyright and license details.

#![feature(old_path, old_io, core, std_misc, collections, str_words, box_syntax)] // TODO

#[cfg(target_os = "android")]
#[macro_use]
extern crate android_glue;

extern crate rand;
extern crate libc;
extern crate time;
extern crate cgmath;
extern crate "zoc_gl" as gl;
extern crate glutin;
extern crate image;
extern crate stb_tt;

mod core;
mod visualizer;

use visualizer::{Visualizer};

#[cfg(target_os = "android")]
android_start!(main);

pub fn main() {
    let mut visualizer = Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn rust_android_main(app: *mut()) {
    android_glue::android_main2(app, move|| main());
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
