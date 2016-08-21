use std::sync::mpsc::{channel, Sender, Receiver};
use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use std::path::{Path};
use std::iter::IntoIterator;
use std::collections::{HashMap};
use cgmath::{
    Array,
    Vector2,
    Vector3,
    Vector4,
    InnerSpace,
    rad,
    Matrix3,
    Matrix4,
    SquareMatrix,
    EuclideanSpace,
    Point3,
};
use collision::{Plane, Ray, Intersect};
use glutin::{self, VirtualKeyCode, Event, MouseButton, TouchPhase};
use glutin::ElementState::{Released};
use types::{Size2, Time};
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
    ObjectClass,
    Sector,
    SectorId,
    Score,
    check_command,
    get_unit_ids_at,
    find_next_player_unit_id,
    find_prev_player_unit_id,
    get_free_exact_pos,
};
use core::db::{Db};
use core::unit::{UnitTypeId};
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
    EventSectorOwnerChangedVisualizer,
    EventVictoryPointVisualizer,
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
use game_results_screen::{GameResultsScreen};
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

// TODO: get from Core
fn target_score() -> Score {
    Score{n: 5}
}

fn score_text(state: &PartialState) -> String {
    let score = state.score();
    // TODO: get rid of magic num
    format!("P0:{}/{}, P1:{}/{}",
        score[&PlayerId{id: 0}].n,
        target_score().n,
        score[&PlayerId{id: 1}].n,
        target_score().n,
    )
}

fn generate_tiles_mesh<I: IntoIterator<Item=MapPos>>(
    context: &mut Context,
    tex: Texture,
    positions: I
) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut i = 0;
    for tile_pos in positions {
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in dirs() {
            let vertex = geom::index_to_hex_vertex(dir.to_int());
            let uv = vertex.v.truncate() / (geom::HEX_EX_RADIUS * 2.0)
                + Vector2::from_value(0.5);
            vertices.push(Vertex {
                pos: (pos.v + vertex.v).into(),
                uv: uv.into(),
            });
        }
        indices.extend_from_slice(&[
            i, i + 1, i + 2,
            i, i + 2, i + 3,
            i, i + 3, i + 5,
            i + 3, i + 4, i + 5,
        ]);
        i += 6;
    }
    Mesh::new(context, &vertices, &indices, tex)
}

fn generate_sector_mesh(context: &mut Context, sector: &Sector, tex: Texture) -> Mesh {
    generate_tiles_mesh(context, tex, sector.positions.to_vec())
}

fn generate_map_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    let mut normal_positions = Vec::new();
    for tile_pos in state.map().get_iter() {
        if *state.map().tile(&tile_pos) != Terrain::Water {
            normal_positions.push(tile_pos);
        }
    }
    generate_tiles_mesh(context, tex, normal_positions)
}

fn generate_water_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    let mut normal_positions = Vec::new();
    for pos in state.map().get_iter() {
        if *state.map().tile(&pos) == Terrain::Water {
            normal_positions.push(pos);
        }
    }
    generate_tiles_mesh(context, tex, normal_positions)
}

fn generate_fogged_tiles_mesh(context: &mut Context, state: &PartialState, tex: Texture) -> Mesh {
    let mut fogged_positions = Vec::new();
    for tile_pos in state.map().get_iter() {
        if !state.is_tile_visible(&tile_pos) {
            fogged_positions.push(tile_pos);
        }
    }
    generate_tiles_mesh(context, tex, fogged_positions)
}

fn empty_mesh(context: &mut Context) -> Mesh {
    Mesh::new_wireframe(context, &[], &[])
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
        if let Some(ref parent_dir) = *pf.get_map().tile(&tile_pos).parent() {
            let tile_pos_to = Dir::get_neighbour_pos(&tile_pos, parent_dir);
            let exact_pos = ExactPos {
                map_pos: tile_pos,
                slot_id: pf.get_map().tile(&tile_pos).slot_id().clone(),
            };
            let exact_pos_to = ExactPos {
                map_pos: tile_pos_to,
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
            indices.extend_from_slice(&[i, i + 1]);
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
        indices.extend_from_slice(&[i, i + 1]);
        i += 2;
    }
    Mesh::new_wireframe(context, &vertices, &indices)
}

fn get_shell_mesh(context: &mut Context) -> Mesh {
    let w = 0.05;
    let l = w * 3.0;
    let h = 0.1;
    let vertices = [
        Vertex{pos: [-w, -l, h], uv: [0.0, 0.0]},
        Vertex{pos: [-w, l, h], uv: [0.0, 1.0]},
        Vertex{pos: [w, l, h], uv: [1.0, 0.0]},
        Vertex{pos: [w, -l, h], uv: [1.0, 0.0]},
    ];
    let indices = [0, 1, 2, 2, 3, 0];
    let texture_data = fs::load("shell.png").into_inner();
    let texture = load_texture(context, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

fn get_road_mesh(context: &mut Context) -> Mesh {
    let w = geom::HEX_EX_RADIUS * 0.3;
    let l = geom::HEX_EX_RADIUS;
    let h = geom::MIN_LIFT_HEIGHT / 2.0;
    let vertices = [
        Vertex{pos: [-w, -l, h], uv: [0.0, 0.0]},
        Vertex{pos: [-w, l, h], uv: [0.0, 1.0]},
        Vertex{pos: [w, l, h], uv: [1.0, 1.0]},
        Vertex{pos: [w, -l, h], uv: [1.0, 0.0]},
    ];
    let indices = [0, 1, 2, 2, 3, 0];
    let texture_data = fs::load("road.png").into_inner();
    let texture = load_texture(context, &texture_data);
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
    let texture = load_texture(context, &texture_data);
    Mesh::new(context, &vertices, &indices, texture)
}

fn load_object_mesh(context: &mut Context, name: &str) -> Mesh {
    let model = obj::Model::new(&format!("{}.obj", name));
    let (vertices, indices) = obj::build(&model);
    if model.is_wire() {
        Mesh::new_wireframe(context, &vertices, &indices)
    } else {
        let texture_data = fs::load(format!("{}.png", name)).into_inner();
        let texture = load_texture(context, &texture_data);
        Mesh::new(context, &vertices, &indices, texture)
    }
}

fn get_marker_mesh_id(mesh_ids: &MeshIdManager, player_id: PlayerId) -> &MeshId {
    // TODO: use one mesh, just different node colors
    match player_id.id {
        0 => &mesh_ids.marker_1_mesh_id,
        1 => &mesh_ids.marker_2_mesh_id,
        n => panic!("Wrong player id: {}", n),
    }
}

struct MeshManager {
    meshes: Vec<Mesh>,
}

impl MeshManager {
    fn new() -> MeshManager {
        MeshManager {
            meshes: Vec::new(),
        }
    }

    fn add(&mut self, mesh: Mesh) -> MeshId {
        self.meshes.push(mesh);
        MeshId{id: (self.meshes.len() as i32) - 1}
    }

    fn set(&mut self, id: MeshId, mesh: Mesh) {
        let index = id.id as usize;
        self.meshes[index] = mesh;
    }

    fn get(&self, id: MeshId) -> &Mesh {
        let index = id.id as usize;
        &self.meshes[index]
    }
}

struct MeshIdManager {
    big_building_mesh_w_id: MeshId,
    building_mesh_w_id: MeshId,
    road_mesh_id: MeshId,
    trees_mesh_id: MeshId,
    shell_mesh_id: MeshId,
    marker_1_mesh_id: MeshId,
    marker_2_mesh_id: MeshId,
    walkable_mesh_id: MeshId,
    targets_mesh_id: MeshId,
    map_mesh_id: MeshId,
    water_mesh_id: MeshId,
    fow_mesh_id: MeshId,
    selection_marker_mesh_id: MeshId,
    sector_mesh_ids: HashMap<SectorId, MeshId>,
}

impl MeshIdManager {
    fn new(
        context: &mut Context,
        meshes: &mut MeshManager,
        state: &PartialState,
    ) -> MeshIdManager {
        let floor_tex = load_texture(context, &fs::load("hex.png").into_inner());
        let chess_grid_tex = load_texture(context, &fs::load("chess_grid.png").into_inner());
        let map_mesh_id = meshes.add(generate_map_mesh(
            context, state, floor_tex.clone()));
        let water_mesh_id = meshes.add(generate_water_mesh(
            context, state, floor_tex.clone()));
        let fow_mesh_id = meshes.add(generate_fogged_tiles_mesh(
            context, state, floor_tex.clone()));
        let mut sector_mesh_ids = HashMap::new();
        for (id, sector) in state.sectors() {
            let mesh_id = meshes.add(generate_sector_mesh(
                context, sector, chess_grid_tex.clone()));
            sector_mesh_ids.insert(*id, mesh_id);
        }
        let selection_marker_mesh_id = meshes.add(get_selection_mesh(context));
        let big_building_mesh_w_id = meshes.add(
            load_object_mesh(context, "big_building_wire"));
        let building_mesh_w_id = meshes.add(
            load_object_mesh(context, "building_wire"));
        let trees_mesh_id = meshes.add(load_object_mesh(context, "trees"));
        let shell_mesh_id = meshes.add(get_shell_mesh(context));
        let road_mesh_id = meshes.add(get_road_mesh(context));
        // TODO: use one mesh but with different node colors
        let marker_1_mesh_id = meshes.add(get_marker(context, "flag1.png"));
        let marker_2_mesh_id = meshes.add(get_marker(context, "flag2.png"));
        let walkable_mesh_id = meshes.add(empty_mesh(context));
        let targets_mesh_id = meshes.add(empty_mesh(context));
        MeshIdManager {
            big_building_mesh_w_id: big_building_mesh_w_id,
            building_mesh_w_id: building_mesh_w_id,
            trees_mesh_id: trees_mesh_id,
            road_mesh_id: road_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_1_mesh_id: marker_1_mesh_id,
            marker_2_mesh_id: marker_2_mesh_id,
            walkable_mesh_id: walkable_mesh_id,
            targets_mesh_id: targets_mesh_id,
            map_mesh_id: map_mesh_id,
            water_mesh_id: water_mesh_id,
            fow_mesh_id: fow_mesh_id,
            selection_marker_mesh_id: selection_marker_mesh_id,
            sector_mesh_ids: sector_mesh_ids,
        }
    }
}

fn get_unit_type_visual_info(
    db: &Db,
    context: &mut Context,
    meshes: &mut MeshManager,
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
            mesh_id: meshes.add(load_object_mesh(context, model_name)),
            move_speed: move_speed,
        });
    }
    manager
}

struct PlayerInfo {
    game_state: PartialState,
    pathfinder: Pathfinder,
    scene: Scene,
    camera: Camera,
}

struct PlayerInfoManager {
    info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    fn new(context: &Context, map_size: &Size2, options: &core::Options) -> PlayerInfoManager {
        let mut m = HashMap::new();
        let mut camera = Camera::new(&context.win_size);
        camera.set_max_pos(get_max_camera_pos(map_size));
        camera.set_pos(get_initial_camera_pos(map_size));
        m.insert(PlayerId{id: 0}, PlayerInfo {
            game_state: PartialState::new(map_size, &PlayerId{id: 0}),
            pathfinder: Pathfinder::new(map_size),
            scene: Scene::new(),
            camera: camera.clone(),
        });
        if options.game_type == core::GameType::Hotseat {
            m.insert(PlayerId{id: 1}, PlayerInfo {
                game_state: PartialState::new(map_size, &PlayerId{id: 1}),
                pathfinder: Pathfinder::new(map_size),
                scene: Scene::new(),
                camera: camera,
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

pub struct Gui {
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    button_deselect_unit_id: ButtonId,
    button_next_unit_id: ButtonId,
    button_prev_unit_id: ButtonId,
    label_unit_info_id: Option<ButtonId>,
    label_score_id: ButtonId,
}

impl Gui {
    fn new(context: &mut Context, state: &PartialState) -> Gui {
        let mut button_manager = ButtonManager::new();
        let mut pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let button_end_turn_id = button_manager.add_button(
            Button::new(context, "[end turn]", &pos));
        pos.v.y += (button_manager.buttons()[&button_end_turn_id].size().h as f32 * 1.2) as i32; // TODO
        let button_deselect_unit_id = button_manager.add_button(
            Button::new(context, "[X]", &pos));
        pos.v.x += button_manager.buttons()[&button_deselect_unit_id].size().w;
        let button_prev_unit_id = button_manager.add_button(
            Button::new(context, "[<]", &pos));
        pos.v.x += button_manager.buttons()[&button_prev_unit_id].size().w;
        let button_next_unit_id = button_manager.add_button(
            Button::new(context, "[>]", &pos));
        let label_score_id = {
            let vp_pos = ScreenPos{v: Vector2 {
                x: context.win_size.w - 10,
                y: context.win_size.h - 10,
            }};
            let text = score_text(state);
            let mut label_score = Button::new_small(context, &text, &vp_pos);
            let mut pos = label_score.pos().clone();
            pos.v.y -= label_score.size().h;
            pos.v.x -= label_score.size().w;
            label_score.set_pos(pos);
            button_manager.add_button(label_score)
        };
        Gui {
            button_manager: button_manager,
            button_end_turn_id: button_end_turn_id,
            button_deselect_unit_id: button_deselect_unit_id,
            button_prev_unit_id: button_prev_unit_id,
            button_next_unit_id: button_next_unit_id,
            label_unit_info_id: None,
            label_score_id: label_score_id,
        }
    }
}

fn make_scene(state: &PartialState, mesh_ids: &MeshIdManager) -> Scene {
    let mut scene = Scene::new();
    let map = state.map();
    scene.add_node(SceneNode {
        pos: WorldPos{v: Vector3::from_value(0.0)},
        rot: rad(0.0),
        mesh_id: Some(mesh_ids.walkable_mesh_id),
        color: [0.0, 0.0, 1.0, 1.0],
        children: Vec::new(),
    });
    scene.add_node(SceneNode {
        pos: WorldPos{v: Vector3::from_value(0.0)},
        rot: rad(0.0),
        mesh_id: Some(mesh_ids.targets_mesh_id),
        color: [1.0, 0.0, 0.0, 1.0],
        children: Vec::new(),
    });
    scene.add_node(SceneNode {
        pos: WorldPos{v: Vector3::from_value(0.0)},
        rot: rad(0.0),
        mesh_id: Some(mesh_ids.map_mesh_id),
        color: [0.8, 0.9, 0.3, 1.0],
        children: Vec::new(),
    });
    scene.add_node(SceneNode {
        pos: WorldPos{v: Vector3::from_value(0.0)},
        rot: rad(0.0),
        mesh_id: Some(mesh_ids.water_mesh_id),
        color: [0.6, 0.6, 0.9, 1.0],
        children: Vec::new(),
    });
    for (sector_id, sector_mesh_id) in &mesh_ids.sector_mesh_ids {
        scene.add_sector(*sector_id, SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.015}}, // TODO
            rot: rad(0.0),
            mesh_id: Some(*sector_mesh_id),
            color: [1.0, 1.0, 1.0, 0.5],
            children: Vec::new(),
        });
    }
    scene.add_node(SceneNode {
        pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.02}}, // TODO
        rot: rad(0.0),
        mesh_id: Some(mesh_ids.fow_mesh_id),
        color: [0.0, 0.1, 0.0, 0.3],
        children: Vec::new(),
    });
    for tile_pos in map.get_iter() {
        let objects = state.objects_at(&tile_pos);
        if *map.tile(&tile_pos) == Terrain::Trees {
            let pos = geom::map_pos_to_world_pos(&tile_pos);
            let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
            scene.add_node(SceneNode {
                pos: pos,
                rot: rot,
                mesh_id: Some(mesh_ids.trees_mesh_id),
                color: [1.0, 1.0, 1.0, 1.0],
                children: Vec::new(),
            });
        }
        for object in objects {
            match object.class {
                ObjectClass::Building => {
                    let pos = geom::exact_pos_to_world_pos(&object.pos);
                    let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
                    scene.add_node(SceneNode {
                        pos: pos,
                        rot: rot,
                        mesh_id: Some(match object.pos.slot_id {
                            SlotId::Id(_) => mesh_ids.building_mesh_w_id,
                            SlotId::WholeTile => mesh_ids.big_building_mesh_w_id,
                            SlotId::TwoTiles(_) => unimplemented!(),
                        }),
                        color: [0.0, 0.0, 0.0, 1.0],
                        children: Vec::new(),
                    });
                }
                ObjectClass::Road => {
                    let pos = geom::exact_pos_to_world_pos(&object.pos);
                    let rot = match object.pos.slot_id {
                        SlotId::TwoTiles(ref dir) => {
                            rad(dir.to_int() as f32 * PI / 3.0 + PI / 6.0)
                        },
                        _ => panic!(),
                    };
                    scene.add_node(SceneNode {
                        pos: pos,
                        rot: rot,
                        mesh_id: Some(mesh_ids.road_mesh_id),
                        color: [1.0, 1.0, 1.0, 1.0],
                        children: Vec::new(),
                    });
                }
            }
        }
    }
    scene
}

pub struct TacticalScreen {
    map_text_manager: MapTextManager,
    gui: Gui,
    player_info: PlayerInfoManager,
    core: Core,
    event: Option<CoreEvent>,
    event_visualizer: Option<Box<EventVisualizer>>,
    mesh_ids: MeshIdManager,
    meshes: MeshManager,
    unit_type_visual_info: UnitTypeVisualInfoManager,
    selected_unit_id: Option<UnitId>,
    selection_manager: SelectionManager,
    tx: Sender<context_menu_popup::Command>,
    rx: Receiver<context_menu_popup::Command>,
}

impl TacticalScreen {
    pub fn new(context: &mut Context, core_options: &core::Options) -> TacticalScreen {
        let core = Core::new(core_options);
        let map_size = core.map_size().clone();
        let mut player_info = PlayerInfoManager::new(context, &map_size, core_options);
        let mut meshes = MeshManager::new();
        let mesh_ids = MeshIdManager::new(
            context,
            &mut meshes,
            &player_info.get(core.player_id()).game_state,
        );
        let unit_type_visual_info
            = get_unit_type_visual_info(core.db(), context, &mut meshes);
        let map_text_manager = MapTextManager::new();
        let (tx, rx) = channel();
        let gui = Gui::new(context, &player_info.get(core.player_id()).game_state);
        let selection_manager = SelectionManager::new(mesh_ids.selection_marker_mesh_id);
        for (_, player_info) in &mut player_info.info {
            player_info.scene = make_scene(&player_info.game_state, &mesh_ids);
        }
        TacticalScreen {
            gui: gui,
            player_info: player_info,
            core: core,
            event: None,
            event_visualizer: None,
            mesh_ids: mesh_ids,
            meshes: meshes,
            unit_type_visual_info: unit_type_visual_info,
            selected_unit_id: None,
            selection_manager: selection_manager,
            map_text_manager: map_text_manager,
            tx: tx,
            rx: rx,
        }
    }

    fn pick_world_pos(&self, context: &Context) -> WorldPos {
        let camera = &self.current_player_info().camera;
        let im = camera.mat().invert()
            .expect("Can`t invert camera matrix");
        let w = context.win_size.w as f32;
        let h = context.win_size.h as f32;
        let x = context.mouse().pos.v.x as f32;
        let y = context.mouse().pos.v.y as f32;
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

    fn end_turn(&mut self, context: &mut Context) {
        if self.player_info.info.len() > 1 {
            let next_id = self.core.next_player_id(self.core.player_id());
            let screen = Box::new(EndTurnScreen::new(context, &next_id));
            context.add_command(ScreenCommand::PushScreen(screen));
        }
        self.deselect_unit(context);
        self.core.do_command(Command::EndTurn);
        self.regenerate_fow(context);
    }

    fn regenerate_fow(&mut self, context: &mut Context) {
        let state = &self.player_info.get_mut(self.core.player_id()).game_state;
        let texture = self.meshes.get(self.mesh_ids.fow_mesh_id).texture.clone();
        let new_fow_mesh = generate_fogged_tiles_mesh(context, state, texture);
        self.meshes.set(self.mesh_ids.fow_mesh_id, new_fow_mesh);
    }

    fn hide_selected_unit_meshes(&mut self, context: &mut Context) {
        let scene = &mut self.player_info.get_mut(self.core.player_id()).scene;
        self.selection_manager.deselect(scene);
        self.meshes.set(self.mesh_ids.walkable_mesh_id, empty_mesh(context));
        self.meshes.set(self.mesh_ids.targets_mesh_id, empty_mesh(context));
    }

    fn deselect_unit(&mut self, context: &mut Context) {
        if let Some(label_id) = self.gui.label_unit_info_id.take() {
            self.gui.button_manager.remove_button(label_id);
        }
        self.selected_unit_id = None;
        self.hide_selected_unit_meshes(context);
    }

    fn current_state(&self) -> &PartialState {
        &self.player_info.get(self.core.player_id()).game_state
    }

    fn current_player_info(&self) -> &PlayerInfo {
        self.player_info.get(self.core.player_id())
    }

    fn current_player_info_mut(&mut self) -> &mut PlayerInfo {
        self.player_info.get_mut(self.core.player_id())
    }

    fn can_unload_unit(&self, transporter_id: UnitId, pos: &MapPos) -> Option<ExactPos> {
        let state = self.current_state();
        let transporter = state.unit(&transporter_id);
        let passenger_id = match transporter.passenger_id {
            Some(id) => id,
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
            transporter_id: transporter_id,
            passenger_id: passenger_id,
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
        let mut menu_pos = context.mouse().pos;
        menu_pos.v.y = context.win_size.h - menu_pos.v.y;
        let screen = ContextMenuPopup::new(
            self.current_state(),
            self.core.db(),
            context,
            &menu_pos,
            options,
            self.tx.clone(),
        );
        context.add_command(ScreenCommand::PushPopup(Box::new(screen)));
    }

    fn get_context_menu_popup_options(
        &self,
        pos: &MapPos,
    ) -> context_menu_popup::Options {
        let player_info = self.current_player_info();
        let state = &player_info.game_state;
        let db = self.core.db();
        let mut options = context_menu_popup::Options::new();
        let unit_ids = get_unit_ids_at(db, state, pos);
        if let Some(selected_unit_id) = self.selected_unit_id {
            for unit_id in unit_ids {
                let unit = state.unit(&unit_id);
                if unit.player_id == *self.core.player_id() {
                    if unit_id == selected_unit_id {
                        // TODO: do not show both options if unit has no weapons
                        if unit.reaction_fire_mode == ReactionFireMode::HoldFire {
                            options.enable_reaction_fire = Some(selected_unit_id);
                        } else {
                            options.disable_reaction_fire = Some(selected_unit_id);
                        }
                    } else {
                        options.selects.push(unit_id);
                        let load_command = Command::LoadUnit {
                            transporter_id: selected_unit_id,
                            passenger_id: unit_id,
                        };
                        if check_command(db, state, &load_command).is_ok() {
                            options.loads.push(unit_id);
                        }
                    }
                } else {
                    let attacker = state.unit(&selected_unit_id);
                    let defender = state.unit(&unit_id);
                    let hit_chance = self.core.hit_chance(attacker, defender);
                    let attack_command = Command::AttackUnit {
                        attacker_id: attacker.id,
                        defender_id: defender.id,
                    };
                    if check_command(db, state, &attack_command).is_ok() {
                        options.attacks.push((unit_id, hit_chance));
                    }
                }
            }
            if let Some(pos) = self.can_unload_unit(selected_unit_id, pos) {
                options.unload_pos = Some(pos);
            }
            if let Some(destination) = get_free_exact_pos(
                db, state, &state.unit(&selected_unit_id).type_id, pos,
            ) {
                if let Some(path) = player_info.pathfinder.get_path(&destination) {
                    if check_command(db, state, &Command::Move {
                        unit_id: selected_unit_id,
                        path: path.clone(),
                        mode: MoveMode::Fast,
                    }).is_ok() {
                        options.move_pos = Some(destination.clone());
                    }
                    if check_command(db, state, &Command::Move {
                        unit_id: selected_unit_id,
                        path: path.clone(),
                        mode: MoveMode::Hunt,
                    }).is_ok() {
                        options.hunt_pos = Some(destination);
                    }
                }
            }
        } else {
            for unit_id in unit_ids {
                let unit = state.unit(&unit_id);
                if unit.player_id == *self.core.player_id() {
                    options.selects.push(unit_id);
                }
            }
        }
        options
    }

    fn create_unit(&mut self, context: &Context, type_id: UnitTypeId) {
        if self.event_visualizer.is_some() {
            return;
        }
        let pick_result = self.pick_tile(context);
        if let Some(ref pos) = pick_result {
            if let Some(exact_pos) = get_free_exact_pos(
                self.core.db(),
                self.current_state(),
                &type_id,
                pos,
            ) {
                self.core.do_command(Command::CreateUnit {
                    pos: exact_pos,
                    type_id: type_id,
                });
            } else {
                self.map_text_manager.add_text(&pos, "No free slot for unit");
            }
        }
    }

    // TODO: add ability to select enemy units
    fn select_unit(&mut self, context: &mut Context, unit_id: &UnitId) {
        if self.selected_unit_id.is_some() {
            self.deselect_unit(context);
        }
        self.selected_unit_id = Some(unit_id.clone());
        let mut player_info = self.player_info.get_mut(self.core.player_id());
        let state = &player_info.game_state;
        let pf = &mut player_info.pathfinder;
        pf.fill_map(self.core.db(), state, state.unit(unit_id));
        let new_walkable_mesh = build_walkable_mesh(
            context, pf, state.map(), &state.unit(unit_id).move_points);
        self.meshes.set(self.mesh_ids.walkable_mesh_id, new_walkable_mesh);
        let new_targets_mesh = build_targets_mesh(self.core.db(), context, state, unit_id);
        self.meshes.set(self.mesh_ids.targets_mesh_id, new_targets_mesh);
        let scene = &mut player_info.scene;
        self.selection_manager.create_selection_marker(
            state, scene, unit_id);
        {
            let pos = ScreenPos{v: Vector2{x: 10, y: context.win_size.h - 10}};
            let text = {
                let unit = state.unit(unit_id);
                let unit_type = self.core.db().unit_type(&unit.type_id);
                // TODO: core.rs: print_unit_info
                format!("MP={}/{}, AP={}/{}, RAP={}/{}, C={}, M={}",
                    unit.move_points.n,
                    unit_type.move_points.n,
                    unit.attack_points.n,
                    unit_type.attack_points.n,
                    if let Some(ref rap) = unit.reactive_attack_points { rap.n } else { 0 },
                    unit_type.reactive_attack_points.n,
                    unit.count,
                    unit.morale,
                )
                // TODO: print info about unit type and weapon
            };
            let mut unit_info_button = Button::new_small(context, &text, &pos);
            let mut pos = unit_info_button.pos().clone();
            pos.v.y -= unit_info_button.size().h;
            unit_info_button.set_pos(pos);
            if let Some(label_id) = self.gui.label_unit_info_id.take() {
                self.gui.button_manager.remove_button(label_id);
            }
            self.gui.label_unit_info_id = Some(self.gui.button_manager.add_button(
                unit_info_button));
        }
    }

    fn move_unit(&mut self, pos: &ExactPos, move_mode: &MoveMode) {
        let unit_id = self.selected_unit_id.unwrap();
        let player_info = self.player_info.get_mut(self.core.player_id());
        // TODO: duplicated get_path =\
        let path = player_info.pathfinder.get_path(pos).unwrap();
        self.core.do_command(Command::Move {
            unit_id: unit_id,
            path: path,
            mode: move_mode.clone(),
        });
    }

    fn handle_camera_move(&mut self, context: &Context, pos: &ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let camera_move_speed = geom::HEX_EX_RADIUS * 12.0;
        let per_x_pixel = camera_move_speed / (context.win_size.w as f32);
        let per_y_pixel = camera_move_speed / (context.win_size.h as f32);
        let camera = &mut self.current_player_info_mut().camera;
        camera.move_camera(rad(PI), diff.x as f32 * per_x_pixel);
        camera.move_camera(rad(PI * 1.5), diff.y as f32 * per_y_pixel);
    }

    fn handle_camera_rotate(&mut self, context: &Context, pos: &ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let per_x_pixel = PI / (context.win_size.w as f32);
        // TODO: get max angles from camera
        let per_y_pixel = (PI / 4.0) / (context.win_size.h as f32);
        let camera = &mut self.current_player_info_mut().camera;
        camera.add_horizontal_angle(rad(diff.x as f32 * per_x_pixel));
        camera.add_vertical_angle(rad(diff.y as f32 * per_y_pixel));
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
                self.current_player_info_mut().camera.move_camera(rad(PI * 1.5), s);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.current_player_info_mut().camera.move_camera(rad(PI * 0.5), s);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.current_player_info_mut().camera.move_camera(rad(PI * 0.0), s);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.current_player_info_mut().camera.move_camera(rad(PI * 1.0), s);
            },
            VirtualKeyCode::I => {
                self.print_info(context);
            },
            VirtualKeyCode::U => {
                let type_id = self.core.db().unit_type_id("soldier");
                self.create_unit(context, type_id);
            },
            VirtualKeyCode::T => {
                let type_id = self.core.db().unit_type_id("medium_tank");
                self.create_unit(context, type_id);
            },
            VirtualKeyCode::Subtract | VirtualKeyCode::Key1 => {
                self.current_player_info_mut().camera.change_zoom(1.3);
            },
            VirtualKeyCode::Equals | VirtualKeyCode::Key2 => {
                self.current_player_info_mut().camera.change_zoom(0.7);
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
        if let Some(button_id) = self.gui.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, &button_id);
        } else if let Some(pick_result) = pick_result {
            self.try_create_context_menu_popup(context, &pick_result);
        }
    }

    fn handle_event_button_press(&mut self, context: &mut Context, button_id: &ButtonId) {
        if *button_id == self.gui.button_end_turn_id {
            self.end_turn(context);
        } else if *button_id == self.gui.button_deselect_unit_id {
            self.deselect_unit(context);
        } else if *button_id == self.gui.button_prev_unit_id {
            if let Some(id) = self.selected_unit_id {
                let prev_id = find_prev_player_unit_id(
                    self.current_state(), self.core.player_id(), &id);
                self.select_unit(context, &prev_id);
            }
        } else if *button_id == self.gui.button_next_unit_id {
            if let Some(id) = self.selected_unit_id {
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
        m: Matrix4<f32>,
    ) {
        let tr_mat = Matrix4::from_translation(node.pos.v);
        let rot_mat = Matrix4::from(Matrix3::from_angle_z(node.rot));
        let m = m * tr_mat * rot_mat;
        if let Some(mesh_id) = node.mesh_id {
            context.data.mvp = m.into(); // TODO: use separate model matrix
            context.data.basic_color = node.color;
            context.draw_mesh(&self.meshes.get(mesh_id));
        }
        for node in &node.children {
            self.draw_scene_node(context, node, m);
        }
    }

    fn draw_scene_nodes(&self, context: &mut Context) {
        let m = self.current_player_info().camera.mat();
        for node in self.scene().nodes().values() {
            if !(node.color[3] < 1.0) {
                self.draw_scene_node(context, node, m);
            }
        }
        for layer in self.scene().transparent_node_ids().values() {
            for node_id in layer {
                let node = self.scene().node(node_id);
                self.draw_scene_node(context, node, m);
            }
        }
    }

    fn draw_scene(&mut self, context: &mut Context, dtime: &Time) {
        self.draw_scene_nodes(context);
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let player_info = self.player_info.get_mut(self.core.player_id());
            event_visualizer.draw(&mut player_info.scene, dtime);
        }
    }

    fn draw(&mut self, context: &mut Context, dtime: &Time) {
        context.clear_color = [0.7, 0.7, 0.7, 1.0];
        context.encoder.clear(&context.data.out, context.clear_color);
        self.draw_scene(context, dtime);
        let player_info = self.player_info.get(self.core.player_id());
        self.map_text_manager.draw(context, &player_info.camera, dtime);
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
        self.gui.button_manager.draw(context);
    }

    fn pick_tile(&mut self, context: &Context) -> Option<MapPos> {
        let p = self.pick_world_pos(context);
        let origin = MapPos{v: Vector2 {
            x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as i32,
            y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as i32,
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
        let mut player_info = self.player_info.get_mut(current_player_id);
        let scene = &mut player_info.scene;
        let state = &player_info.game_state;
        match *event {
            CoreEvent::Move{ref unit_id, ref to, ..} => {
                let type_id = state.unit(unit_id).type_id.clone();
                let visual_info = self.unit_type_visual_info.get(&type_id);
                EventMoveVisualizer::new(scene, unit_id, visual_info, to)
            },
            CoreEvent::EndTurn{..} => {
                EventEndTurnVisualizer::new()
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, unit_info.player_id);
                EventCreateUnitVisualizer::new(
                    self.core.db(), scene, unit_info, mesh_id, marker_mesh_id)
            },
            CoreEvent::AttackUnit{ref attack_info} => {
                EventAttackUnitVisualizer::new(
                    state,
                    scene,
                    attack_info,
                    &self.mesh_ids.shell_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::ShowUnit{ref unit_info, ..} => {
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, unit_info.player_id);
                EventShowUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    marker_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::HideUnit{ref unit_id} => {
                EventHideUnitVisualizer::new(
                    scene,
                    state,
                    unit_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::LoadUnit{ref passenger_id, ref to, ..} => {
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
            CoreEvent::UnloadUnit{ref unit_info, ref from, ..} => {
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(&unit_info.type_id);
                let mesh_id = &self.unit_type_visual_info
                    .get(&unit_info.type_id).mesh_id;
                let marker_mesh_id = get_marker_mesh_id(
                    &self.mesh_ids, unit_info.player_id);
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
            CoreEvent::SetReactionFireMode{ref unit_id, ref mode} => {
                EventSetReactionFireModeVisualizer::new(
                    state,
                    unit_id,
                    mode,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::SectorOwnerChanged{sector_id, new_owner_id} => {
                EventSectorOwnerChangedVisualizer::new(
                    scene,
                    state,
                    sector_id,
                    new_owner_id,
                    &mut self.map_text_manager,
                )
            }
            CoreEvent::VictoryPoint{ref pos, count, ..} => {
                EventVictoryPointVisualizer::new(
                    pos,
                    count,
                    &mut self.map_text_manager,
                )
            }
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
            self.hide_selected_unit_meshes(context);
        }
    }

    /// handle case when attacker == selected_unit and it dies from reaction fire
    fn attacker_died_from_reaction_fire(&mut self) {
        // TODO: simplify
        if let Some(CoreEvent::AttackUnit{ref attack_info})
            = self.event
        {
            let mut player_info = self.player_info.get_mut(self.core.player_id());
            let state = &mut player_info.game_state;
            let selected_unit_id = match self.selected_unit_id {
                Some(id) => id,
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

    fn check_game_end(&mut self, context: &mut Context) {
        for (_, score) in self.current_state().score() {
            if score.n >= target_score().n {
                context.add_command(ScreenCommand::PopScreen);
                let screen = Box::new(GameResultsScreen::new(context, self.current_state()));
                context.add_command(ScreenCommand::PushScreen(screen));
            }
        }
    }

    fn update_score_labels(&mut self, context: &mut Context) {
        let pos = self.gui.button_manager.buttons()[&self.gui.label_score_id].pos().clone();
        let label_score = Button::new_small(context, &score_text(self.current_state()), &pos);
        self.gui.button_manager.remove_button(self.gui.label_score_id);
        self.gui.label_score_id = self.gui.button_manager.add_button(label_score);
    }

    fn end_event_visualization(&mut self, context: &mut Context) {
        self.attacker_died_from_reaction_fire();
        {
            let player_info = self.player_info.get_mut(self.core.player_id());
            let scene = &mut player_info.scene;
            let state = &mut player_info.game_state;
            self.event_visualizer.as_mut().unwrap().end(scene, state);
            state.apply_event(self.core.db(), self.event.as_ref().unwrap());
        }
        if let Some(label_id) = self.gui.label_unit_info_id.take() {
            self.gui.button_manager.remove_button(label_id);
        }
        if let Some(CoreEvent::VictoryPoint{..}) = self.event {
            self.update_score_labels(context);
            self.check_game_end(context);
        }
        self.regenerate_fow(context);
        self.event_visualizer = None;
        self.event = None;
        if let Some(event) = self.core.get_event() {
            self.start_event_visualization(context, event);
        } else if let Some(unit_id) = self.selected_unit_id {
            self.select_unit(context, &unit_id);
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
        let selected_unit_id = self.selected_unit_id.unwrap();
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
                    attacker_id: selected_unit_id,
                    defender_id: id,
                });
            },
            context_menu_popup::Command::LoadUnit{passenger_id} => {
                self.core.do_command(Command::LoadUnit {
                    transporter_id: selected_unit_id,
                    passenger_id: passenger_id,
                });
            },
            context_menu_popup::Command::UnloadUnit{pos} => {
                let passenger_id = {
                    let transporter = self.current_state()
                        .unit(&selected_unit_id);
                    transporter.passenger_id.unwrap()
                };
                self.core.do_command(Command::UnloadUnit {
                    transporter_id: selected_unit_id,
                    passenger_id: passenger_id,
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
                for (_, player_info) in &mut self.player_info.info {
                    player_info.camera.regenerate_projection_mat(&context.win_size);
                }
            },
            Event::MouseMoved(x, y) => {
                let pos = ScreenPos{v: Vector2{x: x as i32, y: y as i32}};
                self.handle_event_mouse_move(context, &pos);
            },
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(context, key);
            },
            Event::Touch(glutin::Touch{location: (x, y), phase, ..}) => {
                let pos = ScreenPos{v: Vector2{x: x as i32, y: y as i32}};
                match phase {
                    TouchPhase::Started | TouchPhase::Moved => {
                        self.handle_event_mouse_move(context, &pos);
                    },
                    TouchPhase::Ended => {
                        self.handle_event_mouse_move(context, &pos);
                        self.handle_event_lmb_release(context);
                    },
                    TouchPhase::Cancelled => {
                        unimplemented!();
                    },
                }
            },
            _ => {},
        }
        EventStatus::Handled
    }
}
