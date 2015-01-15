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
    TextureCoord,
};
use zgl::{Zgl};
use mesh::{Mesh};
use camera::Camera;
use shader::{Shader};
use geom;
use core_map::{MapPosIter};
use dir::{DirIter};
use picker::{TilePicker, PickResult};
use texture::{Texture};
use obj;

static BG_COLOR: Color3 = Color3{r: 0.0, g: 0.0, b: 0.4};

static VS_SRC: &'static str = "\
    #version 100\n\
    uniform mat4 mvp_mat;\n\
    attribute vec3 position;\n\
    attribute vec2 in_texture_coordinates;\n\
    varying vec2 texture_coordinates;\n\
    void main() {\n\
        gl_Position = mvp_mat * vec4(position, 1.0);\n\
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

fn get_win_size(window: &Window) -> Size2<ZInt> {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as ZInt, h: h as ZInt}
}

fn get_max_camera_pos(map_size: &Size2<ZInt>) -> WorldPos {
    let pos = geom::map_pos_to_world_pos(
        &MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}});
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

fn generate_map_mesh(map_size: &Size2<ZInt>, zgl: &Zgl) -> Mesh {
    let mut vertex_data = Vec::new();
    let mut tex_data = Vec::new();
    for tile_pos in MapPosIter::new(map_size) {
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in DirIter::new() {
            let num = dir.to_int();
            let vertex = geom::index_to_hex_vertex(num);
            let next_vertex = geom::index_to_hex_vertex(num + 1);
            vertex_data.push(VertexCoord{v: pos.v + vertex.v});
            vertex_data.push(VertexCoord{v: pos.v + next_vertex.v});
            vertex_data.push(VertexCoord{v: pos.v});
            tex_data.push(TextureCoord{v: Vector2{x: 0.0, y: 0.0}});
            tex_data.push(TextureCoord{v: Vector2{x: 1.0, y: 0.0}});
            tex_data.push(TextureCoord{v: Vector2{x: 0.5, y: 0.5}});
        }
    }
    let mut mesh = Mesh::new(zgl, vertex_data.as_slice());
    let tex = Texture::new(zgl, &Path::new("data/floor.png"));
    mesh.add_texture(zgl, tex, tex_data.as_slice());
    mesh
}

fn load_unit_mesh(zgl: &Zgl, name: &str) -> Mesh {
    let tex_path = Path::new(format!("data/{}.png", name).as_slice());
    let obj_path = Path::new(format!("data/{}.obj", name).as_slice());
    let tex = Texture::new(zgl, &tex_path);
    let obj = obj::Model::new(&obj_path);
    let mut mesh = Mesh::new(zgl, obj.build().as_slice());
    mesh.add_texture(zgl, tex, obj.build_tex_coord().as_slice());
    mesh
}

pub struct Visualizer {
    zgl: Zgl,
    window: Window,
    should_close: bool,
    shader: Shader,
    color_uniform_location: ColorId,
    camera: Camera,
    mouse_pos: ScreenPos,
    is_lmb_pressed: bool,
    win_size: Size2<ZInt>,
    map_mesh: Mesh,
    picker: TilePicker,
    unit_mesh: Mesh,
    map_pos_under_cursor: Option<MapPos>,
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
        zgl.init_opengl();
        zgl.print_gl_info();
        let mut shader = Shader::new(&zgl, VS_SRC, FS_SRC);
        shader.enable_texture_coords(&zgl);
        shader.activate(&zgl);
        let color_uniform_location = shader.get_uniform_color(
            &zgl, "basic_color");
        zgl.set_clear_color(&BG_COLOR);
        let mut camera = Camera::new(&win_size);
        let map_size = Size2{w: 5, h: 8};
        camera.set_max_pos(get_max_camera_pos(&map_size));
        let map_mesh = generate_map_mesh(&map_size, &zgl);
        let picker = TilePicker::new(&zgl, &map_size);
        let unit_mesh = load_unit_mesh(&zgl, "tank");
        // let unit_mesh = load_unit_mesh(&zgl, "soldier");
        Visualizer {
            zgl: zgl,
            window: window,
            should_close: false,
            shader: shader,
            color_uniform_location: color_uniform_location,
            camera: camera,
            mouse_pos: ScreenPos{v: Vector2::from_value(0)},
            is_lmb_pressed: false,
            win_size: win_size,
            map_mesh: map_mesh,
            picker: picker,
            unit_mesh: unit_mesh,
            map_pos_under_cursor: None,
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
                match self.map_pos_under_cursor {
                    Some(ref pos) => {
                        println!("Pos: x: {}, y: {}", pos.v.x, pos.v.y);
                    },
                    None => {},
                }
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

    fn draw(&mut self) {
        self.zgl.set_clear_color(&BG_COLOR);
        self.zgl.clear_screen();
        self.shader.activate(&self.zgl);
        self.shader.set_uniform_color(
            &self.zgl, &self.color_uniform_location,
            &Color4{r: 1.0, g: 1.0, b: 1.0, a: 1.0},
        );
        self.shader.set_uniform_mat4f(
            &self.zgl,
            self.shader.get_mvp_mat(),
            &self.camera.mat(&self.zgl),
        );
        self.map_mesh.draw(&self.zgl, &self.shader);
        self.unit_mesh.draw(&self.zgl, &self.shader);
        self.window.swap_buffers();
    }

    fn pick_tile(&mut self) {
        let pick_result = self.picker.pick_tile(
            &mut self.zgl, &self.camera, &self.win_size, &self.mouse_pos);
        self.map_pos_under_cursor = match pick_result {
            PickResult::Nothing => None,
            PickResult::MapPos(map_pos) => Some(map_pos),
        }
    }

    pub fn tick(&mut self) {
        self.handle_events();
        // self.logic();
        self.pick_tile();
        self.draw();
        // self.update_time();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
