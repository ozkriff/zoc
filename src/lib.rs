#![feature(phase)]
#![feature(macro_rules)]

#[cfg(target_os = "android")]
#[phase(plugin, link)]
extern crate android_glue;

extern crate libc;

extern crate cgmath;

mod visualizer;
mod visualizer_types;
mod core_types;
mod core_misc;
mod mgl;
mod gl;

pub fn main() {
    let mut visualizer = visualizer::Visualizer::new();
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
