#![feature(phase)]

#[phase(plugin, link)]
extern crate android_glue;

#[phase(plugin)]
extern crate gl_generator;

extern crate libc;
extern crate glutin;

mod gl {
    // pub use self::Gles2 as Gl;
    // generate_gl_bindings!(api: "gles2", profile: "core", version: "2.0", generator: "struct")
    pub use self::Gles1 as Gl;
    generate_gl_bindings!(api: "gles1", profile: "core", version: "1.1", generator: "struct")
}

fn main() {
    println!("start");

    let window = glutin::Window::new().unwrap();

    unsafe {
        window.make_current();
    };

    let gl = gl::Gl::load_with(|symbol| {
        window.get_proc_address(symbol)
    });

    println!("glGetError: {}", gl.GetError());

    /*
    println!("get version <");
    let version = {
        use std::c_str::CString;
        unsafe {
            // error here: "task '<unnamed>' panicked at '`GetString` was not loaded', \
            //     /home/ozkriff/marauder/new-android-gl-init-test/src/lib.rs:14"
            CString::new(gl.GetString(gl::VERSION) as *const i8, false)
        }
    };
    println!("get version >");
    let v = match version.as_str() {
        Some(v) => v,
        None => panic!("Can not read version"),
    };
    println!("OpenGL version {}", v);
    */

    while !window.is_closed() {
        println!("draw 1");
        gl.ClearColor(1.0f32, 0.0f32, 1.0f32, 1.0f32);
        println!("draw 2");
        gl.Clear(gl::COLOR_BUFFER_BIT);
        println!("draw 3");

        window.swap_buffers();
        println!("{}", window.wait_events().collect::<Vec<glutin::Event>>());
    }
}

#[no_mangle]
pub fn rust_android_main(app: *mut()) {
    android_glue::android_main2(app, proc() main());
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
