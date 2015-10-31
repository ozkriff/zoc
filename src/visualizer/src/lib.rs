// See LICENSE file for copyright and license details.

extern crate num;
extern crate rand;
extern crate time;
extern crate cgmath;
extern crate glutin;
extern crate common;
extern crate core;
extern crate zgl;

mod gui;
mod scene;
mod event_visualizer;
mod unit_type_visual_info;
mod selection;
mod map_text;
mod move_helper;
mod geom;
mod screen;
mod tactical_screen;
mod main_menu_screen;
mod context;

use glutin::{WindowBuilder};
use zgl::{Zgl, Time, Color3};
use screen::{Screen, ScreenCommand};
use context::{Context};
use main_menu_screen::{MainMenuScreen};

fn make_window() -> glutin::Window {
    let gl_version = glutin::GlRequest::GlThenGles {
        opengles_version: (2, 0),
        opengl_version: (2, 0)
    };
    let window_builder = WindowBuilder::new()
        .with_title("Zone of Control".to_owned())
        .with_pixel_format(24, 8)
        .with_gl(gl_version);
    let window = window_builder.build()
        .expect("Can`t create window");
    unsafe {
        window.make_current()
            .expect("Can`t make window current");
    };
    window
}

pub struct Visualizer {
    screens: Vec<Box<Screen>>,
    should_close: bool,
    last_time: Time,
    context: Context,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window = make_window();
        let zgl = Zgl::new(|s| window.get_proc_address(s));
        let mut context = Context::new(zgl, window);
        let screens = vec![
            Box::new(MainMenuScreen::new(&mut context)) as Box<Screen>,
        ];
        Visualizer {
            screens: screens,
            should_close: false,
            last_time: Time{n: time::precise_time_ns()},
            context: context,
        }
    }

    pub fn tick(&mut self) {
        self.draw();
        self.handle_events();
        self.handle_commands();
    }

    fn draw(&mut self) {
        let dtime = self.update_time();
        let bg_color = Color3{r: 0.8, g: 0.8, b: 0.8};
        self.context.zgl.set_clear_color(&bg_color);
        self.context.zgl.clear_screen();
        {
            let screen = self.screens.last_mut().unwrap();
            screen.tick(&mut self.context, &dtime);
        }
        self.context.window.swap_buffers()
            .expect("Can`t swap buffers");
    }

    fn handle_events(&mut self) {
        let events: Vec<_> = self.context.window.poll_events().collect();
        for event in &events {
            self.context.handle_event_pre(event);
            {
                let screen = self.screens.last_mut().unwrap();
                screen.handle_event(&mut self.context, event);
            }
            self.context.handle_event_post(event);
        }
    }

    fn handle_commands(&mut self) {
        let commands = self.context.get_commnands();
        for command in commands {
            match command {
                ScreenCommand::PushScreen(screen) => {
                    self.screens.push(screen);
                },
                ScreenCommand::PopScreen => {
                    let _ = self.screens.pop();
                    if self.screens.is_empty() {
                        self.should_close = true;
                    }
                },
            }
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close && !self.context.should_close()
    }

    fn update_time(&mut self) -> Time {
        let time = time::precise_time_ns();
        let dtime = Time{n: time - self.last_time.n};
        self.last_time = Time{n: time};
        dtime
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
