// See LICENSE file for copyright and license details.

extern crate glutin;
extern crate cgmath;
extern crate serialize;

use core_types::{Size2, MInt};
use visualizer_types::{Color3, Color4, ColorId, MatId};
use mgl::Mgl;
use mgl;
use std::mem;
use camera::Camera;

// TODO: remove 'gl'
use gl;
use gl::types::{GLfloat, GLuint};

static VS_SRC: &'static str = "\
    #version 100\n\
    uniform mat4 mvp_mat;\n\
    attribute vec2 position;\n\
    void main() {\n\
        gl_Position = mvp_mat * vec4(position, 0.0, 1.0);\n\
    }\n\
";

static FS_SRC: &'static str = "\
    #version 100\n\
    precision mediump float;
    uniform vec4 col;\n\
    void main() {\n\
        gl_FragColor = col;\n\
    }\n\
";

fn get_win_size(window: &glutin::Window) -> Size2<MInt> {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as MInt, h: h as MInt}
}

fn print_gl_info(mgl: &Mgl) {
    println!("GL_VERSION: {}", mgl.get_info(gl::VERSION));
    println!("GL_SHADING_LANGUAGE_VERSION: {}", mgl.get_info(gl::SHADING_LANGUAGE_VERSION));
    println!("GL_VENDOR: {}", mgl.get_info(gl::VENDOR));
    println!("GL_RENDERER: {}", mgl.get_info(gl::RENDERER));
    // println!("GL_EXTENSIONS: {}", mgl.get_info(gl::EXTENSIONS));
}

// TODO: Create 'Shader' class
fn compile_shaders(mgl: &Mgl) -> GLuint {
    let vs = mgl::compile_shader(&mgl.gl, VS_SRC, gl::VERTEX_SHADER);
    let fs = mgl::compile_shader(&mgl.gl, FS_SRC, gl::FRAGMENT_SHADER);
    mgl::link_program(&mgl.gl, vs, fs)
}

pub struct Visualizer {
    mgl: Mgl,
    window: glutin::Window,
    should_close: bool,
    color_counter: i32, // TODO: remove
    test_color: Color4,
    program: GLuint,
    color_uniform_location: ColorId,
    mvp_uniform_location: MatId,
    camera: Camera,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window_builder = glutin::WindowBuilder::new().with_gl_version((2, 0));
        let window = window_builder.build().ok().expect("Can`t create window");
        unsafe {
            window.make_current();
        };
        let win_size = get_win_size(&window);
        let mut mgl = Mgl::new(|s| window.get_proc_address(s));
        print_gl_info(&mgl);
        let program = compile_shaders(&mgl);
        let color_uniform_location = ColorId {
            id: mgl.get_uniform(program, "col") as GLuint
        };
        let mvp_uniform_location = MatId {
            id: mgl.get_uniform(program, "mvp_mat") as GLuint
        };
        mgl.set_clear_color(Color3{r: 0.0, g: 0.0, b: 0.4});
        Visualizer {
            mgl: mgl,
            window: window,
            should_close: false,
            color_counter: 0,
            test_color: Color4{r: 0.0, g: 0.0, b: 0.0, a: 0.0},
            program: program,
            color_uniform_location: color_uniform_location,
            mvp_uniform_location: mvp_uniform_location,
            camera: Camera::new(win_size),
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close
    }

    fn handle_events(&mut self) {
        let events = self.window.poll_events().collect::<Vec<_>>();
        if events.is_empty() {
            return;
        }
        println!("{}", events);
        for event in events.iter() {
            match *event {
                glutin::Event::Closed => {
                    self.should_close = true;
                },
                glutin::Event::KeyboardInput(_, _, Some(key)) => match key {
                    glutin::VirtualKeyCode::Q | glutin::VirtualKeyCode::Escape => {
                        self.should_close = true;
                    }
                    _ => {},
                },
                _ => {},
            }
        }
    }

    fn update_test_color(&mut self) {
        self.test_color = match self.color_counter {
            0 => Color4{r: 1.0, g: 0.0, b: 0.0, a: 1.0},
            30 => Color4{r: 0.0, g: 1.0, b: 0.0, a: 1.0},
            60 => Color4{r: 0.0, g: 0.0, b: 1.0, a: 1.0},
            _ => self.test_color,
        };
        self.color_counter += 1;
        if self.color_counter > 90 {
            self.color_counter = -1;
        }
    }

    fn draw(&mut self) {
        self.mgl.clear_screen();
        let vertices: [GLfloat, ..3 * 3] = [
            0.0,  0.5, 0.0,
            0.5, -0.5, 0.0,
            -0.5, -0.5, 0.0,
        ];
        let win_size = get_win_size(&self.window);
        self.mgl.set_viewport(win_size);
        unsafe {
            self.mgl.gl.UseProgram(self.program);
        }
        self.mgl.set_uniform_color(self.color_uniform_location, &self.test_color);
        self.camera.add_z_angle(1.0); // TODO: move to events handling
        self.mgl.set_uniform_mat4f(
            self.mvp_uniform_location,
            &self.camera.mat(&self.mgl),
        );
        unsafe {
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
        self.update_test_color();
        self.draw();
        // self.update_time();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
