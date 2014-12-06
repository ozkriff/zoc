#[phase(plugin)]
extern crate gl_generator;

extern crate glutin;

use self::glutin::{Event, VirtualKeyCode}; // TODO: why 'self'?

mod gl {
    generate_gl_bindings! {
        api: "gles2",
        profile: "core",
        version: "2.0",
        generator: "static_struct",
    }
}

static mut COLOR_COUNTER: i32 = 0; // TODO

pub struct Visualizer {
    gl: gl::Gles2,
    window: glutin::Window,
    should_close: bool,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window = glutin::Window::new().unwrap();
        unsafe {
            window.make_current();
        };
        let gl = gl::Gles2::load_with(|s| window.get_proc_address(s));
        Visualizer {
            gl: gl,
            window: window,
            should_close: false,
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close
    }

    fn handle_events(&mut self) {
        let events = self.window.poll_events().collect::<Vec<_>>();
        if !events.is_empty() {
            println!("{}", events);
        }
        for event in events.iter() {
            match *event {
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape))
                    | Event::Closed =>
                {
                    self.should_close = true;
                },
                _ => {},
            }
        }
    }

    fn draw(&self) {
        unsafe {
            match COLOR_COUNTER {
                0 => self.gl.ClearColor(0.3, 0.0, 0.0, 1.0),
                30 => self.gl.ClearColor(0.0, 0.3, 0.0, 1.0),
                60 => self.gl.ClearColor(0.0, 0.0, 0.3, 1.0),
                _ => if COLOR_COUNTER > 90 { COLOR_COUNTER = -1; }
            }
            COLOR_COUNTER += 1;
            assert!(self.gl.GetError() == 0);
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
        }
        self.window.swap_buffers();
    }

    pub fn tick(&mut self) {
        self.handle_events();
        // self.logic();
        self.draw();
        // self.update_time();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
