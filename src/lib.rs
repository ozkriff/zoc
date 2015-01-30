// See LICENSE file for copyright and license details.

#![feature(rand, hash, path, io, libc, core, std_misc, collections)] // TODO
#![feature(box_syntax)]

#[cfg(target_os = "android")]
#[macro_use]
extern crate android_glue;

extern crate libc;
extern crate time;
extern crate cgmath;
extern crate "zoc_gl" as gl;
extern crate glutin;
extern crate image;
extern crate stb_tt;

use visualizer::{Visualizer};

mod core;
mod visualizer;

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

// vim: set tabstop=5 shiftwidth=4 softtabstop=4 expandtab:
