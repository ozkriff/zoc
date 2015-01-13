// See LICENSE file for copyright and license details.

use cgmath::{Vector2, Vector3, deg};
use glutin::{Window, WindowBuilder, VirtualKeyCode, Event};
use glutin::ElementState::{Pressed, Released};
use glutin::MouseButton::{LeftMouseButton};
use core_types::{Size2, ZInt, MapPos};
use visualizer_types::{
    ZFloat,
    Color3,
    Color4,
    ColorId,
    WorldPos,
    ScreenPos,
    VertexCoord,
};
use zgl::{Zgl};
use mesh::{Mesh};
use camera::Camera;
use shader::{Shader};
use geom;
use core_map::{MapPosIter};
use dir::{DirIter};
use picker::{TilePicker, PickResult};

static BG_COLOR: Color3 = Color3{r: 0.0, g: 0.0, b: 0.4};

static VS_SRC: &'static str = "\
    #version 100\n\
    uniform mat4 mvp_mat;\n\
    attribute vec3 position;\n\
    void main() {\n\
        gl_Position = mvp_mat * vec4(position, 1.0);\n\
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

fn get_win_size(window: &Window) -> Size2<ZInt> {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as ZInt, h: h as ZInt}
}

fn get_max_camera_pos(map_size: &Size2<ZInt>) -> WorldPos {
    let pos = geom::map_pos_to_world_pos(
        &MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}});
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

fn generate_mesh(map_size: &Size2<ZInt>, zgl: &Zgl) -> Mesh {
    let mut vertex_data = Vec::new();
    for tile_pos in MapPosIter::new(map_size) {
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in DirIter::new() {
            let num = dir.to_int();
            let vertex = geom::index_to_hex_vertex(num);
            let next_vertex = geom::index_to_hex_vertex(num + 1);
            vertex_data.push(VertexCoord{v: pos.v + vertex.v});
            vertex_data.push(VertexCoord{v: pos.v + next_vertex.v});
            vertex_data.push(VertexCoord{v: pos.v});
        }
    }
    Mesh::new(zgl, vertex_data.as_slice())
}

pub struct Visualizer {
    zgl: Zgl,
    window: Window,
    should_close: bool,
    color_counter: i32, // TODO: remove
    test_color: Color4,
    shader: Shader,
    color_uniform_location: ColorId,
    camera: Camera,
    mouse_pos: ScreenPos,
    is_lmb_pressed: bool,
    win_size: Size2<ZInt>,
    mesh: Mesh,
    picker: TilePicker,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window_builder = WindowBuilder::new().with_gl_version((2, 0));
        let window = window_builder.build().ok().expect("Can`t create window");
        unsafe {
            window.make_current();
        };
        let win_size = get_win_size(&window);
        let mut zgl = Zgl::new(|s| window.get_proc_address(s));
        zgl.print_gl_info();
        let shader = Shader::new(&zgl, VS_SRC, FS_SRC);
        let color_uniform_location = shader.get_uniform_color(&zgl, "col");
        zgl.set_clear_color(&BG_COLOR);
        let mut camera = Camera::new(&win_size);
        let map_size = Size2{w: 5, h: 8};
        camera.set_max_pos(get_max_camera_pos(&map_size));
        let mesh = generate_mesh(&map_size, &zgl);
        let picker = TilePicker::new(&zgl, &map_size);
        Visualizer {
            zgl: zgl,
            window: window,
            should_close: false,
            color_counter: 0,
            test_color: Color4{r: 0.0, g: 0.0, b: 0.0, a: 0.0},
            shader: shader,
            color_uniform_location: color_uniform_location,
            camera: camera,
            mouse_pos: ScreenPos{v: Vector2::from_value(0)},
            is_lmb_pressed: false,
            win_size: win_size,
            mesh: mesh,
            picker: picker,
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close
    }

    fn handle_event_mouse_move(&mut self, pos: &ScreenPos) {
        if self.is_lmb_pressed {
            let diff = pos.v - self.mouse_pos.v;
            let win_w = self.win_size.w as ZFloat;
            let win_h = self.win_size.h as ZFloat;
            self.camera.add_horizontal_angle(
                deg(diff.x as ZFloat * (360.0 / win_w)));
            self.camera.add_vertical_angle(
                deg(diff.y as ZFloat * (360.0 / win_h)));
        }
        self.mouse_pos = pos.clone();
    }

    fn handle_event_key_press(&mut self, key: VirtualKeyCode) {
        match key {
            VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                self.should_close = true;
            },
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.camera.move_camera(deg(270.0), 0.1);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.camera.move_camera(deg(90.0), 0.1);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.camera.move_camera(deg(0.0), 0.1);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.camera.move_camera(deg(180.0), 0.1);
            },
            VirtualKeyCode::Minus => {
                self.camera.change_zoom(1.3);
            },
            VirtualKeyCode::Equals => {
                self.camera.change_zoom(0.7);
            },
            _ => {},
        }
    }

    fn handle_event(&mut self, event: &Event) {
        match *event {
            Event::Closed => {
                self.should_close = true;
            },
            Event::Resized(w, h) => {
                self.win_size = Size2{w: w as ZInt, h: h as ZInt};
                self.zgl.set_viewport(&self.win_size);
            },
            Event::MouseMoved((x, y)) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                self.handle_event_mouse_move(&pos);
            },
            Event::MouseInput(Pressed, LeftMouseButton) => {
                self.is_lmb_pressed = true;
            },
            Event::MouseInput(Released, LeftMouseButton) => {
                self.is_lmb_pressed = false;
                let (r, g, b, a) = self.zgl.read_pixel_bytes(
                    &self.win_size, &self.mouse_pos);
                println!("r: {}, g: {}, b: {}, a: {}", r, g, b, a);
            },
            Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(key);
            },
            _ => {},
        }
    }

    fn handle_events(&mut self) {
        let events = self.window.poll_events().collect::<Vec<_>>();
        if events.is_empty() {
            return;
        }
        // println!("{}", events);
        for event in events.iter() {
            self.handle_event(event);
        }
    }

    fn update_test_color(&mut self) {
        self.test_color = match self.color_counter {
            0 => Color4{r: 1.0, g: 0.0, b: 0.0, a: 1.0},
            30 => Color4{r: 0.0, g: 1.0, b: 0.0, a: 1.0},
            60 => Color4{r: 0.0, g: 0.0, b: 1.0, a: 1.0},
            _ => self.test_color.clone(),
        };
        self.color_counter += 1;
        if self.color_counter > 90 {
            self.color_counter = -1;
        }
    }

    fn draw(&mut self) {
        self.zgl.set_clear_color(&BG_COLOR);
        self.zgl.clear_screen();
        self.shader.activate(&self.zgl);
        self.shader.set_uniform_color(
            &self.zgl, &self.color_uniform_location, &self.test_color);
        self.shader.set_uniform_mat4f(
            &self.zgl,
            self.shader.get_mvp_mat(),
            &self.camera.mat(&self.zgl),
        );
        self.mesh.draw(&self.zgl, &self.shader);
        self.window.swap_buffers();
    }

    fn pick_tile(&mut self) {
        let pick_result = self.picker.pick_tile(
            &mut self.zgl, &self.camera, &self.win_size, &self.mouse_pos);
        match pick_result {
            PickResult::Nothing => {},
            PickResult::MapPos(map_pos) => {
                println!("PICKED: x: {}, y: {}", map_pos.v.x, map_pos.v.y);
            },
        }
    }

    pub fn tick(&mut self) {
        self.handle_events();
        // self.logic();
        self.pick_tile();
        self.update_test_color();
        self.draw();
        // self.update_time();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
