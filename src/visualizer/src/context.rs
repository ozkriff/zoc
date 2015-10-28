// See LICENSE file for copyright and license details.

use std::path::{Path};
use cgmath::{Vector};
use cgmath::{Vector2};
use glutin::{self, Event, MouseButton};
use glutin::ElementState::{Pressed, Released};
use zgl::{Zgl, ColorId, Color4, ScreenPos};
use zgl::font_stash::{FontStash};
use zgl::shader::{Shader};
use common::types::{Size2, ZInt};

static VS_SRC: &'static str = "\
    #version 100\n\
    uniform mat4 mvp_mat;\n\
    attribute vec3 position;\n\
    attribute vec2 in_texture_coordinates;\n\
    varying vec2 texture_coordinates;\n\
    void main() {\n\
        gl_Position = mvp_mat * vec4(position, 1.0);\n\
        gl_PointSize = 2.0;\n\
        texture_coordinates = in_texture_coordinates;\n\
    }\n\
";

static FS_SRC: &'static str = "\
    #version 100\n\
    precision mediump float;\n\
    uniform sampler2D basic_texture;\n\
    uniform vec4 basic_color;
    varying vec2 texture_coordinates;\n\
    void main() {\n\
        gl_FragColor = basic_color\n\
            * texture2D(basic_texture, texture_coordinates);\n\
    }\n\
";

fn get_win_size(window: &glutin::Window) -> Size2 {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as ZInt, h: h as ZInt}
}

pub struct MouseState {
    pub is_left_button_pressed: bool,
    pub is_right_button_pressed: bool,
    pub last_press_pos: ScreenPos,
    pub pos: ScreenPos,
}

// TODO: make more fields private?
pub struct Context {
    pub window: glutin::Window,
    pub win_size: Size2,
    pub zgl: Zgl,
    pub font_stash: FontStash,
    pub shader: Shader,
    pub basic_color_id: ColorId,
    mouse: MouseState,
    should_close: bool,
}

impl Context {
    pub fn new(zgl: Zgl, window: glutin::Window) -> Context {
        let shader = Shader::new(&zgl, VS_SRC, FS_SRC);
        shader.activate(&zgl);
        let basic_color_id = shader.get_uniform_color(&zgl, "basic_color");
        let win_size = get_win_size(&window);
        let font_size = 40.0;
        // TODO: read font name from config
        let font_stash = FontStash::new(
            &zgl, &Path::new("DroidSerif-Regular.ttf"), font_size);
        Context {
            shader: shader,
            zgl: zgl,
            window: window,
            win_size: win_size,
            font_stash: font_stash,
            basic_color_id: basic_color_id,
            should_close: false,
            mouse: MouseState {
                is_left_button_pressed: false,
                is_right_button_pressed: false,
                last_press_pos: ScreenPos{v: Vector::from_value(0)},
                pos: ScreenPos{v: Vector::from_value(0)},
            },
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    pub fn set_basic_color(&self, color: &Color4) {
        self.shader.set_uniform_color(&self.zgl, &self.basic_color_id, color);
    }

    pub fn handle_event_pre(&mut self, event: &glutin::Event) {
        match *event {
            Event::Closed => {
                self.should_close = true;
            },
            Event::MouseInput(Pressed, MouseButton::Left) => {
                self.mouse.is_left_button_pressed = true;
                self.mouse.last_press_pos = self.mouse.pos.clone();
            },
            Event::MouseInput(Released, MouseButton::Left) => {
                self.mouse.is_left_button_pressed = false;
            },
            Event::MouseInput(Pressed, MouseButton::Right) => {
                self.mouse.is_right_button_pressed = true;
            },
            Event::MouseInput(Released, MouseButton::Right) => {
                self.mouse.is_right_button_pressed = false;
            },
            Event::Resized(w, h) => {
                self.win_size = Size2{w: w as ZInt, h: h as ZInt};
                self.zgl.set_viewport(&self.win_size);
            },
            _ => {},
        }
    }

    pub fn handle_event_post(&mut self, event: &glutin::Event) {
        match *event {
            Event::MouseMoved((x, y)) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                self.mouse.pos = pos;
            },
            Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                match phase {
                    glutin::TouchPhase::Moved => {
                        self.mouse.pos = pos;
                    },
                    glutin::TouchPhase::Started => {
                        self.mouse.pos = pos.clone();
                        self.mouse.last_press_pos = pos;
                        self.mouse.is_left_button_pressed = true;
                    },
                    glutin::TouchPhase::Ended => {
                        self.mouse.pos = pos;
                        self.mouse.is_left_button_pressed = false;
                    },
                    glutin::TouchPhase::Cancelled => {
                        unimplemented!();
                    },
                }
            },
            _ => {},
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
