// See LICENSE file for copyright and license details.

use std::sync::mpsc::{channel, Sender, Receiver};
use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use std::path::{Path};
use std::collections::{HashMap};
use cgmath::{
    Vector2,
    Vector3,
    Vector4,
    EuclideanVector,
    rad,
    Matrix3,
    Matrix4,
    SquareMatrix,
    Point,
    Point3,
};
use collision::{Plane, Ray, Intersect};
use glutin::{self, VirtualKeyCode, Event, MouseButton};
use glutin::ElementState::{Released};
use types::{Size2, ZInt, ZFloat, Time};
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
    MovePoints,
    UnitId,
    PlayerId,
    MapPos,
    ExactPos,
    SlotId,
    check_command,
    get_unit_ids_at,
    find_next_player_unit_id,
    find_prev_player_unit_id,
    get_free_exact_pos,
};
use core::db::{Db};
use obj;
use camera::Camera;
use gui::{ButtonManager, Button, ButtonId, is_tap};
use scene::{Scene, SceneNode};
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
use texture::{Texture, load_texture};
use mesh::{Mesh, MeshId};
use pipeline::{Vertex};
use fs;
use geom;
use screen::{Screen, ScreenCommand, EventStatus};
use context_menu_popup::{self, ContextMenuPopup};
use end_turn_screen::{EndTurnScreen};
use types::{ScreenPos, WorldPos};

fn get_initial_camera_pos(map_size: &Size2) -> WorldPos {
    let pos = get_max_camera_pos(map_size);
    WorldPos{v: Vector3{x: pos.v.x / 2.0, y: pos.v.y / 2.0, z: 0.0}}
}

fn get_max_camera_pos(map_size: &Size2) -> WorldPos {
    let map_pos = MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}};
    let pos = geom::map_pos_to_world_pos(&map_pos);
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

// TODO: `cond: F` -> `enum NameMe{Visible, Fogged}`
fn gen_tiles<F: Fn(bool) -> bool>(
    context: &mut Context,
    state: &PartialState,
    tex: Texture,
    cond: F,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for tile_pos in state.map().get_iter() {
        if !cond(state.is_tile_visible(&tile_pos)) {
            continue;
        }
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in dirs() {
            let vertex = geom::index_to_hex_vertex(dir.to_int());
            let uv = vertex.v.truncate() / (geom::HEX_EX_RADIUS * 2.0);
            let uv = uv + Vector2{x: 0.5, y: 0.5};
            vertices.push(Vertex {
                pos: (pos.v + vertex.v).into(),
                uv: uv.into(),
            });
        }
        indices.extend(&[
            i + 0, i + 1, i + 2,
            i + 0, i + 2, i + 3,
            i + 0, i + 3, i + 5,
            i + 3, i + 4, i + 5,
        ]);
        i += 6;
    }
    Mesh::new(context, &vertices, &indices, tex)
}

fn generate_visible_tiles_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    gen_tiles(context, state, tex, |vis| vis)
}

fn generate_fogged_tiles_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    gen_tiles(context, state, tex, |vis| !vis)
}

fn build_walkable_mesh(
    context: &mut Context,
    pf: &Pathfinder,
    map: &Map<Terrain>,
    move_points: &MovePoints,
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for tile_pos in map.get_iter() {
        if pf.get_map().tile(&tile_pos).cost().n > move_points.n {
            continue;
        }
        if let &Some(ref parent_dir) = pf.get_map().tile(&tile_pos).parent() {
            let tile_pos_to = Dir::get_neighbour_pos(&tile_pos, parent_dir);
            let exact_pos = ExactPos {
                map_pos: tile_pos.clone(),
                slot_id: pf.get_map().tile(&tile_pos).slot_id().clone(),
            };
            let exact_pos_to = ExactPos {
                map_pos: tile_pos_to.clone(),
                slot_id: pf.get_map().tile(&tile_pos_to).slot_id().clone(),
            };
            let world_pos_from = geom::exact_pos_to_world_pos(&exact_pos);
            let world_pos_to = geom::exact_pos_to_world_pos(&exact_pos_to);
            vertices.push(Vertex {
                pos: geom::lift(world_pos_from.v).into(),
                uv: [0.5, 0.5],
            });
            vertices.push(Vertex {
                pos: geom::lift(world_pos_to.v).into(),
                uv: [0.5, 0.5],
            });
            indices.extend(&[i, i + 1]);
            i += 2;
        }
    }
    Mesh::new_wireframe(context, &vertices, &indices)
}

fn build_targets_mesh(db: &Db, context: &mut Context, state: &PartialState, unit_id: &UnitId) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let unit = state.unit(unit_id);
    let mut i = 0;
    for (enemy_id, enemy) in state.units() {
        if unit.player_id == enemy.player_id {
            continue;
        }
        let command = Command::AttackUnit {
            attacker_id: unit_id.clone(),
            defender_id: enemy_id.clone(),
        };
        if !check_command(db, state, &command).is_ok() {
            continue;
        }
        let world_pos_from = geom::exact_pos_to_world_pos(&unit.pos);
        let world_pos_to = geom::exact_pos_to_world_pos(&enemy.pos);
        vertices.push(Vertex {
            pos: geom::lift(world_pos_from.v).into(),
            uv: [0.5, 0.5],
        });
        vertices.push(Vertex {
            pos: geom::lift(world_pos_to.v).into(),
            uv: [0.5, 0.5],
        });
        indices.extend(&[i, i + 1]);
        i += 2;
    }
    Mesh::new_wireframe(context, &vertices, &indices)
}

fn get_shell_mesh(context: &mut Context) -> Mesh {
    let w = 0.05;
    let l = w * 3.0;
    let vertices = [
        Vertex{pos: [-w, -l, 0.1], uv: [0.0, 0.0]},
        Vertex{pos: [-w, l, 0.1], uv: [0.0, 1.0]},
        Vertex{pos: [w, l, 0.1], uv: [1.0, 0.0]},
        Vertex{pos: [w, -l, 0.1], uv: [1.0, 0.0]},
    ];
    let indices = [0, 1, 2, 2, 3 ,4];
    let texture_data = fs::load("shell.png").into_inner();
    let texture = load_texture(&mut context.factory, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

fn get_marker<P: AsRef<Path>>(context: &mut Context, tex_path: P) -> Mesh {
    let n = 0.2;
    let vertices = [
        Vertex{pos: [-n, 0.0, 0.1], uv: [0.0, 0.0]},
        Vertex{pos: [0.0, n * 1.4, 0.1], uv: [1.0, 0.0]},
        Vertex{pos: [n, 0.0, 0.1], uv: [0.5, 0.5]},
    ];
    let indices = [0, 1, 2];
    let texture_data = fs::load(tex_path).into_inner();
    let texture = load_texture(&mut context.factory, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

fn load_object_mesh(context: &mut Context, name: &str) -> Mesh {
    let model = obj::Model::new(&format!("{}.obj", name));
    let (vertices, indices) = obj::build(&model);
    if model.is_wire() {
        Mesh::new_wireframe(context, &vertices, &indices)
    } else {
        let texture_data = fs::load(format!("{}.png", name)).into_inner();
        let texture = load_texture(&mut context.factory, &texture_data);
        Mesh::new(context, &vertices, &indices, texture)
    }
}

fn get_marker_mesh_id<'a>(mesh_ids: &'a MeshIdManager, player_id: &PlayerId) -> &'a MeshId {
    match player_id.id {
        0 => &mesh_ids.marker_1_mesh_id,
        1 => &mesh_ids.marker_2_mesh_id,
        n => panic!("Wrong player id: {}", n),
    }
}

struct MeshIdManager {
    big_building_mesh_w_id: MeshId,
    building_mesh_w_id: MeshId,
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
    context: &mut Context,
    meshes: &mut Vec<Mesh>,
) -> UnitTypeVisualInfoManager {
    let mut manager = UnitTypeVisualInfoManager::new();
    for &(unit_name, model_name, move_speed) in &[
        ("soldier", "soldier", 2.0),
        ("smg", "submachine", 2.0),
        ("scout", "scout", 2.5),
        ("mortar", "mortar", 1.5),
        ("field_gun", "field_gun", 1.5),
        ("light_spg", "light_spg", 3.0),
        ("light_tank", "light_tank", 3.0),
        ("medium_tank", "medium_tank", 2.5),
        ("heavy_tank", "tank", 2.0),
        ("mammoth_tank", "mammoth", 1.5),
        ("truck", "truck", 3.0),
        ("jeep", "jeep", 3.5),
    ] {
        manager.add_info(&db.unit_type_id(unit_name), UnitTypeVisualInfo {
            mesh_id: add_mesh(meshes, load_object_mesh(context, model_name)),
            move_speed: move_speed,
        });
    }
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

pub struct TacticalScreen {
    camera: Camera,
    map_text_manager: MapTextManager,
    // TODO: Move buttons to 'Gui'/'Ui' struct
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
    targets_mesh: Option<Mesh>,
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
        let floor_tex = load_texture(&mut context.factory, &fs::load("hex.png").into_inner());
        let mut meshes = Vec::new();
        let visible_map_mesh = generate_visible_tiles_mesh(
            context, &player_info.get(core.player_id()).game_state, floor_tex.clone());
        let fow_map_mesh = generate_fogged_tiles_mesh(
            context, &player_info.get(core.player_id()).game_state, floor_tex.clone());
        let selection_marker_mesh_id = add_mesh(
            &mut meshes, get_selection_mesh(context));
        let big_building_mesh_w_id = add_mesh(
            &mut meshes, load_object_mesh(context, "big_building_wire"));
        let building_mesh_w_id = add_mesh(
            &mut meshes, load_object_mesh(context, "building_wire"));
        let trees_mesh_id = add_mesh(
            &mut meshes, load_object_mesh(context, "trees"));
        let shell_mesh_id = add_mesh(
            &mut meshes, get_shell_mesh(context));
        let marker_1_mesh_id = add_mesh(
            &mut meshes, get_marker(context, "flag1.png"));
        let marker_2_mesh_id = add_mesh(
            &mut meshes, get_marker(context, "flag2.png"));
        let unit_type_visual_info
            = get_unit_type_visual_info(core.db(), context, &mut meshes);
        let mut camera = Camera::new(&context.win_size);
        camera.set_max_pos(get_max_camera_pos(&map_size));
        camera.set_pos(get_initial_camera_pos(&map_size));
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
            big_building_mesh_w_id: big_building_mesh_w_id,
            building_mesh_w_id: building_mesh_w_id,
            trees_mesh_id: trees_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_1_mesh_id: marker_1_mesh_id,
            marker_2_mesh_id: marker_2_mesh_id,
        };
        let map_text_manager = MapTextManager::new();
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
            targets_mesh: None,
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
        let im = self.camera.mat().invert()
            .expect("Can`t invert camera matrix");
        let w = context.win_size.w as ZFloat;
        let h = context.win_size.h as ZFloat;
        let x = context.mouse().pos.v.x as ZFloat;
        let y = context.mouse().pos.v.y as ZFloat;
        let x = (2.0 * x) / w - 1.0;
        let y = 1.0 - (2.0 * y) / h;
        let p0_raw = im * Vector4{x: x, y: y, z: 0.0, w: 1.0};
        let p0 = (p0_raw / p0_raw.w).truncate();
        let p1_raw = im * Vector4{x: x, y: y, z: 1.0, w: 1.0};
        let p1 = (p1_raw / p1_raw.w).truncate();
        let plane = Plane::from_abcd(0.0, 0.0, 1.0, 0.0);
        let ray = Ray::new(Point3::from_vec(p0), p1 - p0);
        let p = (plane, ray).intersection()
            .expect("Can`t find mouse ray/plane intersection");
        WorldPos{v: p.to_vec()}
    }

    fn add_marker(&mut self, pos: &WorldPos) {
        for (_, player_info) in self.player_info.info.iter_mut() {
            player_info.scene.add_node(SceneNode {
                pos: pos.clone(),
                rot: rad(0.0),
                mesh_id: Some(self.mesh_ids.shell_mesh_id.clone()),
                children: Vec::new(),
            });
        }
    }

    fn add_map_objects(&mut self) {
        for (_, player_info) in self.player_info.info.iter_mut() {
            let state = &player_info.game_state;
            let map = state.map();
            for tile_pos in map.get_iter() {
                if let &Terrain::Trees = map.tile(&tile_pos) {
                    let pos = geom::map_pos_to_world_pos(&tile_pos);
                    let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
                    player_info.scene.add_node(SceneNode {
                        pos: pos.clone(),
                        rot: rot,
                        mesh_id: Some(self.mesh_ids.trees_mesh_id.clone()),
                        children: Vec::new(),
                    });
                }
                if let &Terrain::City = map.tile(&tile_pos) {
                    let objects = state.objects_at(&tile_pos);
                    for object in objects {
                        let pos = geom::exact_pos_to_world_pos(&object.pos);
                        let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
                        player_info.scene.add_node(SceneNode {
                            pos: pos.clone(),
                            rot: rot,
                            mesh_id: Some(match object.pos.slot_id {
                                SlotId::Id(_) => self.mesh_ids.building_mesh_w_id.clone(),
                                SlotId::WholeTile => self.mesh_ids.big_building_mesh_w_id.clone(),
                            }),
                            children: Vec::new(),
                        });
                    }
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
        self.deselect_unit();
        self.core.do_command(Command::EndTurn);
    }

    fn deselect_unit(&mut self) {
        self.selected_unit_id = None;
        let i = self.player_info.get_mut(self.core.player_id());
        self.selection_manager.deselect(&mut i.scene);
        self.walkable_mesh = None;
        self.targets_mesh = None;
    }

    fn current_state(&self) -> &PartialState {
        &self.player_info.get(self.core.player_id()).game_state
    }

    fn can_unload_unit(&self, transporter_id: &UnitId, pos: &MapPos) -> Option<ExactPos> {
        let state = self.current_state();
        let transporter = state.unit(&transporter_id);
        let passenger_id = match transporter.passenger_id {
            Some(ref id) => id.clone(),
            None => return None,
        };
        let exact_pos = match get_free_exact_pos(
            self.core.db(),
            state,
            &state.unit(&passenger_id).type_id,
            pos,
        ) {
            Some(pos) => pos,
            None => return None,
        };
        if check_command(self.core.db(), state, &Command::UnloadUnit {
            transporter_id: transporter_id.clone(),
            passenger_id: passenger_id.clone(),
            pos: exact_pos.clone(),
        }).is_ok() {
            Some(exact_pos)
        } else {
            None
        }
    }

    // TODO: show commands preview
    fn try_create_context_menu_popup(
        &mut self,
        context: &mut Context,
        pos: &MapPos,
    ) {
        let options = self.get_context_menu_popup_options(pos);
        if options == context_menu_popup::Options::new() {
            return;
        }
        let mut menu_pos = context.mouse().pos.clone();
        menu_pos.v.y = context.win_size.h - menu_pos.v.y;
        let screen = ContextMenuPopup::new(
            context, &menu_pos, options, self.tx.clone());
        context.add_command(ScreenCommand::PushPopup(Box::new(screen)));
    }

    fn get_context_menu_popup_options(
        &mut self,
        pos: &MapPos,
    ) -> context_menu_popup::Options {
        let i = self.player_info.get(self.core.player_id());
        let state = &i.game_state;
        let db = self.core.db();
        let mut options = context_menu_popup::Options::new();
        let unit_ids = get_unit_ids_at(db, state, pos);
        if let Some(selected_unit_id) = self.selected_unit_id.clone() {
            for unit_id in &unit_ids {
                let unit = state.unit(&unit_id);
                if unit.player_id == *self.core.player_id() {
                    if *unit_id == selected_unit_id {
                        // TODO: do not show both options if unit has no weapons
                        if unit.reaction_fire_mode == ReactionFireMode::HoldFire {
                            options.enable_reaction_fire = Some(selected_unit_id.clone());
                        } else {
                            options.disable_reaction_fire = Some(selected_unit_id.clone());
                        }
                    } else {
                        options.selects.push(unit_id.clone());
                        let load_command = Command::LoadUnit {
                            transporter_id: selected_unit_id.clone(),
                            passenger_id: unit_id.clone(),
                        };
                        if check_command(db, state, &load_command).is_ok() {
                            options.loads.push(unit_id.clone());
                        }
                    }
                } else {
                    let attack_command = Command::AttackUnit {
                        attacker_id: selected_unit_id.clone(),
                        defender_id: unit_id.clone(),
                    };
                    if check_command(db, state, &attack_command).is_ok() {
                        options.attacks.push(unit_id.clone());
                    }
                }
            }
            if let Some(pos) = self.can_unload_unit(&selected_unit_id, pos) {
                options.unload_pos = Some(pos);
            }
            if let Some(destination) = get_free_exact_pos(
                db, state, &state.unit(&selected_unit_id).type_id, pos,
            ) {
                if let Some(path) = i.pathfinder.get_path(&destination) {
                    if check_command(db, state, &Command::Move {
                        unit_id: selected_unit_id.clone(),
                        path: path.clone(),
                        mode: MoveMode::Fast,
                    }).is_ok() {
                        options.move_pos = Some(destination.clone());
                    }
                    if check_command(db, state, &Command::Move {
                        unit_id: selected_unit_id.clone(),
                        path: path.clone(),
                        mode: MoveMode::Hunt,
                    }).is_ok() {
                        options.hunt_pos = Some(destination.clone());
                    }
                }
            }
        } else {
            for unit_id in &unit_ids {
                let unit = state.unit(&unit_id);
                if unit.player_id == *self.core.player_id() {
                    options.selects.push(unit_id.clone());
                }
            }
        }
        options
    }

    fn create_unit(&mut self, context: &Context) {
        let pick_result = self.pick_tile(context);
        if let Some(ref pos) = pick_result {
            let type_id = self.core.db().unit_type_id("soldier");
            let exact_pos = get_free_exact_pos(
                self.core.db(),
                self.current_state(),
                &type_id,
                pos,
            ).unwrap();
            let cmd = Command::CreateUnit{pos: exact_pos, type_id: type_id};
            self.core.do_command(cmd);
        }
    }

    // TODO: add ability to select enemy units
    fn select_unit(&mut self, context: &mut Context, unit_id: &UnitId) {
        self.selected_unit_id = Some(unit_id.clone());
        let mut i = self.player_info.get_mut(self.core.player_id());
        let state = &i.game_state;
        let pf = &mut i.pathfinder;
        pf.fill_map(self.core.db(), state, state.unit(unit_id));
        self.walkable_mesh = Some(build_walkable_mesh(
            context, pf, state.map(), &state.unit(unit_id).move_points));
        self.targets_mesh = Some(build_targets_mesh(
            self.core.db(), context, state, unit_id));
        let scene = &mut i.scene;
        self.selection_manager.create_selection_marker(
            state, scene, unit_id);
    }

    fn move_unit(&mut self, pos: &ExactPos, move_mode: &MoveMode) {
        let unit_id = self.selected_unit_id.as_ref().unwrap();
        let i = self.player_info.get_mut(self.core.player_id());
        // TODO: duplicated get_path =\
        let path = i.pathfinder.get_path(pos).unwrap();
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

    fn print_info(&mut self, context: &Context) {
        // TODO: move this to `fn Core::get_unit_info(...) -> &str`?
        let pick_result = self.pick_tile(context);
        if let Some(ref pos) = pick_result {
            core::print_terrain_info(self.current_state(), pos);
            println!("");
            for unit in self.current_state().units_at(pos) {
                core::print_unit_info(self.core.db(), unit);
                println!("");
            }
        }
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
        } else if let Some(pick_result) = pick_result {
            self.try_create_context_menu_popup(context, &pick_result);
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
        context: &mut Context,
        node: &SceneNode,
        m: Matrix4<ZFloat>,
    ) {
        let tr_mat = Matrix4::from_translation(node.pos.v);
        let rot_mat = Matrix4::from(Matrix3::from_angle_z(node.rot));
        let m = m * tr_mat * rot_mat;
        if let Some(ref mesh_id) = node.mesh_id {
            let id = mesh_id.id as usize;
            let mesh = &self.meshes[id];
            context.data.mvp = m.into(); // TODO: use separate model matrix
            if mesh.is_wire() {
                context.draw_mesh_with_color([0.0, 0.0, 0.0, 1.0], &mesh);
            } else {
                context.draw_mesh(&mesh);
            }
        }
        for node in &node.children {
            self.draw_scene_node(context, node, m);
        }
    }

    fn draw_scene_nodes(&self, context: &mut Context) {
        for (_, node) in self.scene().nodes() {
            let m = self.camera.mat();
            self.draw_scene_node(context, node, m);
        }
    }

    fn draw_map(&mut self, context: &mut Context) {
        context.data.mvp = self.camera.mat().into();
        context.data.basic_color = [0.85, 0.85, 0.85, 1.0];
        context.draw_mesh(&self.visible_map_mesh);
        context.data.basic_color = [0.5, 0.5, 0.5, 1.0];
        context.draw_mesh(&self.fow_map_mesh);
    }

    fn draw_scene(&mut self, context: &mut Context, dtime: &Time) {
        context.data.basic_color = [1.0, 1.0, 1.0, 1.0];
        self.draw_scene_nodes(context);
        self.draw_map(context);
        if let Some(ref walkable_mesh) = self.walkable_mesh {
            context.data.basic_color = [0.0, 0.0, 1.0, 1.0];
            context.draw_mesh(walkable_mesh);
        }
        if let Some(ref targets_mesh) = self.targets_mesh {
            context.data.basic_color = [1.0, 0.0, 0.0, 1.0];
            context.draw_mesh(targets_mesh);
        }
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let i = self.player_info.get_mut(self.core.player_id());
            event_visualizer.draw(&mut i.scene, dtime);
        }
    }

    fn draw(&mut self, context: &mut Context, dtime: &Time) {
        context.clear_color = [0.7, 0.7, 0.7, 1.0];
        context.encoder.clear(&context.data.out, context.clear_color);
        self.draw_scene(context, dtime);
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
        self.map_text_manager.draw(context, &self.camera, dtime);
        self.button_manager.draw(context);
    }

    fn pick_tile(&mut self, context: &Context) -> Option<MapPos> {
        let p = self.pick_world_pos(context);
        let origin = MapPos{v: Vector2 {
            x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as ZInt,
            y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as ZInt,
        }};
        let origin_world_pos = geom::map_pos_to_world_pos(&origin);
        let mut closest_map_pos = origin.clone();
        let mut min_dist = (origin_world_pos.v - p.v).magnitude();
        for map_pos in spiral_iter(&origin, 1) {
            let pos = geom::map_pos_to_world_pos(&map_pos);
            let d = (pos.v - p.v).magnitude();
            if d < min_dist {
                min_dist = d;
                closest_map_pos = map_pos;
            }
        }
        let pos = closest_map_pos;
        let state = self.current_state();
        if state.map().is_inboard(&pos) {
            Some(pos)
        } else {
            None
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
            &CoreEvent::Move{ref unit_id, ref to, ..} => {
                let type_id = state.unit(unit_id).type_id.clone();
                let visual_info = self.unit_type_visual_info.get(&type_id);
                EventMoveVisualizer::new(scene, unit_id, visual_info, to)
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
            &CoreEvent::LoadUnit{ref passenger_id, ref to, ..} => {
                let type_id = state.unit(passenger_id).type_id.clone();
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&type_id);
                EventLoadUnitVisualizer::new(
                    scene,
                    state,
                    passenger_id,
                    to,
                    unit_type_visual_info,
                    &mut self.map_text_manager,
                )
            },
            &CoreEvent::UnloadUnit{ref unit_info, ref from, ..} => {
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&unit_info.type_id);
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
                    from,
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

    fn start_event_visualization(&mut self, context: &mut Context, event: CoreEvent) {
        let vis = self.make_event_visualizer(&event);
        self.event = Some(event);
        self.event_visualizer = Some(vis);
        if self.is_event_visualization_finished() {
            self.end_event_visualization(context);
        } else {
            let i = &mut self.player_info.get_mut(self.core.player_id());
            self.selection_manager.deselect(&mut i.scene);
            self.walkable_mesh = None;
            self.targets_mesh = None;
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

    fn end_event_visualization(&mut self, context: &mut Context) {
        self.attacker_died_from_reaction_fire();
        {
            let i = self.player_info.get_mut(self.core.player_id());
            let scene = &mut i.scene;
            let state = &mut i.game_state;
            self.event_visualizer.as_mut().unwrap().end(scene, state);
            state.apply_event(self.core.db(), self.event.as_ref().unwrap());
            self.visible_map_mesh = generate_visible_tiles_mesh(context, state, self.floor_tex.clone());
            self.fow_map_mesh = generate_fogged_tiles_mesh(context, state, self.floor_tex.clone());
        }
        self.event_visualizer = None;
        self.event = None;
        if let Some(event) = self.core.get_event() {
            self.start_event_visualization(context, event);
        } else if let Some(ref unit_id) = self.selected_unit_id.clone() {
            self.select_unit(context, unit_id);
        }
    }

    fn logic(&mut self, context: &mut Context) {
        if self.event_visualizer.is_none() {
            if let Some(event) = self.core.get_event() {
                self.start_event_visualization(context, event);
            }
        } else if self.is_event_visualization_finished() {
            self.end_event_visualization(context);
        }
    }

    fn handle_context_menu_popup_command(
        &mut self,
        context: &mut Context,
        command: context_menu_popup::Command,
    ) {
        if let context_menu_popup::Command::Select{id} = command {
            self.select_unit(context, &id);
            return;
        }
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
                    pos: pos,
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

    fn handle_context_menu_popup_commands(&mut self, context: &mut Context) {
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
            Event::MouseMoved(x, y) => {
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
