// See LICENSE file for copyright and license details.

#[phase(plugin)]
extern crate gl_generator;

extern crate glutin;
extern crate cgmath;
extern crate serialize;

use self::glutin::{Event, VirtualKeyCode}; // TODO: why 'self'?
use core_types::{Size2, MInt};
use visualizer_types::{Color3, Color4, ColorId};
use mgl::Mgl;
use mgl;
use std::mem;

// TODO: remove 'gl'
use gl;
use gl::types::{GLfloat, GLuint};

// TODO: fix indent
static VS_SRC: &'static str =
   "#version 100\n\
    attribute vec2 position;\n\
    void main() {\n\
        gl_Position = vec4(position, 0.0, 1.0);\n\
    }";

static FS_SRC: &'static str =
   "#version 100\n\
    precision mediump float;
    uniform vec4 col;\n\
    void main() {\n\
        gl_FragColor = col;\n\
    }";

pub struct Visualizer {
    mgl: Mgl,
    window: glutin::Window,
    should_close: bool,
    color_counter: i32, // TODO: remove
    program: GLuint,
    color_unifrom_location: ColorId,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window = glutin::Window::new().unwrap(); // TODO: unwrap -> expect
        unsafe {
            window.make_current();
        };
        let mgl = Mgl::new(|s| window.get_proc_address(s));
        // TODO: extract to separate func 'print_gl_info'
        println!("GL_VERSION: {}", mgl.get_info(gl::VERSION));
        println!("GL_SHADING_LANGUAGE_VERSION: {}", mgl.get_info(gl::SHADING_LANGUAGE_VERSION));
        println!("GL_VENDOR: {}", mgl.get_info(gl::VENDOR));
        println!("GL_RENDERER: {}", mgl.get_info(gl::RENDERER));
        // println!("GL_EXTENSIONS: {}", mgl.get_info(gl::EXTENSIONS));
        // TODO: extract to separate func 'compile_shaders'
        let vs = mgl::compile_shader(&mgl.gl, VS_SRC, gl::VERTEX_SHADER);
        let fs = mgl::compile_shader(&mgl.gl, FS_SRC, gl::FRAGMENT_SHADER);
        let program = mgl::link_program(&mgl.gl, vs, fs);
        let color_unifrom_location = ColorId {
            id: mgl.get_uniform(program, "col") as GLuint
        };
        Visualizer {
            mgl: mgl,
            window: window,
            should_close: false,
            color_counter: 0,
            program: program,
            color_unifrom_location: color_unifrom_location,
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
        unsafe {
            let vertices: [GLfloat, ..3 * 3] = [
                0.0,  0.5, 0.0,
                0.5, -0.5, 0.0,
                -0.5, -0.5, 0.0,
            ];
            let (w, h) = self.window.get_inner_size().unwrap(); // TODO: unwrap -> expect
            self.mgl.set_viewport(Size2{w: w as MInt, h: h as MInt});
            self.mgl.gl.UseProgram(self.program);
            self.mgl.set_uniform_color(
                self.color_unifrom_location, Color4{r: 1.0, g: 0.0, b: 0.0, a: 1.0});
            self.mgl.gl.VertexAttribPointer(
                0, 3, gl::FLOAT, gl::FALSE, 0, mem::transmute(&vertices));
            self.mgl.gl.EnableVertexAttribArray(0);
            self.mgl.gl.DrawArrays(gl::TRIANGLES, 0, 3);
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
