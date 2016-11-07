use std::sync::mpsc::{Sender};
use time::{precise_time_ns};
use cgmath::{Vector2, Matrix4, SquareMatrix, Array};
use glutin::{self, Api, Event, MouseButton, GlRequest};
use glutin::ElementState::{Pressed, Released};
use rusttype;
use gfx::traits::{FactoryExt, Device};
use gfx::handle::{Program};
use gfx;
use gfx_gl;
use gfx_glutin;
use screen::{ScreenCommand};
use types::{Size2, ScreenPos, Time};
use texture::{load_texture_raw};
use pipeline::{pipe};
use fs;
use mesh::{Mesh};

fn new_shader(
    window: &glutin::Window,
    factory: &mut gfx_gl::Factory,
) -> Program<gfx_gl::Resources> {
    let shader_header = match window.get_api() {
        Api::OpenGl => fs::load("shader/pre_gl.glsl").into_inner(),
        Api::OpenGlEs | Api::WebGl => fs::load("shader/pre_gles.glsl").into_inner(),
    };
    let mut vertex_shader = shader_header.clone();
    vertex_shader.extend(fs::load("shader/v.glsl").into_inner());
    let mut fragment_shader = shader_header;
    fragment_shader.extend(fs::load("shader/f.glsl").into_inner());
    factory.link_program(&vertex_shader, &fragment_shader).unwrap()
}

fn new_pso(
    factory: &mut gfx_gl::Factory,
    program: &Program<gfx_gl::Resources>,
    primitive: gfx::Primitive,
) -> gfx::PipelineState<gfx_gl::Resources, pipe::Meta> {
    let rasterizer = gfx::state::Rasterizer::new_fill();
    let pso = factory.create_pipeline_from_program(
        program, primitive, rasterizer, pipe::new());
    pso.unwrap()
}

// TODO: read font name from config
fn new_font() -> rusttype::Font<'static> {
    let font_data = fs::load("DroidSerif-Regular.ttf").into_inner();
    let collection = rusttype::FontCollection::from_bytes(font_data);
    collection.into_font().unwrap()
}

fn get_win_size(window: &glutin::Window) -> Size2 {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as i32, h: h as i32}
}

#[derive(Clone, Debug)]
pub struct MouseState {
    pub is_left_button_pressed: bool,
    pub is_right_button_pressed: bool,
    pub last_press_pos: ScreenPos,
    pub pos: ScreenPos,
}

// TODO: use gfx-rs generics, not gfx_gl types
pub struct Context {
    win_size: Size2,
    mouse: MouseState,
    should_close: bool,
    commands_tx: Sender<ScreenCommand>,
    window: glutin::Window,
    clear_color: [f32; 4],
    device: gfx_gl::Device,
    encoder: gfx::Encoder<gfx_gl::Resources, gfx_gl::CommandBuffer>,
    pso: gfx::PipelineState<gfx_gl::Resources, pipe::Meta>,
    pso_wire: gfx::PipelineState<gfx_gl::Resources, pipe::Meta>,
    factory: gfx_gl::Factory,
    font: rusttype::Font<'static>,
    data: pipe::Data<gfx_gl::Resources>,
    start_time_ns: u64,
}

impl Context {
    pub fn new(tx: Sender<ScreenCommand>) -> Context {
        let gl_version = GlRequest::GlThenGles {
            opengles_version: (2, 0),
            opengl_version: (2, 1),
        };
        let builder = glutin::WindowBuilder::new()
            .with_title("Zone of Control".to_string())
            .with_pixel_format(24, 8)
            .with_gl(gl_version);
        let (window, device, mut factory, main_color, main_depth)
            = gfx_glutin::init(builder);
        let encoder = factory.create_command_buffer().into();
        let program = new_shader(&window, &mut factory);
        let pso = new_pso(&mut factory, &program, gfx::Primitive::TriangleList);
        let pso_wire = new_pso(&mut factory, &program, gfx::Primitive::LineList);
        let sampler = factory.create_sampler_linear();
        let win_size = get_win_size(&window);
        // fake mesh for pipeline initialization
        let vb = factory.create_vertex_buffer(&[]);
        let fake_texture = load_texture_raw(&mut factory, Size2{w: 2, h: 2}, &[0; 4]);
        let data = pipe::Data {
            basic_color: [1.0, 1.0, 1.0, 1.0],
            vbuf: vb,
            texture: (fake_texture, sampler),
            out: main_color,
            out_depth: main_depth,
            mvp: Matrix4::identity().into(),
        };
        Context {
            data: data,
            win_size: win_size,
            clear_color: [0.0, 0.0, 1.0, 1.0],
            window: window,
            device: device,
            factory: factory,
            encoder: encoder,
            pso: pso,
            pso_wire: pso_wire,
            should_close: false,
            commands_tx: tx,
            font: new_font(),
            mouse: MouseState {
                is_left_button_pressed: false,
                is_right_button_pressed: false,
                last_press_pos: ScreenPos{v: Vector2::from_value(0)},
                pos: ScreenPos{v: Vector2::from_value(0)},
            },
            start_time_ns: precise_time_ns(),
        }
    }

    pub fn set_clear_color(&mut self, color: [f32; 4]) {
        self.clear_color = color;
    }

    pub fn clear(&mut self) {
        self.encoder.clear(&self.data.out, self.clear_color);
        self.encoder.clear_depth(&self.data.out_depth, 1.0);
    }

    pub fn current_time(&self) -> Time {
        let ns = precise_time_ns() - self.start_time_ns;
        Time{n: ns as f32 / 1_000_000_000.0}
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn flush(&mut self) {
        self.encoder.flush(&mut self.device);
        self.window.swap_buffers().expect("Can`t swap buffers");
        self.device.cleanup();
    }

    pub fn poll_events(&mut self) -> Vec<glutin::Event> {
        self.window.poll_events().collect()
    }

    pub fn font(&self) -> &rusttype::Font {
        &self.font
    }

    pub fn win_size(&self) -> Size2 {
        self.win_size
    }

    pub fn factory_mut(&mut self) -> &mut gfx_gl::Factory {
        &mut self.factory
    }

    pub fn set_mvp(&mut self, mvp: Matrix4<f32>) {
        self.data.mvp = mvp.into();
    }

    pub fn set_basic_color(&mut self, color: [f32; 4]) {
        self.data.basic_color = color;
    }

    pub fn mouse(&self) -> &MouseState {
        &self.mouse
    }

    pub fn draw_mesh(&mut self, mesh: &Mesh) {
        self.data.texture.0 = mesh.texture().clone();
        self.data.vbuf = mesh.vertex_buffer().clone();
        let pso = if mesh.is_wire() {
            &self.pso_wire
        } else {
            &self.pso
        };
        self.encoder.draw(mesh.slice(), pso, &self.data);
    }

    pub fn add_command(&mut self, command: ScreenCommand) {
        self.commands_tx.send(command)
            .expect("Can't send command to Visualizer");
    }

    pub fn handle_event_pre(&mut self, event: &glutin::Event) {
        match *event {
            Event::Closed => {
                self.should_close = true;
            },
            Event::MouseInput(Pressed, MouseButton::Left) => {
                self.mouse.is_left_button_pressed = true;
                self.mouse.last_press_pos = self.mouse.pos;
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
                if w == 0 || h == 0 {
                    return
                }
                self.win_size = Size2{w: w as i32, h: h as i32};
                gfx_glutin::update_views(
                    &self.window,
                    &mut self.data.out,
                    &mut self.data.out_depth,
                );
            },
            _ => {},
        }
    }

    pub fn handle_event_post(&mut self, event: &glutin::Event) {
        match *event {
            Event::MouseMoved(x, y) => {
                let pos = ScreenPos{v: Vector2{x: x as i32, y: y as i32}};
                self.mouse.pos = pos;
            },
            Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                let pos = ScreenPos{v: Vector2{x: x as i32, y: y as i32}};
                match phase {
                    glutin::TouchPhase::Moved => {
                        self.mouse.pos = pos;
                    },
                    glutin::TouchPhase::Started => {
                        self.mouse.pos = pos;
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
