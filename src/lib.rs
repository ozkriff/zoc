#![feature(phase)]

extern crate libc;
extern crate time;
extern crate native;

#[cfg(target_os = "android")]
#[phase(plugin, link)]
extern crate android_glue;

#[phase(plugin)]
extern crate gl_generator;

extern crate glutin;

mod gl {
    generate_gl_bindings! {
        api: "gles2",
        profile: "core",
        version: "2.0",
        generator: "static", // TODO: global? struct?
    }
}

static mut COLOR_COUNTER: i32 = 0;

pub fn main() {
    let window = glutin::Window::new().unwrap();
    unsafe {
        window.make_current();
    };
    while !window.is_closed() {
        unsafe {
            match COLOR_COUNTER {
                0 => gl::ClearColor(0.3, 0.0, 0.0, 1.0),
                30 => gl::ClearColor(0.0, 0.3, 0.0, 1.0),
                60 => gl::ClearColor(0.0, 0.0, 0.3, 1.0),
                _ => if COLOR_COUNTER > 90 { COLOR_COUNTER = -1; }
            }
            COLOR_COUNTER += 1;
        }
        unsafe {
            assert!(gl::GetError() == 0);
        }
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        window.swap_buffers();
        let events = window.wait_events().collect::<Vec<_>>();
        if !events.is_empty() {
            println!("{}", events);
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn rust_android_main(app: *mut()) {
    android_glue::android_main2(app, proc() main());
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
