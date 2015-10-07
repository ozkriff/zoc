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

use std::f32::consts::{PI};
use num::{Float};
use rand::{thread_rng, Rng};
use std::path::{Path, PathBuf};
use time::precise_time_ns;
use std::collections::{HashMap};
use cgmath::{
    Vector,
    Vector2,
    Vector3,
    Vector4,
    EuclideanVector,
    rad,
    Matrix,
    Matrix4,
    Plane,
    Point,
    Ray,
    Intersect,
};
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
use zgl::{Zgl, MeshRenderMode};
use zgl::mesh::{Mesh, MeshId};
use zgl::camera::Camera;
use zgl::shader::{Shader};
use core::map::{Map, distance, Terrain, spiral_iter};
use core::dir::{Dir, dirs};
use core::game_state::GameState;
use core::pathfinder::Pathfinder;
use core::{
    Core,
    CoreEvent,
    Command,
    MoveMode,
    ReactionFireMode,
    los,
    get_unit_id_at,
};
use core::unit::{Unit, UnitClass};
use core::db::{Db};
use zgl::texture::{Texture};
use zgl::obj;
use zgl::font_stash::{FontStash};
use gui::{ButtonManager, Button, ButtonId};
use scene::{NodeId, Scene, SceneNode, MIN_MAP_OBJECT_NODE_ID};
use event_visualizer::{
    EventVisualizer,
    EventMoveVisualizer,
    EventEndTurnVisualizer,
    EventCreateUnitVisualizer,
    EventUnloadUnitVisualizer,
    EventLoadUnitVisualizer,
    EventAttackUnitVisualizer,
    EventShowUnitVisualizer,
    EventHideUnitVisualizer,
    EventSetReactionFireModeVisualizer,
};
use unit_type_visual_info::{
    UnitTypeVisualInfo,
    UnitTypeVisualInfoManager,
};
use selection::{SelectionManager, get_selection_mesh};
use map_text::{MapTextManager};

const BG_COLOR: Color3 = Color3{r: 0.8, g: 0.8, b: 0.8};

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

fn get_win_size(window: &Window) -> Size2 {
    let (w, h) = window.get_inner_size().expect("Can`t get window size");
    Size2{w: w as ZInt, h: h as ZInt}
}

fn get_initial_camera_pos(map_size: &Size2) -> WorldPos {
    let pos = get_max_camera_pos(map_size);
    WorldPos{v: Vector3{x: pos.v.x / 2.0, y: pos.v.y / 2.0, z: 0.0}}
}

fn get_max_camera_pos(map_size: &Size2) -> WorldPos {
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
    db: &Db,
    zgl: &Zgl,
    meshes: &mut Vec<Mesh>,
) -> UnitTypeVisualInfoManager {
    let unit_types_count = db.unit_types_count();
    let mut manager = UnitTypeVisualInfoManager::new(unit_types_count);
    let tank_id = db.unit_type_id("tank");
    let tank_mesh_id = add_mesh(meshes, load_unit_mesh(zgl, "tank"));
    manager.add_info(&tank_id, UnitTypeVisualInfo {
        mesh_id: tank_mesh_id,
        move_speed: 3.8,
    });
    let truck_id = db.unit_type_id("truck");
    let truck_mesh_id = add_mesh(meshes, load_unit_mesh(zgl, "truck"));
    manager.add_info(&truck_id, UnitTypeVisualInfo {
        mesh_id: truck_mesh_id,
        move_speed: 4.8,
    });
    let soldier_id = db.unit_type_id("soldier");
    let soldier_mesh_id = add_mesh(meshes, load_unit_mesh(zgl, "soldier"));
    manager.add_info(&soldier_id, UnitTypeVisualInfo {
        mesh_id: soldier_mesh_id.clone(),
        move_speed: 2.0,
    });
    let scout_id = db.unit_type_id("scout");
    manager.add_info(&scout_id, UnitTypeVisualInfo {
        mesh_id: soldier_mesh_id.clone(),
        move_speed: 3.0,
    });
    manager
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
    fn new(map_size: &Size2) -> PlayerInfoManager {
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

#[derive(Clone)]
enum PickResult {
    Pos(MapPos),
    UnitId(UnitId),
    None,
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
    is_rmb_pressed: bool,
    win_size: Size2,
    last_press_pos: ScreenPos,
    font_stash: FontStash,
    map_text_manager: MapTextManager,
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    last_time: Time,
    player_info: PlayerInfoManager,
    core: Core,
    event: Option<CoreEvent>,
    event_visualizer: Option<Box<EventVisualizer>>,
    mesh_ids: MeshIdManager,
    meshes: Vec<Mesh>,
    unit_type_visual_info: UnitTypeVisualInfoManager,
    selected_unit_id: Option<UnitId>,
    selection_manager: SelectionManager,
    // TODO: move to 'meshes'
    walkable_mesh: Option<Mesh>,
    visible_map_mesh: Mesh,
    fow_map_mesh: Mesh,
    floor_tex: Texture,
    pick_result: PickResult,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        let gl_version = glutin::GlRequest::GlThenGles {
            opengles_version: (2, 0),
            opengl_version: (2, 0)
        };
        let window_builder = WindowBuilder::new()
            .with_title(format!("Zone of Control"))
            .with_pixel_format(24, 8)
            .with_gl(gl_version);
        let window = window_builder.build().ok().expect("Can`t create window");
        unsafe {
            window.make_current()
                .ok().expect("Can`t make window current");
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
            = get_unit_type_visual_info(core.db(), &zgl, &mut meshes);

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
        let map_text_manager = MapTextManager::new(&mut font_stash);
        let mut visualizer = Visualizer {
            zgl: zgl,
            window: window,
            should_close: false,
            shader: shader,
            basic_color_id: basic_color_id,
            camera: camera,
            mouse_pos: ScreenPos{v: Vector::from_value(0)},
            is_lmb_pressed: false,
            is_rmb_pressed: false,
            win_size: win_size,
            last_press_pos: ScreenPos{v: Vector::from_value(0)},
            font_stash: font_stash,
            button_manager: button_manager,
            button_end_turn_id: button_end_turn_id,
            last_time: Time{n: precise_time_ns()},
            player_info: player_info,
            core: core,
            event: None,
            event_visualizer: None,
            mesh_ids: mesh_ids,
            meshes: meshes,
            unit_type_visual_info: unit_type_visual_info,
            selected_unit_id: None,
            selection_manager: SelectionManager::new(selection_marker_mesh_id),
            walkable_mesh: None,
            map_text_manager: map_text_manager,
            visible_map_mesh: visible_map_mesh,
            fow_map_mesh: fow_map_mesh,
            floor_tex: floor_tex,
            pick_result: PickResult::None,
        };
        visualizer.add_map_objects();
        visualizer
    }

    fn pick_world_pos(&self) -> WorldPos {
        let im = self.camera.mat(&self.zgl).invert()
            .expect("Can`t invert camera matrix");
        let w = self.win_size.w as ZFloat;
        let h = self.win_size.h as ZFloat;
        let x = self.mouse_pos.v.x as ZFloat;
        let y = self.mouse_pos.v.y as ZFloat;
        let x = (2.0 * x) / w - 1.0;
        let y = 1.0 - (2.0 * y) / h;
        let p0_raw = im.mul_v(&Vector4{x: x, y: y, z: 0.0, w: 1.0});
        let p0 = (p0_raw.div_s(p0_raw.w)).truncate();
        let p1_raw = im.mul_v(&Vector4{x: x, y: y, z: 1.0, w: 1.0});
        let p1 = (p1_raw.div_s(p1_raw.w)).truncate();
        let plane = Plane::from_abcd(0.0, 0.0, 1.0, 0.0);
        let ray = Ray::new(Point::from_vec(&p0), p1 - p0);
        let p = (plane, ray).intersection()
            .expect("Can`t find mouse ray/plane intersection");
        WorldPos{v: p.to_vec()}
    }

    fn add_marker(&mut self, pos: &WorldPos) {
        for (_, player_info) in self.player_info.info.iter_mut() {
            let node_id = NodeId{id: 3000}; // TODO: remove magic
            player_info.scene.nodes.insert(node_id, SceneNode {
                pos: pos.clone(),
                rot: rad(0.0),
                mesh_id: Some(self.mesh_ids.shell_mesh_id.clone()),
                children: Vec::new(),
            });
        }
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
        self.pick_result = PickResult::None;
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

    fn load_unit(&mut self, passanger_id: &UnitId) {
        let state = &self.player_info.get(self.core.player_id()).game_state;
        let passanger = state.unit(&passanger_id);
        let pos = passanger.pos.clone();
        let transporter_id = if let Some(id) = self.selected_unit_id.clone() {
            id
        } else {
            self.map_text_manager.add_text(&pos, "No selected unit");
            return;
        };
        let transporter = state.unit(&transporter_id);
        if !self.core.db().unit_type(&transporter.type_id).is_transporter {
            self.map_text_manager.add_text(&pos, "Not transporter");
            return;
        }
        match self.core.db().unit_type(&passanger.type_id).class {
            UnitClass::Infantry => {},
            _ => {
                self.map_text_manager.add_text(&pos, "Bad passanger class");
                return;
            }
        }
        if transporter.passanger_id.is_some() {
            self.map_text_manager.add_text(&pos, "Transporter is not empty");
            return;
        }
        if distance(&transporter.pos, &pos) > 1 {
            self.map_text_manager.add_text(&pos, "Distance > 1");
            return;
        }
        // TODO: 0 -> real move cost of transport tile for passanger
        if passanger.move_points == 0 {
            self.map_text_manager.add_text(&pos, "Passanger move point == 0");
            return;
        }
        self.core.do_command(Command::LoadUnit {
            transporter_id: transporter_id,
            passanger_id: passanger_id.clone(),
        });
    }

    fn unload_unit(&mut self, pos: &MapPos) {
        let state = &self.player_info.get(self.core.player_id()).game_state;
        let transporter_id = if let Some(id) = self.selected_unit_id.clone() {
            id
        } else {
            self.map_text_manager.add_text(&pos, "No selected unit");
            return;
        };
        let transporter = state.units().get(&transporter_id)
            .expect("Bad transporter_id");
        // TODO: Duplicate all this checks ib Core
        // TODO: check that tile is empty and walkable for passanger
        if !self.core.db().unit_type(&transporter.type_id).is_transporter {
            self.map_text_manager.add_text(&pos, "Not transporter");
            return;
        }
        if distance(&transporter.pos, &pos) > 1 {
            self.map_text_manager.add_text(&pos, "Distance > 1");
            return;
        }
        let passanger_id = match transporter.passanger_id.clone() {
            Some(id) => id,
            None => {
                self.map_text_manager.add_text(&pos, "Transporter is empty");
                return;
            },
        };
        if state.units_at(pos).len() > 0 {
            self.map_text_manager.add_text(&pos, "Destination tile is not empty");
            return;
        }
        self.core.do_command(Command::UnloadUnit {
            transporter_id: transporter_id,
            passanger_id: passanger_id,
            pos: pos.clone(),
        });
    }

    fn change_reaction_fire_mode(&mut self) {
        self.pick_tile();
        let state = &self.player_info.get(self.core.player_id()).game_state;
        let unit_id = match self.pick_result {
            PickResult::UnitId(ref id) => id,
            PickResult::Pos(ref pos) => {
                self.map_text_manager.add_text(pos, "No selected unit");
                return;
            },
            PickResult::None => {
                return;
            },
        };
        let unit = state.unit(&unit_id);
        let mode = match unit.reaction_fire_mode {
            ReactionFireMode::Normal => ReactionFireMode::HoldFire,
            ReactionFireMode::HoldFire => ReactionFireMode::Normal,
        };
        self.core.do_command(Command::SetReactionFireMode {
            unit_id: unit_id.clone(),
            mode: mode,
        });
    }

    fn transport(&mut self) {
        self.pick_tile();
        match self.pick_result.clone() {
            PickResult::Pos(ref pos) => {
                self.unload_unit(pos);
            },
            PickResult::UnitId(ref passanger_id) => {
                self.load_unit(passanger_id);
            },
            PickResult::None => {},
        }
    }

    fn create_unit(&mut self) {
        if let PickResult::Pos(ref pos) = self.pick_result {
            if self.is_tile_occupied(pos) {
                return;
            }
            let cmd = Command::CreateUnit{pos: pos.clone()};
            self.core.do_command(cmd);
        }
    }

    pub fn los(&self, unit: &Unit, from: &MapPos, to: &MapPos) -> bool {
        let unit_type = self.core.db().unit_type(&unit.type_id);
        let i = self.player_info.get(self.core.player_id());
        let map = i.game_state.map();
        los(map, unit_type, from, to)
    }

    fn attack_unit(&mut self, attacker_id: &UnitId, defender_id: &UnitId) {
        let state = &self.player_info.get(self.core.player_id()).game_state;
        let attacker = &state.units()[attacker_id];
        let defender = &state.units()[defender_id];
        if attacker.attack_points <= 0 {
            self.map_text_manager.add_text(
                &defender.pos, "No attack points");
            return;
        }
        if attacker.morale < 50 {
            self.map_text_manager.add_text(
                &defender.pos, "Can`t attack when suppressed");
            return;
        }
        // TODO: merge error handling of visualizer and core
        {
            let attacker_type = self.core.db().unit_type(&attacker.type_id);
            let weapon_type = self.core.db().weapon_type(&attacker_type.weapon_type_id);
            let max_distance = weapon_type.max_distance;
            let min_distance = weapon_type.min_distance;
            if distance(&attacker.pos, &defender.pos) > max_distance {
                self.map_text_manager.add_text(
                    &defender.pos, "Out of range");
                return;
            }
            if distance(&attacker.pos, &defender.pos) < min_distance {
                self.map_text_manager.add_text(
                    &defender.pos, "Too close");
                return;
            }
        }
        if !self.los(attacker, &attacker.pos, &defender.pos) {
            self.map_text_manager.add_text(
                &defender.pos, "No LOS");
            return;
        }
        self.core.do_command(Command::AttackUnit {
            attacker_id: attacker_id.clone(),
            defender_id: defender_id.clone(),
        });
    }

    fn try_to_attack_unit(&mut self) {
        let defender_id = if let PickResult::UnitId(id) = self.pick_result.clone() {
            id
        } else {
            return;
        };
        let attacker_id = if let Some(id) = self.selected_unit_id.clone() {
            id
        } else {
            return;
        };
        self.attack_unit(&attacker_id, &defender_id)
    }

    fn select_unit(&mut self) {
        if let PickResult::UnitId(ref unit_id) = self.pick_result {
            self.selected_unit_id = Some(unit_id.clone());
            let mut i = self.player_info.get_mut(self.core.player_id());
            let state = &i.game_state;
            let pf = &mut i.pathfinder;
            pf.fill_map(self.core.db(), state, &state.units()[unit_id]);
            self.walkable_mesh = Some(build_walkable_mesh(
                &self.zgl, pf, state.map(), state.units()[unit_id].move_points));
            let scene = &mut i.scene;
            self.selection_manager.create_selection_marker(
                state, scene, unit_id);
            // TODO: highlight potential targets
        }
    }

    fn move_unit(&mut self, pos: &MapPos, move_mode: &MoveMode) {
        let unit_id = match self.selected_unit_id {
            Some(ref unit_id) => unit_id.clone(),
            None => return,
        };
        if self.is_tile_occupied(&pos) {
            return;
        }
        let i = self.player_info.get_mut(self.core.player_id());
        let unit = &i.game_state.units()[&unit_id];
        if let Some(path) = i.pathfinder.get_path(&pos) {
            let cost = if let &MoveMode::Hunt = move_mode {
                path.total_cost().n * 2
            } else {
                path.total_cost().n
            };
            if cost > unit.move_points {
                self.map_text_manager.add_text(
                    &pos, "Not enough move points");
                return;
            }
            self.core.do_command(Command::Move {
                unit_id: unit_id,
                path: path,
                mode: move_mode.clone(),
            });
        } else {
            self.map_text_manager.add_text(
                &pos, "Can not reach this tile");
        }
    }

    fn handle_camera_move(&mut self, pos: &ScreenPos) {
        let diff = pos.v - self.mouse_pos.v;
        let camera_move_speed = geom::HEX_EX_RADIUS * 12.0;
        let per_x_pixel = camera_move_speed / (self.win_size.w as ZFloat);
        let per_y_pixel = camera_move_speed / (self.win_size.h as ZFloat);
        self.camera.move_camera(
            rad(PI), diff.x as ZFloat * per_x_pixel);
        self.camera.move_camera(
            rad(PI * 1.5), diff.y as ZFloat * per_y_pixel);
    }

    fn handle_camera_rotate(&mut self, pos: &ScreenPos) {
        let diff = pos.v - self.mouse_pos.v;
        let per_x_pixel = PI / (self.win_size.w as ZFloat);
        // TODO: get max angles from camera
        let per_y_pixel = (PI / 4.0) / (self.win_size.h as ZFloat);
        self.camera.add_horizontal_angle(
            rad(diff.x as ZFloat * per_x_pixel));
        self.camera.add_vertical_angle(
            rad(diff.y as ZFloat * per_y_pixel));
    }

    fn handle_event_mouse_move(&mut self, pos: &ScreenPos) {
        self.handle_event_mouse_move_platform(pos);
        self.mouse_pos = pos.clone();
    }

    #[cfg(not(target_os = "android"))]
    fn handle_event_mouse_move_platform(&mut self, pos: &ScreenPos) {
        if self.is_lmb_pressed {
            self.handle_camera_move(pos);
        } else if self.is_rmb_pressed {
            self.handle_camera_rotate(pos);
        }
    }

    #[cfg(target_os = "android")]
    fn handle_event_mouse_move_platform(&mut self, pos: &ScreenPos) {
        if !self.is_lmb_pressed {
            return;
        }
        if self.must_rotate_camera() {
            self.handle_camera_rotate(pos);
        } else {
            self.handle_camera_move(pos);
        }
    }

    #[cfg(target_os = "android")]
    fn must_rotate_camera(&self) -> bool {
        if self.win_size.w > self.win_size.h {
            self.last_press_pos.v.x > self.win_size.w / 2
        } else {
            self.last_press_pos.v.y < self.win_size.h / 2
        }
    }

    fn handle_event_lmb_press(&mut self) {
        self.is_lmb_pressed = true;
        self.last_press_pos = self.mouse_pos.clone();
    }

    fn print_unit_info(&self, unit_id: &UnitId) {
        let state = &self.player_info.get(self.core.player_id()).game_state;
        let unit = state.units().get(unit_id)
            .expect("Can`t find picked unit in current state");
        // TODO: use only one println
        println!("player_id: {}", unit.player_id.id);
        println!("move_points: {}", unit.move_points);
        println!("attack_points: {}", unit.attack_points);
        if let Some(reactive_attack_points) = unit.reactive_attack_points {
            println!("reactive_attack_points: {}", reactive_attack_points);
        } else {
            println!("reactive_attack_points: ???");
        }
        println!("count: {}", unit.count);
        println!("morale: {}", unit.morale);
        let unit_type = self.core.db().unit_type(&unit.type_id);
        println!("type: name: {}", unit_type.name);
        match unit_type.class {
            UnitClass::Infantry => println!("type: class: Infantry"),
            UnitClass::Vehicle => println!("type: class: Vehicle"),
        }
        println!("type: count: {}", unit_type.count);
        println!("type: size: {}", unit_type.size);
        println!("type: armor: {}", unit_type.armor);
        println!("type: toughness: {}", unit_type.toughness);
        println!("type: weapon_skill: {}", unit_type.weapon_skill);
        println!("type: mp: {}", unit_type.move_points);
        println!("type: ap: {}", unit_type.attack_points);
        println!("type: reactive_ap: {}", unit_type.reactive_attack_points);
        println!("type: los_range: {}", unit_type.los_range);
        println!("type: cover_los_range: {}", unit_type.cover_los_range);
        let weapon_type = self.core.db().weapon_type(&unit_type.weapon_type_id);
        println!("weapon: name: {}", weapon_type.name);
        println!("weapon: damage: {}", weapon_type.damage);
        println!("weapon: ap: {}", weapon_type.ap);
        println!("weapon: accuracy: {}", weapon_type.accuracy);
        println!("weapon: max_distance: {}", weapon_type.max_distance);
    }

    fn print_terrain_info(&self, pos: &MapPos) {
        let state = &self.player_info.get(self.core.player_id()).game_state;
        match state.map().tile(pos) {
            &Terrain::Trees => println!("Trees"),
            &Terrain::Plain => println!("Plain"),
        }
    }

    fn print_info(&mut self) {
        self.pick_tile();
        match self.pick_result {
            PickResult::UnitId(ref id) => self.print_unit_info(id),
            PickResult::Pos(ref pos) => self.print_terrain_info(pos),
            _ => {},
        }
        println!("");
    }

    fn handle_event_key_press(&mut self, key: VirtualKeyCode) {
        let camera_move_speed_on_keypress = geom::HEX_EX_RADIUS;
        let s = camera_move_speed_on_keypress;
        match key {
            VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                self.should_close = true;
            },
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.camera.move_camera(rad(PI * 1.5), s);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.camera.move_camera(rad(PI * 0.5), s);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.camera.move_camera(rad(PI * 0.0), s);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.camera.move_camera(rad(PI * 1.0), s);
            },
            VirtualKeyCode::I => {
                self.print_info();
            },
            VirtualKeyCode::U => {
                self.pick_tile();
                self.create_unit();
            },
            VirtualKeyCode::L => {
                self.transport();
            },
            VirtualKeyCode::R => {
                self.change_reaction_fire_mode();
            },
            VirtualKeyCode::H => {
                self.pick_tile();
                if let PickResult::Pos(pos) = self.pick_result.clone() {
                    self.move_unit(&pos, &MoveMode::Hunt);
                } else {
                    panic!("Can`t move unit if no pos is selected");
                }
            },
            VirtualKeyCode::C => {
                let p = self.pick_world_pos();
                self.add_marker(&p);
            },
            VirtualKeyCode::Subtract | VirtualKeyCode::Key1 => {
                self.camera.change_zoom(1.3);
            },
            VirtualKeyCode::Equals | VirtualKeyCode::Key2 => {
                self.camera.change_zoom(0.7);
            },
            _ => println!("Unknown key pressed"),
        }
    }

    fn handle_event_lmb_release(&mut self) {
        self.is_lmb_pressed = false;
        if self.event_visualizer.is_some() {
            return;
        }
        if !self.is_tap(&self.mouse_pos) {
            return;
        }
        self.pick_tile();
        if let Some(button_id) = self.get_clicked_button_id() {
            self.handle_event_button_press(&button_id);
        }
        match self.pick_result.clone() {
            PickResult::Pos(pos) => {
                self.move_unit(&pos, &MoveMode::Fast);
            },
            PickResult::UnitId(unit_id) => {
                let player_id = {
                    let state = &self.player_info.get(self.core.player_id()).game_state;
                    let unit = state.units().get(&unit_id)
                        .expect("Can`t find picked unit in current state");
                    unit.player_id.clone()
                };
                if player_id == *self.core.player_id() {
                    self.select_unit();
                } else {
                    self.try_to_attack_unit();
                }
            },
            PickResult::None => {},
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
                self.handle_event_mouse_move(&pos);
            },
            Event::MouseInput(Pressed, MouseButton::Left) => {
                self.handle_event_lmb_press();
            },
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release();
            },
            Event::MouseInput(Pressed, MouseButton::Right) => {
                self.is_rmb_pressed = true;
            },
            Event::MouseInput(Released, MouseButton::Right) => {
                self.is_rmb_pressed = false;
            },
            Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(key);
            },
            Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                match phase {
                    glutin::TouchPhase::Moved => {
                        self.handle_event_mouse_move(&pos);
                    },
                    glutin::TouchPhase::Started => {
                        self.handle_event_mouse_move(&pos);
                        self.handle_event_lmb_press();
                    },
                    glutin::TouchPhase::Ended => {
                        self.handle_event_mouse_move(&pos);
                        self.handle_event_lmb_release();
                    },
                    glutin::TouchPhase::Cancelled => {
                        unimplemented!();
                    },
                }
            },
            _ => {},
        }
    }

    fn handle_events(&mut self) {
        let events = self.window.poll_events().collect::<Vec<_>>();
        if events.is_empty() {
            return;
        }
        for event in events {
            self.handle_event(&event);
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
        for node in &node.children {
            self.draw_scene_node(node, m);
        }
    }

    fn draw_scene_nodes(&self) {
        for (_, node) in &self.scene().nodes {
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

    fn draw_scene(&mut self, dtime: &Time) {
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
            event_visualizer.draw(&mut i.scene, dtime);
        }
    }

    fn draw(&mut self, dtime: &Time) {
        self.zgl.set_clear_color(&BG_COLOR);
        self.zgl.clear_screen();
        self.shader.activate(&self.zgl);
        self.shader.set_uniform_mat4f(
            &self.zgl,
            self.shader.get_mvp_mat(),
            &self.camera.mat(&self.zgl),
        );
        self.draw_scene(dtime);
        self.shader.set_uniform_color(
            &self.zgl, &self.basic_color_id, &zgl::BLACK);
        self.map_text_manager.draw(
            &self.zgl, &self.camera, &self.shader, dtime, &mut self.font_stash);
        self.button_manager.draw(
            &self.zgl,
            &self.win_size,
            &self.shader,
            self.shader.get_mvp_mat(),
        );
        self.window.swap_buffers()
            .ok().expect("Can`t swap buffers");
    }

    // TODO: Must return value.
    fn pick_tile(&mut self) {
        let p = self.pick_world_pos();
        let origin = MapPos{v: Vector2 {
            x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as ZInt,
            y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as ZInt,
        }};
        let origin_world_pos = geom::map_pos_to_world_pos(&origin);
        let mut closest_map_pos = origin.clone();
        let mut min_dist = (origin_world_pos.v - p.v).length();
        let state = &self.player_info.get_mut(self.core.player_id()).game_state;
        for map_pos in spiral_iter(&origin, 1) {
            let pos = geom::map_pos_to_world_pos(&map_pos);
            let d = (pos.v - p.v).length();
            if d < min_dist {
                min_dist = d;
                closest_map_pos = map_pos;
            }
        }
        let pos = closest_map_pos;
        if !state.map().is_inboard(&pos) {
            self.pick_result = PickResult::None;
        } else {
            let unit_at = get_unit_id_at(self.core.db(), state, &pos);
            if let Some(id) = unit_at {
                self.pick_result = PickResult::UnitId(id);
            } else {
                self.pick_result = PickResult::Pos(pos);
            }
        }
    }

    fn update_time(&mut self) -> Time {
        let time = precise_time_ns();
        let dtime = Time{n: time - self.last_time.n};
        self.last_time = Time{n: time};
        dtime
    }

    fn make_event_visualizer(
        &mut self,
        event: &CoreEvent,
    ) -> Box<EventVisualizer> {
        let current_player_id = self.core.player_id();
        let mut i = self.player_info.get_mut(current_player_id);
        let scene = &mut i.scene;
        let state = &i.game_state;
        match event {
            &CoreEvent::Move{ref unit_id, ref path, ..} => {
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
            &CoreEvent::CreateUnit{ref unit_info} => {
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, &unit_info.player_id);
                EventCreateUnitVisualizer::new(
                    self.core.db(), scene, unit_info, mesh_id, marker_mesh_id)
            },
            &CoreEvent::AttackUnit{ref attack_info} => {
                EventAttackUnitVisualizer::new(
                    state,
                    scene,
                    attack_info,
                    &self.mesh_ids.shell_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::ShowUnit{ref unit_info, ..} => {
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, &unit_info.player_id);
                EventShowUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    marker_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::HideUnit{ref unit_id} => {
                EventHideUnitVisualizer::new(
                    scene,
                    state,
                    unit_id,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::LoadUnit{ref passanger_id, ref transporter_id} => {
                let type_id = state.unit(passanger_id).type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                EventLoadUnitVisualizer::new(
                    scene,
                    state,
                    passanger_id,
                    &state.unit(transporter_id).pos,
                    unit_type_visual_info,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::UnloadUnit{ref unit_info, ref transporter_id} => {
                let type_id = state.unit(&unit_info.unit_id).type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, &unit_info.player_id);
                EventUnloadUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    marker_mesh_id,
                    &state.unit(transporter_id).pos,
                    unit_type_visual_info,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::SetReactionFireMode{ref unit_id, ref mode} => {
                EventSetReactionFireModeVisualizer::new(
                    state,
                    unit_id,
                    mode,
                    &mut self.map_text_manager,
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
        if let Some(CoreEvent::AttackUnit{ref attack_info})
            = self.event
        {
            let mut i = self.player_info.get_mut(self.core.player_id());
            let state = &mut i.game_state;
            let selected_unit_id = match self.selected_unit_id {
                Some(ref id) => id.clone(),
                None => return,
            };
            let defender = state.unit(&attack_info.defender_id);
            if selected_unit_id == attack_info.defender_id
                && defender.count - attack_info.killed <= 0
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
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            event_visualizer.end(scene, state);
        } else {
            panic!("end_event_visualization: self.event_visualizer == None");
        }
        if let Some(ref event) = self.event {
            state.apply_event(self.core.db(), event);
        } else {
            panic!("end_event_visualization: self.event == None");
        }
        self.event_visualizer = None;
        self.event = None;
        if let Some(ref selected_unit_id) = self.selected_unit_id {
            if let Some(unit) = state.units().get(selected_unit_id) {
                // TODO: do this only if this is last unshowed CoreEvent
                let pf = &mut i.pathfinder;
                pf.fill_map(self.core.db(), state, unit);
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
        let dtime = self.update_time();
        self.handle_events();
        self.logic();
        self.draw(&dtime);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
