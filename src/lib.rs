#![feature(phase)]

#[cfg(target_os = "android")]
#[phase(plugin, link)]
extern crate android_glue;

mod visualizer;

pub fn main() {
    let mut visualizer = visualizer::Visualizer::new();
    while visualizer.is_running() {
        visualizer.tick();
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn rust_android_main(app: *mut()) {
    android_glue::android_main2(app, proc() main());
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
