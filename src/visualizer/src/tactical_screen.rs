// See LICENSE file for copyright and license details.

use std::error::{Error};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use std::path::{Path};
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
    SquareMatrix,
    Point,
    Point3,
};
use collision::{Plane, Ray, Intersect};
use glutin::{self, VirtualKeyCode, Event, MouseButton};
use glutin::ElementState::{Released};
use common::types::{Size2, ZInt, UnitId, PlayerId, MapPos, ZFloat};
use zgl::types::{ScreenPos, VertexCoord, TextureCoord, Time, WorldPos};
use zgl::{self, Zgl, MeshRenderMode};
use zgl::mesh::{Mesh, MeshId};
use zgl::camera::Camera;
use core::map::{Map, Terrain, spiral_iter};
use core::dir::{Dir, dirs};
use core::partial_state::{PartialState};
use core::game_state::{GameState, GameStateMut};
use core::pathfinder::{Pathfinder};
use core::{
    self,
    Core,
    CoreEvent,
    Command,
    MoveMode,
    ReactionFireMode,
    check_command,
    get_unit_id_at,
    find_next_player_unit_id,
    find_prev_player_unit_id,
};
use core::unit::{UnitClass};
use core::db::{Db};
use zgl::texture::{Texture};
use zgl::obj;
use zgl::font_stash::{FontStash};
use gui::{ButtonManager, Button, ButtonId, is_tap};
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
use context::{Context};
use geom;
use screen::{Screen, ScreenCommand, EventStatus};
use context_menu_popup::{self, ContextMenuPopup};
use end_turn_screen::{EndTurnScreen};

fn is_select_only_options(options: &context_menu_popup::Options) -> bool {
    let select_only_options = context_menu_popup::Options {
        show_select_button: true,
        ..context_menu_popup::Options::new()
    };
    *options == select_only_options
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

fn gen_tiles<F>(zgl: &Zgl, state: &PartialState, tex: &Texture, cond: F) -> Mesh
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

fn generate_visible_tiles_mesh(zgl: &Zgl, state: &PartialState, tex: &Texture) -> Mesh {
    gen_tiles(zgl, state, tex, |vis| vis)
}

fn generate_fogged_tiles_mesh(zgl: &Zgl, state: &PartialState, tex: &Texture) -> Mesh {
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

fn get_marker<P: AsRef<Path>>(zgl: &Zgl, tex_path: P) -> Mesh {
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
    let tex = Texture::new(zgl, &format!("{}.png", name));
    let obj = obj::Model::new(&format!("{}.obj", name));
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
    game_state: PartialState,
    pathfinder: Pathfinder,
    scene: Scene,
}

struct PlayerInfoManager {
    info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    fn new(map_size: &Size2, options: &core::Options) -> PlayerInfoManager {
        let mut m = HashMap::new();
        m.insert(PlayerId{id: 0}, PlayerInfo {
            game_state: PartialState::new(map_size, &PlayerId{id: 0}),
            pathfinder: Pathfinder::new(map_size),
            scene: Scene::new(),
        });
        if options.game_type == core::GameType::Hotseat {
            m.insert(PlayerId{id: 1}, PlayerInfo {
                game_state: PartialState::new(map_size, &PlayerId{id: 1}),
                pathfinder: Pathfinder::new(map_size),
                scene: Scene::new(),
            });
        }
        PlayerInfoManager{info: m}
    }

    fn get(&self, player_id: &PlayerId) -> &PlayerInfo {
        &self.info[player_id]
    }

    fn get_mut(&mut self, player_id: &PlayerId) -> &mut PlayerInfo {
        match self.info.get_mut(player_id) {
            Some(i) => i,
            None => panic!("Can`t find player_info for id={}", player_id.id),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum PickResult {
    Pos(MapPos),
    UnitId(UnitId),
}

impl PickResult {
    pub fn pos(&self) -> MapPos {
        if let &PickResult::Pos(ref pos) = self {
            pos.clone()
        } else {
            panic!("Error getting pos from PickResult")
        }
    }

    pub fn unit_id(&self) -> UnitId {
        if let &PickResult::UnitId(ref id) = self {
            id.clone()
        } else {
            panic!("Error getting unit_id from PickResult")
        }
    }
}

pub struct TacticalScreen {
    camera: Camera,
    map_text_manager: MapTextManager,
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    button_deselect_unit_id: ButtonId,
    button_next_unit_id: ButtonId,
    button_prev_unit_id: ButtonId,
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
    tx: Sender<context_menu_popup::Command>,
    rx: Receiver<context_menu_popup::Command>,
}

impl TacticalScreen {
    pub fn new(context: &mut Context, core_options: &core::Options) -> TacticalScreen {
        let core = Core::new(core_options);
        let map_size = core.map_size().clone();
        let player_info = PlayerInfoManager::new(&map_size, core_options);
        let floor_tex = Texture::new(&context.zgl, "floor.png"); // TODO: !!!
        let mut meshes = Vec::new();
        let visible_map_mesh = generate_visible_tiles_mesh(
            &context.zgl, &player_info.get(core.player_id()).game_state, &floor_tex);
        let fow_map_mesh = generate_fogged_tiles_mesh(
            &context.zgl, &player_info.get(core.player_id()).game_state, &floor_tex);
        let trees_mesh_id = add_mesh(
            &mut meshes, load_unit_mesh(&context.zgl, "trees"));
        let selection_marker_mesh_id = add_mesh(
            &mut meshes, get_selection_mesh(&context.zgl));
        let shell_mesh_id = add_mesh(
            &mut meshes, get_marker(&context.zgl, "shell.png"));
        let marker_1_mesh_id = add_mesh(
            &mut meshes, get_marker(&context.zgl, "flag1.png"));
        let marker_2_mesh_id = add_mesh(
            &mut meshes, get_marker(&context.zgl, "flag2.png"));
        let unit_type_visual_info
            = get_unit_type_visual_info(core.db(), &context.zgl, &mut meshes);
        let mut camera = Camera::new(&context.win_size);
        camera.set_max_pos(get_max_camera_pos(&map_size));
        camera.set_pos(get_initial_camera_pos(&map_size));
        let font_size = 40.0;
        let mut font_stash = FontStash::new(
            &context.zgl, "DroidSerif-Regular.ttf", font_size);
        let mut button_manager = ButtonManager::new();
        let mut pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let button_end_turn_id = button_manager.add_button(
            Button::new(context, "end turn", &pos));
        pos.v.y += (button_manager.buttons()[&button_end_turn_id].size().h as ZFloat * 1.2) as ZInt; // TODO
        let button_deselect_unit_id = button_manager.add_button(
            Button::new(context, "[X]", &pos));
        pos.v.x += button_manager.buttons()[&button_deselect_unit_id].size().w;
        let button_prev_unit_id = button_manager.add_button(
            Button::new(context, "[<]", &pos));
        pos.v.x += button_manager.buttons()[&button_prev_unit_id].size().w;
        let button_next_unit_id = button_manager.add_button(
            Button::new(context, "[>]", &pos));
        let mesh_ids = MeshIdManager {
            trees_mesh_id: trees_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_1_mesh_id: marker_1_mesh_id,
            marker_2_mesh_id: marker_2_mesh_id,
        };
        let map_text_manager = MapTextManager::new(&mut font_stash);
        let (tx, rx) = channel();
        let mut screen = TacticalScreen {
            camera: camera,
            button_manager: button_manager,
            button_end_turn_id: button_end_turn_id,
            button_deselect_unit_id: button_deselect_unit_id,
            button_prev_unit_id: button_prev_unit_id,
            button_next_unit_id: button_next_unit_id,
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
            tx: tx,
            rx: rx,
        };
        screen.add_map_objects();
        screen
    }

    fn pick_world_pos(&self, context: &Context) -> WorldPos {
        let im = self.camera.mat(&context.zgl).invert()
            .expect("Can`t invert camera matrix");
        let w = context.win_size.w as ZFloat;
        let h = context.win_size.h as ZFloat;
        let x = context.mouse().pos.v.x as ZFloat;
        let y = context.mouse().pos.v.y as ZFloat;
        let x = (2.0 * x) / w - 1.0;
        let y = 1.0 - (2.0 * y) / h;
        let p0_raw = im.mul_v(Vector4{x: x, y: y, z: 0.0, w: 1.0});
        let p0 = (p0_raw.div_s(p0_raw.w)).truncate();
        let p1_raw = im.mul_v(Vector4{x: x, y: y, z: 1.0, w: 1.0});
        let p1 = (p1_raw.div_s(p1_raw.w)).truncate();
        let plane = Plane::from_abcd(0.0, 0.0, 1.0, 0.0);
        let ray = Ray::new(Point3::from_vec(p0), p1 - p0);
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

    fn end_turn(&mut self, context: &mut Context) {
        if self.player_info.info.len() > 1 {
            let next_id = self.core.next_player_id(self.core.player_id());
            let screen = Box::new(EndTurnScreen::new(context, &next_id));
            context.add_command(ScreenCommand::PushScreen(screen));
        }
        self.core.do_command(Command::EndTurn);
        self.deselect_unit();
    }

    fn deselect_unit(&mut self) {
        self.selected_unit_id = None;
        let i = self.player_info.get_mut(self.core.player_id());
        self.selection_manager.deselect(&mut i.scene);
        self.walkable_mesh = None;
    }

    fn current_state(&self) -> &PartialState {
        &self.player_info.get(self.core.player_id()).game_state
    }

    fn is_tile_occupied(&self, pos: &MapPos) -> bool {
        self.current_state().is_tile_occupied(pos)
    }

    fn can_unload_unit(&self, transporter_id: &UnitId, pos: &MapPos) -> bool {
        let state = self.current_state();
        let transporter = state.unit(&transporter_id);
        let passenger_id = match transporter.passenger_id {
            Some(ref id) => id.clone(),
            None => return false,
        };
        check_command(self.core.db(), state, &Command::UnloadUnit {
            transporter_id: transporter_id.clone(),
            passenger_id: passenger_id.clone(),
            pos: pos.clone(),
        }).is_ok()
    }

    // TODO: show commands preview
    fn try_create_context_menu_popup(
        &mut self,
        context: &mut Context,
        selected_unit_id: &UnitId,
        pick_result: &PickResult,
    ) {
        // TODO: extract func 'context_context_menu_options'
        let mut options = context_menu_popup::Options::new();
        match pick_result {
            &PickResult::UnitId(ref unit_id) => {
                let state = self.current_state();
                let db = self.core.db();
                let player_id = self.current_state().unit(unit_id)
                    .player_id.clone();
                if player_id == *self.core.player_id() {
                    if *unit_id == *selected_unit_id {
                        // TODO: do not show both options if unit has no weapons
                        let unit = state.unit(&unit_id);
                        options.show_enable_reaction_fire
                            = unit.reaction_fire_mode == ReactionFireMode::HoldFire;
                        options.show_disable_reaction_fire
                            = unit.reaction_fire_mode == ReactionFireMode::Normal;
                    } else {
                        options.show_select_button = true;
                        let command = Command::LoadUnit {
                            transporter_id: selected_unit_id.clone(),
                            passenger_id: unit_id.clone(),
                        };
                        options.show_load_button = check_command(
                            db, state, &command).is_ok()
                    }
                } else {
                    let command = Command::AttackUnit {
                        attacker_id: selected_unit_id.clone(),
                        defender_id: unit_id.clone(),
                    };
                    options.show_attack_button = check_command(
                        db, state, &command).is_ok()
                }
            },
            &PickResult::Pos(ref pos) => {
                let db = self.core.db();
                let i = self.player_info.get(self.core.player_id());
                let state = &i.game_state;
                assert!(!self.is_tile_occupied(&pos));
                let path = match i.pathfinder.get_path(&pos) {
                    Some(path) => path,
                    None => return,
                };
                options.show_move_button = check_command(db, state, &Command::Move {
                    unit_id: selected_unit_id.clone(),
                    path: path.clone(),
                    mode: MoveMode::Fast,
                }).is_ok();
                options.show_hunt_button = check_command(db, state, &Command::Move {
                    unit_id: selected_unit_id.clone(),
                    path: path.clone(),
                    mode: MoveMode::Hunt,
                }).is_ok();
                options.show_unload_button = self.can_unload_unit(
                    &selected_unit_id, pos);
            },
        };
        if options == context_menu_popup::Options::new() {
            return;
        }
        if is_select_only_options(&options) {
            self.select_unit(context, &pick_result.unit_id());
            return;
        }
        let mut pos = context.mouse().pos.clone();
        pos.v.y = context.win_size.h - pos.v.y;
        let screen = ContextMenuPopup::new(
            context, &pos, options, pick_result, self.tx.clone());
        context.add_command(ScreenCommand::PushPopup(Box::new(screen)));
    }

    fn create_unit(&mut self, context: &Context) {
        let pick_result = self.pick_tile(context);
        if let Some(PickResult::Pos(ref pos)) = pick_result {
            if self.is_tile_occupied(pos) {
                return;
            }
            let cmd = Command::CreateUnit{pos: pos.clone()};
            self.core.do_command(cmd);
        }
    }

    // TODO: add ability to select enemy units
    fn select_unit(&mut self, context: &Context, unit_id: &UnitId) {
        self.selected_unit_id = Some(unit_id.clone());
        let mut i = self.player_info.get_mut(self.core.player_id());
        let state = &i.game_state;
        let pf = &mut i.pathfinder;
        pf.fill_map(self.core.db(), state, state.unit(unit_id));
        self.walkable_mesh = Some(build_walkable_mesh(
            &context.zgl, pf, state.map(), state.unit(unit_id).move_points));
        let scene = &mut i.scene;
        self.selection_manager.create_selection_marker(
            state, scene, unit_id);
        // TODO: highlight potential targets
    }

    fn move_unit(&mut self, pos: &MapPos, move_mode: &MoveMode) {
        let unit_id = self.selected_unit_id.as_ref().unwrap();
        assert!(!self.is_tile_occupied(&pos));
        let i = self.player_info.get_mut(self.core.player_id());
        let path = i.pathfinder.get_path(&pos).unwrap();
        self.core.do_command(Command::Move {
            unit_id: unit_id.clone(),
            path: path,
            mode: move_mode.clone(),
        });
    }

    fn handle_camera_move(&mut self, context: &Context, pos: &ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let camera_move_speed = geom::HEX_EX_RADIUS * 12.0;
        let per_x_pixel = camera_move_speed / (context.win_size.w as ZFloat);
        let per_y_pixel = camera_move_speed / (context.win_size.h as ZFloat);
        self.camera.move_camera(
            rad(PI), diff.x as ZFloat * per_x_pixel);
        self.camera.move_camera(
            rad(PI * 1.5), diff.y as ZFloat * per_y_pixel);
    }

    fn handle_camera_rotate(&mut self, context: &Context, pos: &ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let per_x_pixel = PI / (context.win_size.w as ZFloat);
        // TODO: get max angles from camera
        let per_y_pixel = (PI / 4.0) / (context.win_size.h as ZFloat);
        self.camera.add_horizontal_angle(
            rad(diff.x as ZFloat * per_x_pixel));
        self.camera.add_vertical_angle(
            rad(diff.y as ZFloat * per_y_pixel));
    }

    fn handle_event_mouse_move(&mut self, context: &Context, pos: &ScreenPos) {
        self.handle_event_mouse_move_platform(context, pos);
    }

    #[cfg(not(target_os = "android"))]
    fn handle_event_mouse_move_platform(&mut self, context: &Context, pos: &ScreenPos) {
        if context.mouse().is_left_button_pressed {
            self.handle_camera_move(context, pos);
        } else if context.mouse().is_right_button_pressed {
            self.handle_camera_rotate(context, pos);
        }
    }

    #[cfg(target_os = "android")]
    fn handle_event_mouse_move_platform(&mut self, context: &Context, pos: &ScreenPos) {
        if !context.mouse().is_left_button_pressed {
            return;
        }
        if self.must_rotate_camera(context) {
            self.handle_camera_rotate(context, pos);
        } else {
            self.handle_camera_move(context, pos);
        }
    }

    #[cfg(target_os = "android")]
    fn must_rotate_camera(&self, context: &Context) -> bool {
        if context.win_size.w > context.win_size.h {
            context.mouse().last_press_pos.v.x > context.win_size.w / 2
        } else {
            context.mouse().last_press_pos.v.y < context.win_size.h / 2
        }
    }

    fn print_unit_info(&self, unit_id: &UnitId) {
        let unit = self.current_state().unit(unit_id);
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
        match self.current_state().map().tile(pos) {
            &Terrain::Trees => println!("Trees"),
            &Terrain::Plain => println!("Plain"),
        }
    }

    fn print_info(&mut self, context: &Context) {
        // TODO: move this to `fn Core::get_unit_info(...) -> &str`?
        let pick_result = self.pick_tile(context);
        match pick_result {
            Some(PickResult::UnitId(ref id)) => self.print_unit_info(id),
            Some(PickResult::Pos(ref pos)) => self.print_terrain_info(pos),
            _ => {},
        }
        println!("");
    }

    fn handle_event_key_press(&mut self, context: &mut Context, key: VirtualKeyCode) {
        let camera_move_speed_on_keypress = geom::HEX_EX_RADIUS;
        let s = camera_move_speed_on_keypress;
        match key {
            VirtualKeyCode::Q | VirtualKeyCode::Escape => {
                context.add_command(ScreenCommand::PopScreen);
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
                self.print_info(context);
            },
            VirtualKeyCode::U => {
                self.create_unit(context);
            },
            VirtualKeyCode::C => {
                let p = self.pick_world_pos(context);
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

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if self.event_visualizer.is_some() {
            return;
        }
        if !is_tap(context) {
            return;
        }
        let pick_result = self.pick_tile(context);
        if let Some(button_id) = self.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, &button_id);
            return;
        }
        let pick_result = if let Some(pick_result) = pick_result {
            pick_result
        } else {
            return;
        };
        if let Some(id) = self.selected_unit_id.clone() {
            self.try_create_context_menu_popup(context, &id, &pick_result);
        } else if let PickResult::UnitId(ref unit_id) = pick_result {
            self.select_unit(context, unit_id);
        }
    }

    fn handle_event_button_press(&mut self, context: &mut Context, button_id: &ButtonId) {
        if *button_id == self.button_end_turn_id {
            self.end_turn(context);
        } else if *button_id == self.button_deselect_unit_id {
            self.deselect_unit();
        } else if *button_id == self.button_prev_unit_id {
            if let Some(id) = self.selected_unit_id.clone() {
                let prev_id = find_prev_player_unit_id(
                    self.current_state(), self.core.player_id(), &id);
                self.select_unit(context, &prev_id);
            }
        } else if *button_id == self.button_next_unit_id {
            if let Some(id) = self.selected_unit_id.clone() {
                let next_id = find_next_player_unit_id(
                    self.current_state(), self.core.player_id(), &id);
                self.select_unit(context, &next_id);
            }
        } else {
            panic!("BUTTON ID ERROR");
        }
    }

    fn scene(&self) -> &Scene {
        &self.player_info.get(self.core.player_id()).scene
    }

    fn draw_scene_node(
        &self,
        context: &Context,
        node: &SceneNode,
        m: Matrix4<ZFloat>,
    ) {
        let m = context.zgl.tr(m, &node.pos.v);
        let m = context.zgl.rot_z(m, &node.rot);
        if let Some(ref mesh_id) = node.mesh_id {
            context.shader.set_uniform_mat4f(
                &context.zgl, context.shader.get_mvp_mat(), &m);
            let id = mesh_id.id as usize;
            self.meshes[id].draw(&context.zgl, &context.shader);
        }
        for node in &node.children {
            self.draw_scene_node(context, node, m);
        }
    }

    fn draw_scene_nodes(&self, context: &Context) {
        for (_, node) in &self.scene().nodes {
            let m = self.camera.mat(&context.zgl);
            self.draw_scene_node(context, node, m);
        }
    }

    fn draw_map(&mut self, context: &Context) {
        context.shader.set_uniform_mat4f(
            &context.zgl,
            context.shader.get_mvp_mat(),
            &self.camera.mat(&context.zgl),
        );
        context.set_basic_color(&zgl::GREY);
        self.fow_map_mesh.draw(&context.zgl, &context.shader);
        context.set_basic_color(&zgl::WHITE);
        self.visible_map_mesh.draw(&context.zgl, &context.shader);
    }

    fn draw_scene(&mut self, context: &Context, dtime: &Time) {
        context.set_basic_color(&zgl::WHITE);
        self.draw_scene_nodes(context);
        self.draw_map(context);
        if let Some(ref walkable_mesh) = self.walkable_mesh {
            context.set_basic_color(&zgl::BLUE);
            walkable_mesh.draw(&context.zgl, &context.shader);
        }
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let i = self.player_info.get_mut(self.core.player_id());
            event_visualizer.draw(&mut i.scene, dtime);
        }
    }

    fn draw(&mut self, context: &mut Context, dtime: &Time) {
        self.draw_scene(context, dtime);
        context.set_basic_color(&zgl::BLACK);
        self.map_text_manager.draw(context, &self.camera, dtime);
        self.button_manager.draw(&context);
    }

    fn pick_tile(&mut self, context: &Context) -> Option<PickResult> {
        let p = self.pick_world_pos(context);
        let origin = MapPos{v: Vector2 {
            x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as ZInt,
            y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as ZInt,
        }};
        let origin_world_pos = geom::map_pos_to_world_pos(&origin);
        let mut closest_map_pos = origin.clone();
        let mut min_dist = (origin_world_pos.v - p.v).length();
        let state = self.current_state();
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
            None
        } else {
            let unit_at = get_unit_id_at(self.core.db(), state, &pos);
            if let Some(id) = unit_at {
                Some(PickResult::UnitId(id))
            } else {
                Some(PickResult::Pos(pos))
            }
        }
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
                let type_id = state.unit(unit_id).type_id.clone();
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
            &CoreEvent::LoadUnit{ref passenger_id, ref transporter_id} => {
                let type_id = state.unit(passenger_id).type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                EventLoadUnitVisualizer::new(
                    scene,
                    state,
                    passenger_id,
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

    fn start_event_visualization(&mut self, context: &Context, event: CoreEvent) {
        let vis = self.make_event_visualizer(&event);
        self.event = Some(event);
        self.event_visualizer = Some(vis);
        if self.is_event_visualization_finished() {
            self.end_event_visualization(context);
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

    fn end_event_visualization(&mut self, context: &Context) {
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
                    &context.zgl, pf, state.map(), unit.move_points));
                self.selection_manager.create_selection_marker(
                    state, scene, selected_unit_id);
            }
        }
        // TODO: recolor terrain objects
        self.visible_map_mesh = generate_visible_tiles_mesh(
            &context.zgl, state, &self.floor_tex);
        self.fow_map_mesh = generate_fogged_tiles_mesh(
            &context.zgl, state, &self.floor_tex);
    }

    fn logic(&mut self, context: &Context) {
        while self.event_visualizer.is_none() {
            // TODO: convert to iterator
            if let Some(event) = self.core.get_event() {
                self.start_event_visualization(context, event);
            } else {
                break;
            }
        }
        if self.event_visualizer.is_some()
            && self.is_event_visualization_finished()
        {
            self.end_event_visualization(context);
        }
    }

    fn handle_context_menu_popup_command(
        &mut self,
        context: &Context,
        command: context_menu_popup::Command,
    ) {
        let selected_unit_id = self.selected_unit_id.clone().unwrap();
        match command {
            context_menu_popup::Command::Select{id} => {
                self.select_unit(context, &id);
            },
            context_menu_popup::Command::Move{pos} => {
                self.move_unit(&pos, &MoveMode::Fast);
            },
            context_menu_popup::Command::Hunt{pos} => {
                self.move_unit(&pos, &MoveMode::Hunt);
            },
            context_menu_popup::Command::Attack{id} => {
                self.core.do_command(Command::AttackUnit {
                    attacker_id: selected_unit_id.clone(),
                    defender_id: id.clone(),
                });
            },
            context_menu_popup::Command::LoadUnit{passenger_id} => {
                self.core.do_command(Command::LoadUnit {
                    transporter_id: selected_unit_id.clone(),
                    passenger_id: passenger_id.clone(),
                });
            },
            context_menu_popup::Command::UnloadUnit{pos} => {
                let passenger_id = {
                    let transporter = self.current_state()
                        .unit(&selected_unit_id);
                    transporter.passenger_id.clone().unwrap()
                };
                self.core.do_command(Command::UnloadUnit {
                    transporter_id: selected_unit_id.clone(),
                    passenger_id: passenger_id.clone(),
                    pos: pos.clone(),
                });
            },
            context_menu_popup::Command::EnableReactionFire{id} => {
                self.core.do_command(Command::SetReactionFireMode {
                    unit_id: id,
                    mode: ReactionFireMode::Normal,
                });
            },
            context_menu_popup::Command::DisableReactionFire{id} => {
                self.core.do_command(Command::SetReactionFireMode {
                    unit_id: id,
                    mode: ReactionFireMode::HoldFire,
                });
            },
        }
    }

    fn handle_context_menu_popup_commands(&mut self, context: &Context) {
        while let Ok(command) = self.rx.try_recv() {
            self.handle_context_menu_popup_command(context, command);
        }
    }
}

impl Screen for TacticalScreen {
    fn tick(&mut self, context: &mut Context, dtime: &Time) {
        self.logic(context);
        self.draw(context, dtime);
        self.handle_context_menu_popup_commands(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) -> EventStatus {
        match *event {
            Event::Resized(..) => {
                self.camera.regenerate_projection_mat(&context.win_size);
            },
            Event::MouseMoved((x, y)) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                self.handle_event_mouse_move(context, &pos);
            },
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(context, key);
            },
            Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                let pos = ScreenPos{v: Vector2{x: x as ZInt, y: y as ZInt}};
                match phase {
                    glutin::TouchPhase::Moved => {
                        self.handle_event_mouse_move(context, &pos);
                    },
                    glutin::TouchPhase::Started => {
                        self.handle_event_mouse_move(context, &pos);
                    },
                    glutin::TouchPhase::Ended => {
                        self.handle_event_mouse_move(context, &pos);
                        self.handle_event_lmb_release(context);
                    },
                    glutin::TouchPhase::Cancelled => {
                        unimplemented!();
                    },
                }
            },
            _ => {},
        }
        EventStatus::Handled
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
