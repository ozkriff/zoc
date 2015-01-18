// See LICENSE file for copyright and license details.

use time::precise_time_ns;
use std::collections::{HashMap};
use std::num::{SignedInt};
use cgmath::{Vector2, Vector3, deg, Matrix4};
use glutin::{Window, WindowBuilder, VirtualKeyCode, Event};
use glutin::ElementState::{Pressed, Released};
use glutin::MouseButton::{LeftMouseButton};
use core::types::{Size2, ZInt, UnitId, PlayerId, MapPos};
use visualizer::types::{
    ZFloat,
    Color3,
    ColorId,
    WorldPos,
    ScreenPos,
    VertexCoord,
    TextureCoord,
    Time,
};
use visualizer::zgl;
use visualizer::zgl::{Zgl, MeshRenderMode};
use visualizer::mesh::{Mesh, MeshId};
use visualizer::camera::Camera;
use visualizer::shader::{Shader};
use visualizer::geom;
use core::map::{MapPosIter, distance};
use core::dir::{DirIter, Dir};
use core::game_state::GameState;
use core::pathfinder::Pathfinder;
use core::core::{Core, UnitTypeId, CoreEvent, Command};
use visualizer::picker::{TilePicker, PickResult};
use visualizer::texture::{Texture};
use visualizer::obj;
use visualizer::font_stash::{FontStash};
use visualizer::gui::{ButtonManager, Button, ButtonId};
use visualizer::scene::{Scene, SceneNode};
use visualizer::event_visualizer::{
    EventVisualizer,
    EventMoveVisualizer,
    EventEndTurnVisualizer,
    EventCreateUnitVisualizer,
    EventAttackUnitVisualizer,
};
use visualizer::unit_type_visual_info::{
    UnitTypeVisualInfo,
    UnitTypeVisualInfoManager,
};
use visualizer::selection::{SelectionManager, get_selection_mesh};

const BG_COLOR: Color3 = Color3{r: 0.8, g: 0.8, b: 0.8};
const CAMERA_MOVE_SPEED: ZFloat = geom::HEX_EX_RADIUS * 12.0;
const CAMERA_MOVE_SPEED_KEY: ZFloat = geom::HEX_EX_RADIUS;

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

// TODO: Replace all Size2<ZInt> with aliases
fn generate_map_mesh(zgl: &Zgl, map_size: &Size2<ZInt>) -> Mesh {
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

fn build_walkable_mesh(zgl: &Zgl, pathfinder: &Pathfinder) -> Mesh {
    let map = pathfinder.get_map();
    let map_size = map.get_size();
    let mut vertex_data = Vec::new();
    for tile_pos in MapPosIter::new(map_size) {
        if let Some(ref parent_dir) = map.tile(&tile_pos).parent {
            let tile_pos_to = Dir::get_neighbour_pos(&tile_pos, parent_dir);
            let world_pos_from = geom::map_pos_to_world_pos(&tile_pos);
            let world_pos_to = geom::map_pos_to_world_pos(&tile_pos_to);
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_from.v)});
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_to.v)});
        }
    }
    let mut mesh = Mesh::new(zgl, vertex_data.as_slice());
    mesh.set_mode(MeshRenderMode::Lines);
    mesh
}

fn get_marker(zgl: &Zgl, tex_path: &Path) -> Mesh {
    let n = 0.2;
    let vertex_data = vec!(
        VertexCoord{v: Vector3{x: -n, y: 0.0, z: 0.1}},
        VertexCoord{v: Vector3{x: 0.0, y: n * 1.4, z: 0.1}},
        VertexCoord{v: Vector3{x: n, y: 0.0, z: 0.1}},
    );
    let tex_data = vec!(
        TextureCoord{v: Vector2{x: 0.0, y: 0.0}},
        TextureCoord{v: Vector2{x: 1.0, y: 0.0}},
        TextureCoord{v: Vector2{x: 0.5, y: 0.5}},
    );
    let mut mesh = Mesh::new(zgl, vertex_data.as_slice());
    let tex = Texture::new(zgl, tex_path);
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

fn get_scenes(players_count: ZInt) -> HashMap<PlayerId, Scene> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId{id: i}, Scene::new());
    }
    m
}

fn get_game_states(players_count: ZInt) -> HashMap<PlayerId, GameState> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId{id: i}, GameState::new());
    }
    m
}

fn get_pathfinders(
    players_count: ZInt,
    map_size: &Size2<ZInt>,
) -> HashMap<PlayerId, Pathfinder> {
    let mut m = HashMap::new();
    for i in range(0, players_count) {
        m.insert(PlayerId{id: i}, Pathfinder::new(map_size));
    }
    m
}

fn get_marker_mesh_id<'a>(mesh_ids: &'a MeshIdManager, player_id: &PlayerId) -> &'a MeshId {
    match player_id.id {
        0 => &mesh_ids.marker_1_mesh_id,
        1 => &mesh_ids.marker_2_mesh_id,
        n => panic!("Wrong player id: {}", n),
    }
}

fn get_unit_mesh_id<'a> (
    unit_type_visual_info: &'a UnitTypeVisualInfoManager,
    unit_type_id: &UnitTypeId,
) -> &'a MeshId {
    &unit_type_visual_info.get(unit_type_id).mesh_id
}

struct MeshIdManager {
    map_mesh_id: MeshId,
    shell_mesh_id: MeshId,
    marker_1_mesh_id: MeshId,
    marker_2_mesh_id: MeshId,
}

fn add_mesh(meshes: &mut Vec<Mesh>, mesh: Mesh) -> MeshId {
    meshes.push(mesh);
    MeshId{id: (meshes.len() as ZInt) - 1}
}

fn get_unit_type_visual_info(
    zgl: &Zgl,
    meshes: &mut Vec<Mesh>,
) -> UnitTypeVisualInfoManager {
    let tank_mesh_id = add_mesh(meshes, load_unit_mesh(zgl, "tank"));
    let soldier_mesh_id = add_mesh(meshes, load_unit_mesh(zgl, "soldier"));
    let mut unit_type_visual_info = UnitTypeVisualInfoManager::new();
    // TODO: Add by name not by order
    unit_type_visual_info.add_info(UnitTypeVisualInfo {
        mesh_id: tank_mesh_id,
        move_speed: 3.8,
    });
    unit_type_visual_info.add_info(UnitTypeVisualInfo {
        mesh_id: soldier_mesh_id,
        move_speed: 2.0,
    });
    unit_type_visual_info
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
    picker: TilePicker,
    map_pos_under_cursor: Option<MapPos>,
    just_pressed_lmb: bool,
    last_press_pos: ScreenPos,
    font_stash: FontStash,
    map_text_mesh: Mesh,
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    button_test_id: ButtonId,
    dtime: Time,
    last_time: Time,
    // TODO: join in one structure?
    game_states: HashMap<PlayerId, GameState>,
    pathfinders: HashMap<PlayerId, Pathfinder>,
    scenes: HashMap<PlayerId, Scene>,
    core: Core,
    event: Option<CoreEvent>,
    event_visualizer: Option<Box<EventVisualizer + 'static>>,
    mesh_ids: MeshIdManager,
    meshes: Vec<Mesh>,
    unit_type_visual_info: UnitTypeVisualInfoManager,
    unit_under_cursor_id: Option<UnitId>,
    selected_unit_id: Option<UnitId>,
    selection_manager: SelectionManager,
    walkable_mesh: Option<Mesh>, // TODO: move to 'meshes'
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let window_builder = WindowBuilder::new().with_gl_version((2, 0));
        let window = window_builder.build().ok().expect("Can`t create window");
        unsafe {
            window.make_current();
        };
        let players_count = 2;
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
        let core = Core::new();
        let game_states = get_game_states(players_count);
        let picker = TilePicker::new(
            &zgl, &game_states[*core.player_id()], &map_size);

        let mut meshes = Vec::new();

        let map_mesh_id = add_mesh(
            &mut meshes, generate_map_mesh(&zgl, &map_size));
        let selection_marker_mesh_id = add_mesh(
            &mut meshes, get_selection_mesh(&zgl));
        let shell_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("data/shell.png")));
        let marker_1_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("data/flag1.png")));
        let marker_2_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("data/flag2.png")));

        let unit_type_visual_info
            = get_unit_type_visual_info(&zgl, &mut meshes);

        let scenes = get_scenes(players_count);
        let pathfinders = get_pathfinders(players_count, &map_size);
        let font_size = 40.0;
        let mut font_stash = FontStash::new(
            &zgl, &Path::new("data/DroidSerif-Regular.ttf"), font_size);
        let map_text_mesh = font_stash.get_mesh(&zgl, "test text");
        let mut button_manager = ButtonManager::new();
        let button_end_turn_id = button_manager.add_button(Button::new(
            &zgl,
            "end turn",
            &mut font_stash,
            ScreenPos{v: Vector2{x: 10, y: 60}})
        );
        let button_test_id = button_manager.add_button(Button::new(
            &zgl,
            "test",
            &mut font_stash,
            ScreenPos{v: Vector2{x: 10, y: 10}})
        );
        let mesh_ids = MeshIdManager {
            map_mesh_id: map_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_1_mesh_id: marker_1_mesh_id,
            marker_2_mesh_id: marker_2_mesh_id,
        };
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
            picker: picker,
            map_pos_under_cursor: None,
            just_pressed_lmb: false,
            last_press_pos: ScreenPos{v: Vector2::from_value(0)},
            font_stash: font_stash,
            map_text_mesh: map_text_mesh,
            button_manager: button_manager,
            button_end_turn_id: button_end_turn_id,
            button_test_id: button_test_id,
            dtime: Time{n: 0},
            last_time: Time{n: precise_time_ns()},
            game_states: game_states,
            scenes: scenes,
            pathfinders: pathfinders,
            core: core,
            event: None,
            event_visualizer: None,
            mesh_ids: mesh_ids,
            meshes: meshes,
            unit_type_visual_info: unit_type_visual_info,
            unit_under_cursor_id: None,
            selected_unit_id: None,
            selection_manager: SelectionManager::new(selection_marker_mesh_id),
            walkable_mesh: None,
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close
    }

    fn end_turn(&mut self) {
        self.core.do_command(Command::EndTurn);
        self.selected_unit_id = None;
        let scene = self.scenes.get_mut(self.core.player_id()).unwrap();
        self.selection_manager.deselect(scene);
        self.walkable_mesh = None;
    }

    fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        let state = &self.game_states[*self.core.player_id()];
        state.units_at(pos).len() > 0
    }

    /*
    fn create_unit(&mut self) {
        if let Some(ref pos) = self.map_pos_under_cursor {
            if self.is_tile_occupied(pos) {
                return;
            }
            let cmd = Command::CreateUnit(pos.clone());
            self.core.do_command(cmd);
        }
    }
    */

    fn attack_unit(&mut self) {
        match (self.unit_under_cursor_id.clone(), self.selected_unit_id.clone()) {
            (Some(defender_id), Some(attacker_id)) => {
                let state = &self.game_states[*self.core.player_id()];
                let attacker = &state.units[attacker_id];
                if attacker.attacked {
                    return;
                }
                let defender = &state.units[defender_id];
                let max_distance = {
                    let attacker_type = self.core.object_types()
                        .get_unit_type(&attacker.type_id);
                    let weapon_type = self.core.get_weapon_type(
                        &attacker_type.weapon_type_id);
                    weapon_type.max_distance
                };
                if distance(&attacker.pos, &defender.pos) > max_distance {
                    return;
                }
                self.core.do_command(Command::AttackUnit {
                    attacker_id: attacker_id,
                    defender_id: defender_id,
                });
            },
            _ => {},
       }
    }

    fn select_unit(&mut self) {
        if let Some(ref unit_id) = self.unit_under_cursor_id {
            self.selected_unit_id = Some(unit_id.clone());
            let state = &self.game_states[*self.core.player_id()];
            let pf = self.pathfinders.get_mut(self.core.player_id()).unwrap();
            pf.fill_map(state, &state.units[*unit_id]);
            self.walkable_mesh = Some(build_walkable_mesh(&self.zgl, pf));
            let scene = self.scenes.get_mut(self.core.player_id()).unwrap();
            self.selection_manager.create_selection_marker(
                state, scene, unit_id);
            // TODO: highlight potential targets
        }
    }

    fn move_unit(&mut self) {
        let pos = self.map_pos_under_cursor.as_ref().unwrap();
        let unit_id = match self.selected_unit_id {
            Some(ref unit_id) => unit_id.clone(),
            None => return,
        };
        if self.is_tile_occupied(pos) {
            return;
        }
        let state = &self.game_states[*self.core.player_id()];
        let unit = &state.units[unit_id];
        if unit.move_points == 0 {
            return;
        }
        let pf = self.pathfinders.get_mut(self.core.player_id()).unwrap();
        let path = pf.get_path(pos);
        if path.len() < 2 {
            return;
        }
        self.core.do_command(Command::Move{unit_id: unit_id, path: path});
    }


    fn handle_event_mouse_move(&mut self, pos: &ScreenPos) {
        if !self.is_lmb_pressed {
            return;
        }
        let diff = pos.v - self.mouse_pos.v;
        let win_w = self.win_size.w as ZFloat;
        let win_h = self.win_size.h as ZFloat;
        if self.last_press_pos.v.x > self.win_size.w / 2 {
            let per_x_pixel = 180.0 / win_w;
            // TODO: get max angles from camera
            let per_y_pixel = (40.0) / win_h;
            self.camera.add_horizontal_angle(
                deg(diff.x as ZFloat * per_x_pixel));
            self.camera.add_vertical_angle(
                deg(diff.y as ZFloat * per_y_pixel));
        } else {
            let per_x_pixel = CAMERA_MOVE_SPEED / win_w;
            let per_y_pixel = CAMERA_MOVE_SPEED / win_h;
            self.camera.move_camera(
                deg(180.0), diff.x as ZFloat * per_x_pixel);
            self.camera.move_camera(
                deg(270.0), diff.y as ZFloat * per_y_pixel);
        }
    }

    fn handle_event_key_press(&mut self, key: VirtualKeyCode) {
        let s = CAMERA_MOVE_SPEED_KEY;
        match key {
            VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                self.should_close = true;
            },
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.camera.move_camera(deg(270.0), s);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.camera.move_camera(deg(90.0), s);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.camera.move_camera(deg(0.0), s);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.camera.move_camera(deg(180.0), s);
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

    fn handle_event_lmb_released(&mut self) {
        self.is_lmb_pressed = false;
        if self.event_visualizer.is_some() {
            return;
        }
        if !self.is_tap(&self.mouse_pos) {
            return;
        }
        if let Some(button_id) = self.get_clicked_button_id() {
            self.handle_event_button_press(&button_id);
        }
        self.pick_tile();
        if self.map_pos_under_cursor.is_some() {
            self.move_unit();
        }
        if let Some(unit_under_cursor_id) = self.unit_under_cursor_id.clone() {
            let player_id = {
                let state = &self.game_states[*self.core.player_id()];
                let unit = &state.units[unit_under_cursor_id];
                unit.player_id.clone()
            };
            if player_id == *self.core.player_id() {
                self.select_unit();
            } else {
                self.attack_unit();
            }
        }
    }

    fn handle_event_button_press(&mut self, button_id: &ButtonId) {
        if *button_id == self.button_end_turn_id {
            self.end_turn();
        } else if *button_id == self.button_test_id {
            println!("test");
        } else {
            panic!("BUTTON ID ERROR");
        }
    }

    fn get_clicked_button_id(&self) -> Option<ButtonId> {
        self.button_manager.get_clicked_button_id(
            &self.mouse_pos, &self.win_size)
    }

    /// Check if this was a tap or swipe
    fn is_tap(&self, pos: &ScreenPos) -> bool {
        let x = pos.v.x - self.last_press_pos.v.x;
        let y = pos.v.y - self.last_press_pos.v.y;
        let tolerance = 20;
        x.abs() < tolerance && y.abs() < tolerance
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
                if self.just_pressed_lmb {
                    self.just_pressed_lmb = false;
                } else {
                    self.handle_event_mouse_move(&pos);
                }
                self.mouse_pos = pos.clone();
            },
            Event::MouseInput(Pressed, LeftMouseButton) => {
                self.is_lmb_pressed = true;
                self.just_pressed_lmb = true;
                self.last_press_pos = self.mouse_pos.clone();
            },
            Event::MouseInput(Released, LeftMouseButton) => {
                self.handle_event_lmb_released();
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
        // println!("{:?}", events);
        for event in events.iter() {
            self.handle_event(event);
        }
    }

   fn draw_3d_text(&mut self) {
        let m = self.camera.mat(&self.zgl);
        let m = self.zgl.scale(m, 1.0 / self.font_stash.get_size());
        let m = self.zgl.rot_x(m, deg(90.0));
        self.shader.set_uniform_mat4f(
            &self.zgl, self.shader.get_mvp_mat(), &m);
        self.map_text_mesh.draw(&self.zgl, &self.shader);
    }

    fn scene<'a>(&'a self) -> &'a Scene {
        &self.scenes[*self.core.player_id()]
    }

    fn draw_scene_node(
        &self,
        node: &SceneNode,
        m: Matrix4<ZFloat>,
    ) {
        let m = self.zgl.tr(m, node.pos.v);
        let m = self.zgl.rot_z(m, node.rot);
        if let Some(ref mesh_id) = node.mesh_id {
            self.shader.set_uniform_mat4f(
                &self.zgl, self.shader.get_mvp_mat(), &m);
            let id = mesh_id.id as usize;
            self.meshes[id].draw(&self.zgl, &self.shader);
        }
        for node in node.children.iter() {
            self.draw_scene_node(node, m);
        }
    }

    fn draw_scene_nodes(&self) {
        for (_, node) in self.scene().nodes.iter() {
            self.draw_scene_node(node, self.camera.mat(&self.zgl));
        }
    }

    fn draw_map(&mut self) {
        self.shader.set_uniform_mat4f(
            &self.zgl, self.shader.get_mvp_mat(), &self.camera.mat(&self.zgl));
        let id = self.mesh_ids.map_mesh_id.id as usize;
        self.meshes[id].draw(&self.zgl, &self.shader);
    }

    fn draw_scene(&mut self) {
        self.shader.set_uniform_color(
            &self.zgl, &self.color_uniform_location, &zgl::WHITE);
        self.draw_scene_nodes();
        self.draw_map();
        if let Some(ref walkable_mesh) = self.walkable_mesh {
            self.shader.set_uniform_color(
                &self.zgl, &self.color_uniform_location, &zgl::BLUE);
            walkable_mesh.draw(&self.zgl, &self.shader);
        }
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let scene = self.scenes.get_mut(self.core.player_id()).unwrap();
            event_visualizer.draw(scene, &self.dtime);
        }
    }

    fn draw(&mut self) {
        self.zgl.set_clear_color(&BG_COLOR);
        self.zgl.clear_screen();
        self.shader.activate(&self.zgl);
        self.shader.set_uniform_mat4f(
            &self.zgl,
            self.shader.get_mvp_mat(),
            &self.camera.mat(&self.zgl),
        );
        self.draw_scene();
        self.shader.set_uniform_color(
            &self.zgl, &self.color_uniform_location, &zgl::BLACK);
        self.draw_3d_text();
        self.button_manager.draw(
            &self.zgl,
            &self.win_size,
            &self.shader,
            self.shader.get_mvp_mat(),
        );
        self.window.swap_buffers();
    }

    fn pick_tile(&mut self) {
        let pick_result = self.picker.pick_tile(
            &mut self.zgl, &self.camera, &self.win_size, &self.mouse_pos);
        match pick_result {
            PickResult::MapPos(pos) => {
                self.map_pos_under_cursor = Some(pos);
                self.unit_under_cursor_id = None;
            },
            PickResult::UnitId(id) => {
                self.map_pos_under_cursor = None;
                self.unit_under_cursor_id = Some(id);
            },
            PickResult::Nothing => {},
        }
    }

    fn update_time(&mut self) {
        let time = precise_time_ns();
        self.dtime = Time{n: time - self.last_time.n};
        self.last_time = Time{n: time};
    }

    fn make_event_visualizer(
        &mut self,
        event: &CoreEvent,
    ) -> Box<EventVisualizer+'static> {
        let player_id = self.core.player_id();
        let scene = self.scenes.get_mut(player_id).unwrap();
        let state = &self.game_states[*player_id];
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let type_id = state.units[*unit_id].type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                EventMoveVisualizer::new(
                    scene,
                    state,
                    unit_id.clone(),
                    unit_type_visual_info,
                    path.clone(),
                )
            },
            &CoreEvent::EndTurn{old_id: _, new_id: _} => {
                EventEndTurnVisualizer::new()
            },
            &CoreEvent::CreateUnit {
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                EventCreateUnitVisualizer::new(
                    &self.core,
                    scene,
                    state,
                    unit_id.clone(),
                    type_id,
                    pos,
                    get_unit_mesh_id(
                        &self.unit_type_visual_info, type_id),
                    get_marker_mesh_id(&self.mesh_ids, player_id),
                )
            },
            &CoreEvent::AttackUnit{ref attacker_id, ref defender_id, ref killed} => {
                EventAttackUnitVisualizer::new(
                    scene,
                    state,
                    attacker_id.clone(),
                    defender_id.clone(),
                    killed.clone(),
                    self.mesh_ids.shell_mesh_id.clone(),
                )
            },
        }
    }

    fn start_event_visualization(&mut self, event: CoreEvent) {
        let vis = self.make_event_visualizer(&event);
        self.event = Some(event);
        self.event_visualizer = Some(vis);
    }

    fn end_event_visualization(&mut self) {
        let scene = self.scenes.get_mut(
            self.core.player_id()).unwrap();
        let state = self.game_states.get_mut(
            self.core.player_id()).unwrap();
        self.event_visualizer.as_mut().unwrap().end(scene, state);
        state.apply_event(
            self.core.object_types(), self.event.as_ref().unwrap());
        self.event_visualizer = None;
        self.event = None;
        if let Some(ref selected_unit_id) = self.selected_unit_id {
            let pf = self.pathfinders.get_mut(self.core.player_id()).unwrap();
            pf.fill_map(state, &state.units[*selected_unit_id]);
            self.walkable_mesh = Some(build_walkable_mesh(&self.zgl, pf));
            self.selection_manager.move_selection_marker(state, scene);
        }
        self.picker.update_units(&self.zgl, state);
    }

    fn logic(&mut self) {
        if self.event_visualizer.is_none() {
            if let Some(e) = self.core.get_event() {
                self.start_event_visualization(e);
            }
        } else if self.event_visualizer.as_ref().unwrap().is_finished() {
            self.end_event_visualization();
        }
    }

    pub fn tick(&mut self) {
        self.handle_events();
        self.logic();
        self.draw();
        self.update_time();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
