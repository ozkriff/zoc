// See LICENSE file for copyright and license details.

use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use std::path::{Path, PathBuf};
use std::num::{Float};
use time::precise_time_ns;
use std::collections::{HashMap};
use std::num::{SignedInt};
use cgmath::{Vector, Vector2, Vector3, rad, Matrix4};
use glutin;
use glutin::{Window, WindowBuilder, VirtualKeyCode, Event, MouseButton};
use glutin::ElementState::{Pressed, Released};
use common::types::{Size2, ZInt, UnitId, PlayerId, MapPos, ZFloat};
use zgl::types::{
    Color3,
    ColorId,
    ScreenPos,
    VertexCoord,
    TextureCoord,
    Time,
    WorldPos,
};
use zgl::zgl;
use zgl::zgl::{Zgl, MeshRenderMode};
use zgl::mesh::{Mesh, MeshId};
use zgl::camera::Camera;
use zgl::shader::{Shader};
use geom;
use core::map::{Map, distance, Terrain};
use core::dir::{Dir, dirs};
use core::game_state::GameState;
use core::pathfinder::Pathfinder;
use core::command::{Command};
use core::unit::{UnitTypeId};
use core::core::{Core, CoreEvent};
use picker::{TilePicker, PickResult};
use zgl::texture::{Texture};
use zgl::obj;
use zgl::font_stash::{FontStash};
use gui::{ButtonManager, Button, ButtonId};
use scene::{Scene, SceneNode, MIN_MAP_OBJECT_NODE_ID};
use event_visualizer::{
    EventVisualizer,
    EventMoveVisualizer,
    EventEndTurnVisualizer,
    EventCreateUnitVisualizer,
    EventAttackUnitVisualizer,
    EventShowUnitVisualizer,
    EventHideUnitVisualizer,
};
use unit_type_visual_info::{
    UnitTypeVisualInfo,
    UnitTypeVisualInfoManager,
};
use selection::{SelectionManager, get_selection_mesh};
use map_text::{MapTextManager};

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

fn get_initial_camera_pos(map_size: &Size2<ZInt>) -> WorldPos {
    let pos = get_max_camera_pos(map_size);
    WorldPos{v: Vector3{x: pos.v.x / 2.0, y: pos.v.y / 2.0, z: 0.0}}
}

// TODO: Replace all Size2<ZInt> with aliases
fn get_max_camera_pos(map_size: &Size2<ZInt>) -> WorldPos {
    let pos = geom::map_pos_to_world_pos(
        &MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}});
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

fn gen_tiles<F>(zgl: &Zgl, state: &GameState, tex: &Texture, cond: F) -> Mesh
    where F: Fn(bool) -> bool
{
    let mut vertex_data = Vec::new();
    let mut tex_data = Vec::new();
    for tile_pos in state.map().get_iter() {
        if !cond(state.is_tile_visible(&tile_pos)) {
            continue;
        }
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in dirs() {
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
    let mut mesh = Mesh::new(zgl, &vertex_data);
    mesh.add_texture(zgl, tex.clone(), &tex_data);
    mesh

}

fn generate_visible_tiles_mesh(zgl: &Zgl, state: &GameState, tex: &Texture) -> Mesh {
    gen_tiles(zgl, state, tex, |vis| vis)
}

fn generate_fogged_tiles_mesh(zgl: &Zgl, state: &GameState, tex: &Texture) -> Mesh {
    gen_tiles(zgl, state, tex, |vis| !vis)
}

fn build_walkable_mesh(zgl: &Zgl, pf: &Pathfinder, map: &Map<Terrain>, move_points: ZInt) -> Mesh {
    let mut vertex_data = Vec::new();
    for tile_pos in map.get_iter() {
        if pf.get_map().tile(&tile_pos).cost().n > move_points {
            continue;
        }
        if let &Some(ref parent_dir) = pf.get_map().tile(&tile_pos).parent() {
            let tile_pos_to = Dir::get_neighbour_pos(&tile_pos, parent_dir);
            let world_pos_from = geom::map_pos_to_world_pos(&tile_pos);
            let world_pos_to = geom::map_pos_to_world_pos(&tile_pos_to);
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_from.v)});
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_to.v)});
        }
    }
    let mut mesh = Mesh::new(zgl, &vertex_data);
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
    let mut mesh = Mesh::new(zgl, &vertex_data);
    let tex = Texture::new(zgl, tex_path);
    mesh.add_texture(zgl, tex, &tex_data);
    mesh
}

fn load_unit_mesh(zgl: &Zgl, name: &str) -> Mesh {
    let tex_path = PathBuf::from(format!("{}.png", name));
    let obj_path = PathBuf::from(format!("{}.obj", name));
    let tex = Texture::new(zgl, &tex_path);
    let obj = obj::Model::new(&obj_path);
    let mut mesh = Mesh::new(zgl, &obj.build());
    mesh.add_texture(zgl, tex, &obj.build_tex_coord());
    mesh
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
    trees_mesh_id: MeshId,
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

struct PlayerInfo {
    game_state: GameState,
    pathfinder: Pathfinder,
    scene: Scene,
}

struct PlayerInfoManager {
    info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    fn new(map_size: &Size2<ZInt>) -> PlayerInfoManager {
        let mut m = HashMap::new();
        m.insert(PlayerId{id: 0}, PlayerInfo {
            game_state: GameState::new(map_size, &PlayerId{id: 0}),
            pathfinder: Pathfinder::new(map_size),
            scene: Scene::new(),
        });
        m.insert(PlayerId{id: 1}, PlayerInfo {
            game_state: GameState::new(map_size, &PlayerId{id: 1}),
            pathfinder: Pathfinder::new(map_size),
            scene: Scene::new(),
        });
        PlayerInfoManager{info: m}
    }

    fn get<'a>(&'a self, player_id: &PlayerId) -> &'a PlayerInfo {
        &self.info[player_id]
    }

    fn get_mut<'a>(&'a mut self, player_id: &PlayerId) -> &'a mut PlayerInfo {
        match self.info.get_mut(player_id) {
            Some(i) => i,
            None => panic!("Can`t find player_info for id={}", player_id.id),
        }
    }
}

pub struct Visualizer {
    zgl: Zgl,
    window: Window,
    should_close: bool,
    shader: Shader,
    basic_color_id: ColorId,
    camera: Camera,
    mouse_pos: ScreenPos,
    is_lmb_pressed: bool,
    win_size: Size2<ZInt>,
    picker: TilePicker,
    clicked_pos: Option<MapPos>,
    just_pressed_lmb: bool,
    last_press_pos: ScreenPos,
    font_stash: FontStash,
    map_text_manager: MapTextManager,
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    dtime: Time,
    last_time: Time,
    player_info: PlayerInfoManager,
    core: Core,
    event: Option<CoreEvent>,
    event_visualizer: Option<Box<EventVisualizer>>,
    mesh_ids: MeshIdManager,
    meshes: Vec<Mesh>,
    unit_type_visual_info: UnitTypeVisualInfoManager,
    unit_under_cursor_id: Option<UnitId>,
    selected_unit_id: Option<UnitId>,
    selection_manager: SelectionManager,
    // TODO: move to 'meshes'
    walkable_mesh: Option<Mesh>,
    visible_map_mesh: Mesh,
    fow_map_mesh: Mesh,
    floor_tex: Texture,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let gl_version = glutin::GlRequest::GlThenGles {
            opengles_version: (2, 0),
            opengl_version: (2, 0)
        };
        let window_builder = WindowBuilder::new()
            .with_title(format!("Zone of Control"))
            .with_gl(gl_version);
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
        let basic_color_id = shader.get_uniform_color(
            &zgl, "basic_color");
        zgl.set_clear_color(&BG_COLOR);
        let mut camera = Camera::new(&win_size);
        let core = Core::new();
        let map_size = core.map_size().clone();
        camera.set_max_pos(get_max_camera_pos(&map_size));
        camera.set_pos(get_initial_camera_pos(&map_size));
        let player_info = PlayerInfoManager::new(&map_size);
        let picker = TilePicker::new(
            &zgl, &player_info.get(core.player_id()).game_state);

        let floor_tex = Texture::new(&zgl, &Path::new("floor.png")); // TODO: !!!

        let mut meshes = Vec::new();

        let visible_map_mesh = generate_visible_tiles_mesh(
            &zgl, &player_info.get(core.player_id()).game_state, &floor_tex);
        let fow_map_mesh = generate_fogged_tiles_mesh(
            &zgl, &player_info.get(core.player_id()).game_state, &floor_tex);

        let trees_mesh_id = add_mesh(
            &mut meshes, load_unit_mesh(&zgl, "trees"));
        let selection_marker_mesh_id = add_mesh(
            &mut meshes, get_selection_mesh(&zgl));
        let shell_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("shell.png")));
        let marker_1_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("flag1.png")));
        let marker_2_mesh_id = add_mesh(
            &mut meshes, get_marker(&zgl, &Path::new("flag2.png")));

        let unit_type_visual_info
            = get_unit_type_visual_info(&zgl, &mut meshes);

        let font_size = 40.0;
        let mut font_stash = FontStash::new(
            &zgl, &Path::new("DroidSerif-Regular.ttf"), font_size);
        let mut button_manager = ButtonManager::new();
        let button_end_turn_id = button_manager.add_button(Button::new(
            &zgl,
            &win_size,
            "end turn",
            &mut font_stash,
            ScreenPos{v: Vector2{x: 10, y: 10}})
        );
        let mesh_ids = MeshIdManager {
            trees_mesh_id: trees_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_1_mesh_id: marker_1_mesh_id,
            marker_2_mesh_id: marker_2_mesh_id,
        };
        let mut visualizer = Visualizer {
            zgl: zgl,
            window: window,
            should_close: false,
            shader: shader,
            basic_color_id: basic_color_id,
            camera: camera,
            mouse_pos: ScreenPos{v: Vector::from_value(0)},
            is_lmb_pressed: false,
            win_size: win_size,
            picker: picker,
            clicked_pos: None,
            just_pressed_lmb: false,
            last_press_pos: ScreenPos{v: Vector::from_value(0)},
            font_stash: font_stash,
            button_manager: button_manager,
            button_end_turn_id: button_end_turn_id,
            dtime: Time{n: 0},
            last_time: Time{n: precise_time_ns()},
            player_info: player_info,
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
            map_text_manager: MapTextManager::new(),
            visible_map_mesh: visible_map_mesh,
            fow_map_mesh: fow_map_mesh,
            floor_tex: floor_tex,
        };
        visualizer.add_map_objects();
        visualizer
    }

    fn add_map_objects(&mut self) {
        let mut node_id = MIN_MAP_OBJECT_NODE_ID.clone();

        for (_, player_info) in self.player_info.info.iter_mut() {
            let map = &player_info.game_state.map();
            for tile_pos in map.get_iter() {
                if let &Terrain::Trees = map.tile(&tile_pos) {
                    let pos = geom::map_pos_to_world_pos(&tile_pos);
                    let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
                        player_info.scene.nodes.insert(node_id.clone(), SceneNode {
                            pos: pos.clone(),
                            rot: rot,
                            mesh_id: Some(self.mesh_ids.trees_mesh_id.clone()),
                            children: Vec::new(),
                        });
                    node_id.id += 1;
                }
            }
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close
    }

    fn end_turn(&mut self) {
        self.core.do_command(Command::EndTurn);
        self.selected_unit_id = None;
        let i = self.player_info.get_mut(self.core.player_id());
        self.selection_manager.deselect(&mut i.scene);
        self.walkable_mesh = None;
    }

    fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        let i = self.player_info.get(self.core.player_id());
        i.game_state.is_tile_occupied(pos)
    }

    fn create_unit(&mut self) {
        if let Some(ref pos) = self.clicked_pos {
            if self.is_tile_occupied(pos) {
                return;
            }
            let cmd = Command::CreateUnit{pos: pos.clone()};
            self.core.do_command(cmd);
        }
    }

    fn attack_unit(&mut self) {
        match (self.unit_under_cursor_id.clone(), self.selected_unit_id.clone()) {
            (Some(defender_id), Some(attacker_id)) => {
                let state = &self.player_info.get(self.core.player_id()).game_state;
                let attacker = &state.units()[&attacker_id];
                if attacker.attack_points <= 0 {
                    println!("No attack points");
                    return;
                }
                let defender = &state.units()[&defender_id];
                let max_distance = self.core.object_types
                    .get_unit_max_attack_dist(attacker);
                if distance(&attacker.pos, &defender.pos) > max_distance {
                    println!("Out of range");
                    return;
                }
                if !self.core.los(&attacker.pos, &defender.pos) {
                    println!("No LOS");
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
            let mut i = self.player_info.get_mut(self.core.player_id());
            let state = &i.game_state;
            let pf = &mut i.pathfinder;
            pf.fill_map(&self.core.object_types, state, &state.units()[unit_id]);
            self.walkable_mesh = Some(build_walkable_mesh(
                &self.zgl, pf, state.map(), state.units()[unit_id].move_points));
            let scene = &mut i.scene;
            self.selection_manager.create_selection_marker(
                state, scene, unit_id);
            // TODO: highlight potential targets
        }
    }

    fn move_unit(&mut self) {
        let pos = self.clicked_pos.as_ref().unwrap();
        let unit_id = match self.selected_unit_id {
            Some(ref unit_id) => unit_id.clone(),
            None => return,
        };
        if self.is_tile_occupied(pos) {
            return;
        }
        let i = self.player_info.get_mut(self.core.player_id());
        let unit = &i.game_state.units()[&unit_id];
        if let Some(path) = i.pathfinder.get_path(pos) {
            if path.total_cost(). n > unit.move_points {
                println!("path cost > unit.move_points");
                return;
            }
            self.core.do_command(
                Command::Move{unit_id: unit_id, path: path});
        } else {
            println!("Can not reach that tile");
        }
    }

    fn handle_event_mouse_move(&mut self, pos: &ScreenPos) {
        if !self.is_lmb_pressed {
            return;
        }
        let diff = pos.v - self.mouse_pos.v;
        let win_w = self.win_size.w as ZFloat;
        let win_h = self.win_size.h as ZFloat;
        if self.last_press_pos.v.x > self.win_size.w / 2 {
            let per_x_pixel = PI / win_w;
            // TODO: get max angles from camera
            let per_y_pixel = (PI / 4.0) / win_h;
            self.camera.add_horizontal_angle(
                rad(diff.x as ZFloat * per_x_pixel));
            self.camera.add_vertical_angle(
                rad(diff.y as ZFloat * per_y_pixel));
        } else {
            let per_x_pixel = CAMERA_MOVE_SPEED / win_w;
            let per_y_pixel = CAMERA_MOVE_SPEED / win_h;
            self.camera.move_camera(
                rad(PI), diff.x as ZFloat * per_x_pixel);
            self.camera.move_camera(
                rad(PI * 1.5), diff.y as ZFloat * per_y_pixel);
        }
    }

    fn handle_event_key_press(&mut self, key: VirtualKeyCode) {
        let s = CAMERA_MOVE_SPEED_KEY;
        match key {
            VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                self.should_close = true;
            },
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.camera.move_camera(rad(PI * 1.5), s);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.camera.move_camera(rad(PI / 2.0), s);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.camera.move_camera(rad(0.0), s);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.camera.move_camera(rad(PI), s);
            },
            VirtualKeyCode::K => {
                if let Some(ref clicked_pos) = self.clicked_pos {
                    self.map_text_manager.add_text(
                        &self.zgl,
                        &mut self.font_stash,
                        "TEST",
                        clicked_pos,
                    );
                }
            },
            VirtualKeyCode::U => {
                self.create_unit();
            },
            VirtualKeyCode::Subtract | VirtualKeyCode::Key1 => {
                self.camera.change_zoom(1.3);
            },
            VirtualKeyCode::Equals | VirtualKeyCode::Key2 => {
                self.camera.change_zoom(0.7);
            },
            key => println!("KEY: {:?}", key),
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
        if self.clicked_pos.is_some() {
            self.move_unit();
        }
        if let Some(unit_under_cursor_id) = self.unit_under_cursor_id.clone() {
            let player_id = {
                let state = &self.player_info.get(self.core.player_id()).game_state;
                let unit = &state.units()[&unit_under_cursor_id];
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
                self.camera.regenerate_projection_mat(&self.win_size);
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
            Event::MouseInput(Pressed, MouseButton::Left) => {
                self.is_lmb_pressed = true;
                self.just_pressed_lmb = true;
                self.last_press_pos = self.mouse_pos.clone();
            },
            Event::MouseInput(Released, MouseButton::Left) => {
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

    fn scene(&self) -> &Scene {
        &self.player_info.get(self.core.player_id()).scene
    }

    fn draw_scene_node(
        &self,
        node: &SceneNode,
        m: Matrix4<ZFloat>,
    ) {
        let m = self.zgl.tr(m, &node.pos.v);
        let m = self.zgl.rot_z(m, &node.rot);
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
        self.shader.set_uniform_color(
            &self.zgl, &self.basic_color_id, &zgl::GREY);
        self.fow_map_mesh.draw(&self.zgl, &self.shader);
        self.shader.set_uniform_color(
            &self.zgl, &self.basic_color_id, &zgl::WHITE);
        self.visible_map_mesh.draw(&self.zgl, &self.shader);
    }

    fn draw_scene(&mut self) {
        self.shader.set_uniform_color(
            &self.zgl, &self.basic_color_id, &zgl::WHITE);
        self.draw_scene_nodes();
        self.draw_map();
        if let Some(ref walkable_mesh) = self.walkable_mesh {
            self.shader.set_uniform_color(
                &self.zgl, &self.basic_color_id, &zgl::BLUE);
            walkable_mesh.draw(&self.zgl, &self.shader);
        }
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let i = self.player_info.get_mut(self.core.player_id());
            event_visualizer.draw(&mut i.scene, &self.dtime);
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
            &self.zgl, &self.basic_color_id, &zgl::BLACK);
        self.map_text_manager.draw(
            &self.zgl, &self.camera, &self.shader, &self.dtime);
        self.button_manager.draw(
            &self.zgl,
            &self.win_size,
            &self.shader,
            self.shader.get_mvp_mat(),
        );
        // You must call glFlush before swap_buffers, or else
        // on Windows 8 nothing will be visible on the window.
        self.zgl.flush();
        self.window.swap_buffers();
    }

    fn pick_tile(&mut self) {
        let pick_result = self.picker.pick_tile(
            &mut self.zgl, &self.camera, &self.win_size, &self.mouse_pos);
        match pick_result {
            PickResult::MapPos(pos) => {
                self.clicked_pos = Some(pos);
                self.unit_under_cursor_id = None;
            },
            PickResult::UnitId(id) => {
                self.clicked_pos = None;
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
    ) -> Box<EventVisualizer> {
        let player_id = self.core.player_id();
        let mut i = self.player_info.get_mut(player_id);
        let scene = &mut i.scene;
        let state = &i.game_state;
        match event {
            &CoreEvent::Move{ref unit_id, ref path} => {
                let type_id = state.units()[unit_id].type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                EventMoveVisualizer::new(
                    scene,
                    unit_id.clone(),
                    unit_type_visual_info,
                    path.clone(),
                )
            },
            &CoreEvent::EndTurn{..} => {
                EventEndTurnVisualizer::new()
            },
            &CoreEvent::CreateUnit {
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                let mesh_id = get_unit_mesh_id(
                    &self.unit_type_visual_info, type_id);
                EventCreateUnitVisualizer::new(
                    &self.core,
                    scene,
                    unit_id.clone(),
                    type_id,
                    pos,
                    mesh_id,
                    get_marker_mesh_id(&self.mesh_ids, player_id),
                )
            },
            &CoreEvent::AttackUnit {
                ref attacker_id,
                ref defender_id,
                ref killed,
                ref mode,
            } => {
                EventAttackUnitVisualizer::new(
                    &self.zgl,
                    scene,
                    attacker_id.clone(),
                    defender_id.clone(),
                    killed.clone(),
                    mode.clone(),
                    self.mesh_ids.shell_mesh_id.clone(),
                    &mut self.map_text_manager,
                    &mut self.font_stash,
                )
            },
            &CoreEvent::ShowUnit {
                ref unit_id,
                ref pos,
                ref type_id,
                ref player_id,
            } => {
                let mesh_id = get_unit_mesh_id(
                    &self.unit_type_visual_info, type_id);
                EventShowUnitVisualizer::new(
                    &self.core,
                    &self.zgl,
                    scene,
                    unit_id.clone(),
                    type_id,
                    pos,
                    mesh_id,
                    get_marker_mesh_id(&self.mesh_ids, player_id),
                    &mut self.map_text_manager,
                    &mut self.font_stash,
                )
            },
            &CoreEvent::HideUnit{ref unit_id} => {
                EventHideUnitVisualizer::new(
                    scene,
                    unit_id,
                    &self.zgl,
                    &mut self.map_text_manager,
                    &mut self.font_stash,
                )
            },
        }
    }

    fn is_event_visualization_finished(&self) -> bool {
        self.event_visualizer.as_ref()
            .expect("No event visualizer")
            .is_finished()
    }

    fn start_event_visualization(&mut self, event: CoreEvent) {
        let vis = self.make_event_visualizer(&event);
        self.event = Some(event);
        self.event_visualizer = Some(vis);
        if self.is_event_visualization_finished() {
            self.end_event_visualization();
        } else {
            let i = &mut self.player_info.get_mut(self.core.player_id());
            self.selection_manager.deselect(&mut i.scene);
            self.walkable_mesh = None;
        }
    }

    /// handle case when attacker == selected_unit and it dies from reaction fire
    fn attacker_died_from_reaction_fire(&mut self) {
        // TODO: simplify
        if let Some(CoreEvent::AttackUnit{ref defender_id, ref killed, ..})
            = self.event
        {
            if self.selected_unit_id.is_some()
                && *self.selected_unit_id.as_ref().unwrap() == *defender_id
                && *killed
            {
                self.selected_unit_id = None;
            }
        }
    }

    fn end_event_visualization(&mut self) {
        self.attacker_died_from_reaction_fire();
        let mut i = self.player_info.get_mut(self.core.player_id());
        let scene = &mut i.scene;
        let state = &mut i.game_state;
        self.event_visualizer.as_mut().unwrap().end(scene, state);
        state.apply_event(
            self.core.object_types(), self.event.as_ref().unwrap());
        self.event_visualizer = None;
        self.event = None;
        if let Some(ref selected_unit_id) = self.selected_unit_id {
            if let Some(unit) = state.units().get(selected_unit_id) {
                // TODO: do this only if this is last unshowed CoreEvent
                let pf = &mut i.pathfinder;
                pf.fill_map(&self.core.object_types, state, unit);
                self.walkable_mesh = Some(build_walkable_mesh(
                    &self.zgl, pf, state.map(), unit.move_points));
                self.selection_manager.create_selection_marker(
                    state, scene, selected_unit_id);
            }
        }
        // TODO: recolor terrain objects
        self.visible_map_mesh = generate_visible_tiles_mesh(
            &self.zgl, state, &self.floor_tex);
        self.fow_map_mesh = generate_fogged_tiles_mesh(
            &self.zgl, state, &self.floor_tex);
        self.picker.update_units(&self.zgl, state);
    }

    fn logic(&mut self) {
        while self.event_visualizer.is_none() {
            // TODO: convert to iterator
            if let Some(e) = self.core.get_event() {
                self.start_event_visualization(e);
            } else {
                break;
            }
        }
        if self.event_visualizer.is_some()
            && self.is_event_visualization_finished()
        {
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
