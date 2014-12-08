// See LICENSE file for copyright and license details.

#[phase(plugin)]
extern crate gl_generator;

extern crate glutin;
extern crate cgmath;
extern crate serialize;

use self::glutin::{Event, VirtualKeyCode}; // TODO: why 'self'?
use visualizer_types::{Color3};
use mgl::Mgl;

pub struct Visualizer {
    mgl: Mgl,
    window: glutin::Window,
    should_close: bool,
    color_counter: i32, // TODO: remove
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window = glutin::Window::new().unwrap();
        unsafe {
            window.make_current();
        };
        let mgl = Mgl::new(|s| window.get_proc_address(s));
        Visualizer {
            mgl: mgl,
            window: window,
            should_close: false,
            color_counter: 0,
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

    fn draw(&mut self) {
        match self.color_counter {
            0 => self.mgl.set_clear_color(Color3{r: 0.5, g: 0.0, b: 0.0}),
            30 => self.mgl.set_clear_color(Color3{r: 0.0, g: 0.5, b: 0.0}),
            60 => self.mgl.set_clear_color(Color3{r: 0.0, g: 0.0, b: 0.5}),
            _ => if self.color_counter > 90 {
                self.color_counter = -1;
            },
        }
        self.color_counter += 1;
        self.mgl.clear_screen();
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
