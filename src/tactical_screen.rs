use std::sync::mpsc::{channel, Sender, Receiver};
use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use std::iter::IntoIterator;
use std::collections::{HashMap};
use cgmath::{self, Array, Vector2, Vector3, rad};
use glutin::{self, VirtualKeyCode, Event, MouseButton, TouchPhase};
use glutin::ElementState::{Released};
use types::{Size2, Time};
use core::map::{Terrain, Map};
use core::partial_state::{PartialState};
use core::game_state::{GameState, GameStateMut};
use core::pathfinder::{Pathfinder};
use core::{self, CoreEvent, Command, UnitId, PlayerId, MapPos, ExactPos, SlotId};
use core::db::{Db};
use core::unit::{UnitTypeId};
use obj;
use camera::Camera;
use gui::{ButtonManager, Button, ButtonId, is_tap};
use scene::{Scene, NodeId, SceneNode};
use event_visualizer;
use unit_type_visual_info::{UnitTypeVisualInfo, UnitTypeVisualInfoManager};
use selection::{SelectionManager, get_selection_mesh};
use map_text::{MapTextManager};
use context::{Context};
use texture::{load_texture};
use mesh::{Mesh, MeshId};
use fs;
use geom;
use screen::{Screen, ScreenCommand, EventStatus};
use context_menu_popup::{self, ContextMenuPopup};
use end_turn_screen::{EndTurnScreen};
use game_results_screen::{GameResultsScreen};
use types::{ScreenPos, WorldPos};
use gen;
use pick;

const FOW_FADING_TIME: f64 = 0.6;

fn get_initial_camera_pos(map_size: Size2) -> WorldPos {
    let pos = get_max_camera_pos(map_size);
    WorldPos{v: Vector3{x: pos.v.x / 2.0, y: pos.v.y / 2.0, z: 0.0}}
}

fn get_max_camera_pos(map_size: Size2) -> WorldPos {
    let map_pos = MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}};
    let pos = geom::map_pos_to_world_pos(map_pos);
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

// TODO: get from Core
fn target_score() -> core::Score {
    core::Score{n: 5}
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct MeshIdManager {
    big_building_mesh_id: MeshId,
    building_mesh_id: MeshId,
    big_building_mesh_w_id: MeshId,
    building_mesh_w_id: MeshId,
    road_mesh_id: MeshId,
    trees_mesh_id: MeshId,
    shell_mesh_id: MeshId,
    marker_mesh_id: MeshId,
    walkable_mesh_id: MeshId,
    targets_mesh_id: MeshId,
    map_mesh_id: MeshId,
    water_mesh_id: MeshId,
    selection_marker_mesh_id: MeshId,
    smoke_mesh_id: MeshId,
    fow_tile_mesh_id: MeshId,
    sector_mesh_ids: HashMap<core::SectorId, MeshId>,
}

impl MeshIdManager {
    fn new(
        context: &mut Context,
        meshes: &mut MeshManager,
        state: &PartialState,
    ) -> MeshIdManager {
        let smoke_tex = load_texture(context, &fs::load("smoke.png").into_inner());
        let floor_tex = load_texture(context, &fs::load("hex.png").into_inner());
        let chess_grid_tex = load_texture(context, &fs::load("chess_grid.png").into_inner());
        let map_mesh_id = meshes.add(gen::generate_map_mesh(
            context, state, floor_tex.clone()));
        let water_mesh_id = meshes.add(gen::generate_water_mesh(
            context, state, floor_tex.clone()));
        let mut sector_mesh_ids = HashMap::new();
        for (&id, sector) in state.sectors() {
            let mesh_id = meshes.add(gen::generate_sector_mesh(
                context, sector, chess_grid_tex.clone()));
            sector_mesh_ids.insert(id, mesh_id);
        }
        let selection_marker_mesh_id = meshes.add(get_selection_mesh(context));
        let smoke_mesh_id = meshes.add(gen::get_one_tile_mesh(context, smoke_tex));
        let fow_tile_mesh_id = meshes.add(gen::get_one_tile_mesh(context, floor_tex));
        let big_building_mesh_id = meshes.add(
            load_object_mesh(context, "big_building"));
        let building_mesh_id = meshes.add(
            load_object_mesh(context, "building"));
        let big_building_mesh_w_id = meshes.add(
            load_object_mesh(context, "big_building_wire"));
        let building_mesh_w_id = meshes.add(
            load_object_mesh(context, "building_wire"));
        let trees_mesh_id = meshes.add(load_object_mesh(context, "trees"));
        let shell_mesh_id = meshes.add(gen::get_shell_mesh(context));
        let road_mesh_id = meshes.add(gen::get_road_mesh(context));
        let marker_mesh_id = meshes.add(gen::get_marker(context, "white.png"));
        let walkable_mesh_id = meshes.add(gen::empty_mesh(context));
        let targets_mesh_id = meshes.add(gen::empty_mesh(context));
        MeshIdManager {
            big_building_mesh_id: big_building_mesh_id,
            building_mesh_id: building_mesh_id,
            big_building_mesh_w_id: big_building_mesh_w_id,
            building_mesh_w_id: building_mesh_w_id,
            trees_mesh_id: trees_mesh_id,
            road_mesh_id: road_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_mesh_id: marker_mesh_id,
            walkable_mesh_id: walkable_mesh_id,
            targets_mesh_id: targets_mesh_id,
            map_mesh_id: map_mesh_id,
            water_mesh_id: water_mesh_id,
            selection_marker_mesh_id: selection_marker_mesh_id,
            smoke_mesh_id: smoke_mesh_id,
            fow_tile_mesh_id: fow_tile_mesh_id,
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
        ("helicopter", "helicopter", 3.0),
    ] {
        manager.add_info(db.unit_type_id(unit_name), UnitTypeVisualInfo {
            mesh_id: meshes.add(load_object_mesh(context, model_name)),
            move_speed: move_speed,
        });
    }
    manager
}

#[derive(Clone, Debug)]
struct Fow {
    map: Map<Option<NodeId>>,
    vanishing_node_ids: HashMap<NodeId, Time>,
    forthcoming_node_ids: HashMap<NodeId, Time>,
}

impl Fow {
    fn new(map_size: Size2) -> Fow {
        Fow {
            map: Map::new(map_size),
            vanishing_node_ids: HashMap::new(),
            forthcoming_node_ids: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct PlayerInfo {
    game_state: PartialState,
    pathfinder: Pathfinder,
    scene: Scene,
    camera: Camera,
    fow: Fow,
}

#[derive(Clone, Debug)]
struct PlayerInfoManager {
    info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    fn new(context: &Context, map_size: Size2, options: &core::Options) -> PlayerInfoManager {
        let mut m = HashMap::new();
        let mut camera = Camera::new(context.win_size);
        camera.set_max_pos(get_max_camera_pos(map_size));
        camera.set_pos(get_initial_camera_pos(map_size));
        m.insert(PlayerId{id: 0}, PlayerInfo {
            game_state: PartialState::new(map_size, PlayerId{id: 0}),
            pathfinder: Pathfinder::new(map_size),
            scene: Scene::new(),
            camera: camera.clone(),
            fow: Fow::new(map_size),
        });
        if options.game_type == core::GameType::Hotseat {
            m.insert(PlayerId{id: 1}, PlayerInfo {
                game_state: PartialState::new(map_size, PlayerId{id: 1}),
                pathfinder: Pathfinder::new(map_size),
                scene: Scene::new(),
                camera: camera,
                fow: Fow::new(map_size),
            });
        }
        PlayerInfoManager{info: m}
    }

    fn get(&self, player_id: PlayerId) -> &PlayerInfo {
        &self.info[&player_id]
    }

    fn get_mut(&mut self, player_id: PlayerId) -> &mut PlayerInfo {
        match self.info.get_mut(&player_id) {
            Some(i) => i,
            None => panic!("Can`t find player_info for id={}", player_id.id),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gui {
    button_manager: ButtonManager,
    button_end_turn_id: ButtonId,
    button_deselect_unit_id: ButtonId,
    button_next_unit_id: ButtonId,
    button_prev_unit_id: ButtonId,
    button_zoom_in_id: ButtonId,
    button_zoom_out_id: ButtonId,
    label_unit_info_id: Option<ButtonId>,
    label_score_id: ButtonId,
}

impl Gui {
    fn new(context: &mut Context, state: &PartialState) -> Gui {
        let mut button_manager = ButtonManager::new();
        let mut pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let button_end_turn_id = button_manager.add_button(
            Button::new(context, "[end turn]", pos));
        let ystep = (button_manager.buttons()[&button_end_turn_id].size().h as f32 * 1.2) as i32; // TODO
        pos.v.y += ystep;
        let button_deselect_unit_id = button_manager.add_button(
            Button::new(context, "[X]", pos));
        pos.v.x += button_manager.buttons()[&button_deselect_unit_id].size().w;
        let button_prev_unit_id = button_manager.add_button(
            Button::new(context, "[<]", pos));
        pos.v.x += button_manager.buttons()[&button_prev_unit_id].size().w;
        let button_next_unit_id = button_manager.add_button(
            Button::new(context, "[>]", pos));
        pos.v.y += ystep;
        pos.v.x = 10;
        let button_zoom_in_id = button_manager.add_button(
            Button::new(context, "[+]", pos));
        pos.v.x += button_manager.buttons()[&button_prev_unit_id].size().w;
        let button_zoom_out_id = button_manager.add_button(
            Button::new(context, "[-]", pos));
        let label_score_id = {
            let vp_pos = ScreenPos{v: Vector2 {
                x: context.win_size.w - 10,
                y: context.win_size.h - 10,
            }};
            let text = score_text(state);
            let mut label_score = Button::new_small(context, &text, vp_pos);
            let mut pos = label_score.pos();
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
            button_zoom_in_id: button_zoom_in_id,
            button_zoom_out_id: button_zoom_out_id,
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
    for (&sector_id, &sector_mesh_id) in &mesh_ids.sector_mesh_ids {
        scene.add_sector(sector_id, SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.015}}, // TODO
            rot: rad(0.0),
            mesh_id: Some(sector_mesh_id),
            color: [1.0, 1.0, 1.0, 0.5],
            children: Vec::new(),
        });
    }
    for tile_pos in map.get_iter() {
        if *map.tile(tile_pos) == Terrain::Trees {
            let pos = geom::map_pos_to_world_pos(tile_pos);
            let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
            scene.add_node(SceneNode {
                pos: pos,
                rot: rot,
                mesh_id: Some(mesh_ids.trees_mesh_id),
                color: [1.0, 1.0, 1.0, 1.0],
                children: Vec::new(),
            });
        }
    }
    for (&object_id, object) in state.objects() {
        match object.class {
            core::ObjectClass::Building => {
                let pos = geom::exact_pos_to_world_pos(object.pos);
                let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
                scene.add_object(object_id, SceneNode {
                    pos: pos,
                    rot: rot,
                    // TODO: merge with switch_wireframe
                    mesh_id: Some(match object.pos.slot_id {
                        SlotId::Id(_) => mesh_ids.building_mesh_id,
                        SlotId::WholeTile => mesh_ids.big_building_mesh_id,
                        SlotId::TwoTiles(_) | SlotId::Air => unimplemented!(),
                    }),
                    color: [1.0, 1.0, 1.0, 1.0],
                    children: Vec::new(),
                });
            }
            core::ObjectClass::Road => {
                let pos = geom::exact_pos_to_world_pos(object.pos);
                let rot = match object.pos.slot_id {
                    SlotId::TwoTiles(dir) => {
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
            core::ObjectClass::Smoke => unimplemented!(),
        }
    }
    scene
}

pub struct TacticalScreen {
    map_text_manager: MapTextManager,
    gui: Gui,
    player_info: PlayerInfoManager,
    core: core::Core,
    event: Option<CoreEvent>,
    event_visualizer: Option<Box<event_visualizer::EventVisualizer>>,
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
        let core = core::Core::new(core_options);
        let map_size = core.map_size();
        let mut player_info = PlayerInfoManager::new(context, map_size, core_options);
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

    fn end_turn(&mut self, context: &mut Context) {
        if self.player_info.info.len() > 1 {
            let next_id = self.core.next_player_id(self.core.player_id());
            let screen = Box::new(EndTurnScreen::new(context, next_id));
            context.add_command(ScreenCommand::PushScreen(screen));
        }
        self.deselect_unit(context);
        self.core.do_command(Command::EndTurn);
        self.regenerate_fow();
    }

    fn regenerate_fow(&mut self) {
        let player_info = self.player_info.get_mut(self.core.player_id());
        let fow = &mut player_info.fow;
        let state = &player_info.game_state;
        for pos in state.map().get_iter() {
            let is_visible = state.is_tile_visible(pos);
            if is_visible {
                if let Some(node_id) = fow.map.tile_mut(pos).take() {
                    if let Some(time) = fow.forthcoming_node_ids.remove(&node_id) {
                        fow.vanishing_node_ids.insert(
                            node_id, Time{n: FOW_FADING_TIME - time.n});
                    } else {
                        fow.vanishing_node_ids.insert(
                            node_id, Time{n: 0.0});
                    }
                }
            }
            if !is_visible && fow.map.tile(pos).is_none() {
                let mut world_pos = geom::map_pos_to_world_pos(pos);
                world_pos.v.z += 0.02; // TODO: magic
                let node_id = player_info.scene.add_node(SceneNode {
                    pos: world_pos,
                    rot: rad(0.0),
                    mesh_id: Some(self.mesh_ids.fow_tile_mesh_id),
                    color: [0.0, 0.1, 0.0, 0.0],
                    children: Vec::new(),
                });
                *fow.map.tile_mut(pos) = Some(node_id);
                fow.forthcoming_node_ids.insert(node_id, Time{n: 0.0});
            }
        }
    }

    fn update_fow(&mut self, dtime: Time) {
        let max_alpha = 0.4;
        let player_info = self.player_info.get_mut(self.core.player_id());
        let scene = &mut player_info.scene;
        let fow = &mut player_info.fow;
        for (&node_id, time) in &mut fow.forthcoming_node_ids {
            time.n += dtime.n;
            let a = (time.n / FOW_FADING_TIME) * max_alpha;
            scene.node_mut(node_id).color[3] = a as f32;
        }
        fow.forthcoming_node_ids = fow.forthcoming_node_ids.clone()
            .into_iter().filter(|&(_, time)| time.n < FOW_FADING_TIME).collect();
        for (&node_id, time) in &mut fow.vanishing_node_ids {
            time.n += dtime.n;
            let a = (1.0 - time.n / FOW_FADING_TIME) * max_alpha;
            scene.node_mut(node_id).color[3] = a as f32;
        }
        let dead_node_ids: HashMap<NodeId, Time> = fow.vanishing_node_ids.clone()
            .into_iter().filter(|&(_, time)| time.n > FOW_FADING_TIME).collect();
        for &node_id in dead_node_ids.keys() {
            scene.remove_node(node_id);
            fow.vanishing_node_ids.remove(&node_id);
        }
    }

    fn hide_selected_unit_meshes(&mut self, context: &mut Context) {
        let scene = &mut self.player_info.get_mut(self.core.player_id()).scene;
        self.selection_manager.deselect(scene);
        self.meshes.set(self.mesh_ids.walkable_mesh_id, gen::empty_mesh(context));
        self.meshes.set(self.mesh_ids.targets_mesh_id, gen::empty_mesh(context));
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

    fn can_unload_unit(&self, transporter_id: UnitId, pos: MapPos) -> Option<ExactPos> {
        let state = self.current_state();
        let transporter = state.unit(transporter_id);
        let passenger_id = match transporter.passenger_id {
            Some(id) => id,
            None => return None,
        };
        let exact_pos = match core::get_free_exact_pos(
            self.core.db(),
            state,
            state.unit(passenger_id).type_id,
            pos,
        ) {
            Some(pos) => pos,
            None => return None,
        };
        if core::check_command(self.core.db(), state, &Command::UnloadUnit {
            transporter_id: transporter_id,
            passenger_id: passenger_id,
            pos: exact_pos,
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
        pos: MapPos,
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
            menu_pos,
            options,
            self.tx.clone(),
        );
        context.add_command(ScreenCommand::PushPopup(Box::new(screen)));
    }

    fn get_context_menu_popup_options(
        &self,
        pos: MapPos,
    ) -> context_menu_popup::Options {
        let player_info = self.current_player_info();
        let state = &player_info.game_state;
        let db = self.core.db();
        let mut options = context_menu_popup::Options::new();
        let unit_ids = core::get_unit_ids_at(db, state, pos);
        if let Some(selected_unit_id) = self.selected_unit_id {
            for unit_id in unit_ids {
                let unit = state.unit(unit_id);
                let unit_type = self.core.db().unit_type(unit.type_id);
                if unit.player_id == self.core.player_id() {
                    if unit_id == selected_unit_id {
                        if unit_type.attack_points.n != 0
                            || unit_type.reactive_attack_points.n != 0
                        {
                            if unit.reaction_fire_mode == core::ReactionFireMode::HoldFire {
                                options.enable_reaction_fire = Some(selected_unit_id);
                            } else {
                                options.disable_reaction_fire = Some(selected_unit_id);
                            }
                        }
                    } else {
                        options.selects.push(unit_id);
                        let load_command = Command::LoadUnit {
                            transporter_id: selected_unit_id,
                            passenger_id: unit_id,
                        };
                        if core::check_command(db, state, &load_command).is_ok() {
                            options.loads.push(unit_id);
                        }
                    }
                } else {
                    let attacker = state.unit(selected_unit_id);
                    let defender = state.unit(unit_id);
                    let hit_chance = self.core.hit_chance(attacker, defender);
                    let attack_command = Command::AttackUnit {
                        attacker_id: attacker.id,
                        defender_id: defender.id,
                    };
                    if core::check_command(db, state, &attack_command).is_ok() {
                        options.attacks.push((unit_id, hit_chance));
                    }
                }
            }
            if core::check_command(db, state, &Command::Smoke {
                unit_id: selected_unit_id,
                pos: pos,
            }).is_ok() {
                options.smoke_pos = Some(pos);
            }
            if let Some(pos) = self.can_unload_unit(selected_unit_id, pos) {
                options.unload_pos = Some(pos);
            }
            let selected_unit = state.unit(selected_unit_id);
            let selected_unit_type = db.unit_type(selected_unit.type_id);
            if let Some(destination) = core::get_free_exact_pos(
                db, state, state.unit(selected_unit_id).type_id, pos,
            ) {
                if let Some(path) = player_info.pathfinder.get_path(destination) {
                    if core::check_command(db, state, &Command::Move {
                        unit_id: selected_unit_id,
                        path: path.clone(),
                        mode: core::MoveMode::Fast,
                    }).is_ok() {
                        options.move_pos = Some(destination);
                    }
                    let hunt_command = Command::Move {
                        unit_id: selected_unit_id,
                        path: path.clone(),
                        mode: core::MoveMode::Hunt,
                    };
                    if !selected_unit_type.is_air
                        && core::check_command(db, state, &hunt_command).is_ok()
                    {
                        options.hunt_pos = Some(destination);
                    }
                }
            }
        } else {
            for unit_id in unit_ids {
                let unit = state.unit(unit_id);
                if unit.player_id == self.core.player_id() {
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
        if let Some(pos) = pick_result {
            if let Some(exact_pos) = core::get_free_exact_pos(
                self.core.db(),
                self.current_state(),
                type_id,
                pos,
            ) {
                self.core.do_command(Command::CreateUnit {
                    pos: exact_pos,
                    type_id: type_id,
                });
            } else {
                self.map_text_manager.add_text(pos, "No free slot for unit");
            }
        }
    }

    // TODO: add ability to select enemy units
    fn select_unit(&mut self, context: &mut Context, unit_id: UnitId) {
        if self.selected_unit_id.is_some() {
            self.deselect_unit(context);
        }
        self.selected_unit_id = Some(unit_id);
        let mut player_info = self.player_info.get_mut(self.core.player_id());
        let state = &player_info.game_state;
        let pf = &mut player_info.pathfinder;
        pf.fill_map(self.core.db(), state, state.unit(unit_id));
        let new_walkable_mesh = gen::build_walkable_mesh(
            context, pf, state.map(), state.unit(unit_id).move_points);
        self.meshes.set(self.mesh_ids.walkable_mesh_id, new_walkable_mesh);
        let new_targets_mesh = gen::build_targets_mesh(self.core.db(), context, state, unit_id);
        self.meshes.set(self.mesh_ids.targets_mesh_id, new_targets_mesh);
        let scene = &mut player_info.scene;
        self.selection_manager.create_selection_marker(
            state, scene, unit_id);
        {
            let pos = ScreenPos{v: Vector2{x: 10, y: context.win_size.h - 10}};
            let text = {
                let unit = state.unit(unit_id);
                let unit_type = self.core.db().unit_type(unit.type_id);
                // TODO: core.rs: print_unit_info
                format!("MP={}/{}, AP={}/{}, RAP={}/{}, C={}, M={}",
                    unit.move_points.n,
                    unit_type.move_points.n,
                    unit.attack_points.n,
                    unit_type.attack_points.n,
                    if let Some(rap) = unit.reactive_attack_points { rap.n } else { 0 },
                    unit_type.reactive_attack_points.n,
                    unit.count,
                    unit.morale,
                )
                // TODO: print info about unit type and weapon
            };
            let mut unit_info_button = Button::new_small(context, &text, pos);
            let mut pos = unit_info_button.pos();
            pos.v.y -= unit_info_button.size().h;
            unit_info_button.set_pos(pos);
            if let Some(label_id) = self.gui.label_unit_info_id.take() {
                self.gui.button_manager.remove_button(label_id);
            }
            self.gui.label_unit_info_id = Some(self.gui.button_manager.add_button(
                unit_info_button));
        }
    }

    fn move_unit(&mut self, pos: ExactPos, move_mode: core::MoveMode) {
        let unit_id = self.selected_unit_id.unwrap();
        let player_info = self.player_info.get_mut(self.core.player_id());
        // TODO: duplicated get_path =\
        let path = player_info.pathfinder.get_path(pos).unwrap();
        self.core.do_command(Command::Move {
            unit_id: unit_id,
            path: path,
            mode: move_mode,
        });
    }

    fn handle_camera_move(&mut self, context: &Context, pos: ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let camera_move_speed = geom::HEX_EX_RADIUS * 12.0;
        let per_x_pixel = camera_move_speed / (context.win_size.w as f32);
        let per_y_pixel = camera_move_speed / (context.win_size.h as f32);
        let camera = &mut self.current_player_info_mut().camera;
        camera.move_in_direction(rad(PI), diff.x as f32 * per_x_pixel);
        camera.move_in_direction(rad(PI * 1.5), diff.y as f32 * per_y_pixel);
    }

    fn handle_camera_rotate(&mut self, context: &Context, pos: ScreenPos) {
        let diff = pos.v - context.mouse().pos.v;
        let per_x_pixel = PI / (context.win_size.w as f32);
        // TODO: get max angles from camera
        let per_y_pixel = (PI / 4.0) / (context.win_size.h as f32);
        let camera = &mut self.current_player_info_mut().camera;
        camera.add_horizontal_angle(rad(diff.x as f32 * per_x_pixel));
        camera.add_vertical_angle(rad(diff.y as f32 * per_y_pixel));
    }

    fn handle_event_mouse_move(&mut self, context: &Context, pos: ScreenPos) {
        self.handle_event_mouse_move_platform(context, pos);
    }

    #[cfg(not(target_os = "android"))]
    fn handle_event_mouse_move_platform(&mut self, context: &Context, pos: ScreenPos) {
        if context.mouse().is_left_button_pressed {
            self.handle_camera_move(context, pos);
        } else if context.mouse().is_right_button_pressed {
            self.handle_camera_rotate(context, pos);
        }
    }

    #[cfg(target_os = "android")]
    fn handle_event_mouse_move_platform(&mut self, context: &Context, pos: ScreenPos) {
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
        if let Some(pos) = pick_result {
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
                self.current_player_info_mut().camera.move_in_direction(rad(PI * 1.5), s);
            },
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.current_player_info_mut().camera.move_in_direction(rad(PI * 0.5), s);
            },
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.current_player_info_mut().camera.move_in_direction(rad(PI * 0.0), s);
            },
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.current_player_info_mut().camera.move_in_direction(rad(PI * 1.0), s);
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
            self.handle_event_button_press(context, button_id);
        } else if let Some(pick_result) = pick_result {
            self.try_create_context_menu_popup(context, pick_result);
        }
    }

    fn handle_event_button_press(&mut self, context: &mut Context, button_id: ButtonId) {
        if button_id == self.gui.button_end_turn_id {
            self.end_turn(context);
        } else if button_id == self.gui.button_deselect_unit_id {
            self.deselect_unit(context);
        } else if button_id == self.gui.button_prev_unit_id {
            if let Some(id) = self.selected_unit_id {
                let prev_id = core::find_prev_player_unit_id(
                    self.current_state(), self.core.player_id(), id);
                self.select_unit(context, prev_id);
            }
        } else if button_id == self.gui.button_next_unit_id {
            if let Some(id) = self.selected_unit_id {
                let next_id = core::find_next_player_unit_id(
                    self.current_state(), self.core.player_id(), id);
                self.select_unit(context, next_id);
            }
        } else if button_id == self.gui.button_zoom_in_id {
            self.current_player_info_mut().camera.change_zoom(0.7);
        } else if button_id == self.gui.button_zoom_out_id {
            self.current_player_info_mut().camera.change_zoom(1.3);
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
        m: cgmath::Matrix4<f32>,
    ) {
        let tr_mat = cgmath::Matrix4::from_translation(node.pos.v);
        let rot_mat = cgmath::Matrix4::from(cgmath::Matrix3::from_angle_z(node.rot));
        let m = m * tr_mat * rot_mat;
        if let Some(mesh_id) = node.mesh_id {
            context.data.mvp = m.into(); // TODO: use separate model matrix
            context.data.basic_color = node.color;
            context.draw_mesh(self.meshes.get(mesh_id));
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
            for &node_id in layer {
                let node = self.scene().node(node_id);
                self.draw_scene_node(context, node, m);
            }
        }
    }

    fn draw_scene(&mut self, context: &mut Context, dtime: Time) {
        self.draw_scene_nodes(context);
        if let Some(ref mut event_visualizer) = self.event_visualizer {
            let player_info = self.player_info.get_mut(self.core.player_id());
            event_visualizer.draw(&mut player_info.scene, dtime);
        }
    }

    fn draw(&mut self, context: &mut Context, dtime: Time) {
        context.clear_color = [0.7, 0.7, 0.7, 1.0];
        context.encoder.clear(&context.data.out, context.clear_color);
        self.draw_scene(context, dtime);
        let player_info = self.player_info.get(self.core.player_id());
        self.map_text_manager.draw(context, &player_info.camera, dtime);
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
        self.gui.button_manager.draw(context);
    }

    fn pick_tile(&self, context: &Context) -> Option<MapPos> {
        let camera = &self.current_player_info().camera;
        let state = self.current_state();
        pick::pick_tile(context, state, camera)
    }

    fn make_event_visualizer(
        &mut self,
        event: &CoreEvent,
    ) -> Box<event_visualizer::EventVisualizer> {
        let current_player_id = self.core.player_id();
        let mut player_info = self.player_info.get_mut(current_player_id);
        let scene = &mut player_info.scene;
        let state = &player_info.game_state;
        match *event {
            CoreEvent::Move{unit_id, to, ..} => {
                let type_id = state.unit(unit_id).type_id;
                let visual_info = self.unit_type_visual_info.get(type_id);
                event_visualizer::EventMoveVisualizer::new(
                    scene,
                    unit_id,
                    visual_info,
                    to,
                )
            },
            CoreEvent::EndTurn{..} => {
                event_visualizer::EventEndTurnVisualizer::new()
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                let mesh_id = self.unit_type_visual_info
                    .get(unit_info.type_id).mesh_id;
                event_visualizer::EventCreateUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    self.mesh_ids.marker_mesh_id,
                )
            },
            CoreEvent::AttackUnit{ref attack_info} => {
                event_visualizer::EventAttackUnitVisualizer::new(
                    state,
                    scene,
                    attack_info,
                    self.mesh_ids.shell_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::ShowUnit{ref unit_info, ..} => {
                let mesh_id = self.unit_type_visual_info
                    .get(unit_info.type_id).mesh_id;
                event_visualizer::EventShowUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    self.mesh_ids.marker_mesh_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::HideUnit{unit_id} => {
                event_visualizer::EventHideUnitVisualizer::new(
                    scene,
                    state,
                    unit_id,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::LoadUnit{passenger_id, to, ..} => {
                let type_id = state.unit(passenger_id).type_id;
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(type_id);
                event_visualizer::EventLoadUnitVisualizer::new(
                    scene,
                    state,
                    passenger_id,
                    to,
                    unit_type_visual_info,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::UnloadUnit{ref unit_info, from, ..} => {
                let unit_type_visual_info
                    = self.unit_type_visual_info.get(unit_info.type_id);
                let mesh_id = self.unit_type_visual_info
                    .get(unit_info.type_id).mesh_id;
                event_visualizer::EventUnloadUnitVisualizer::new(
                    self.core.db(),
                    scene,
                    unit_info,
                    mesh_id,
                    self.mesh_ids.marker_mesh_id,
                    from,
                    unit_type_visual_info,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::SetReactionFireMode{unit_id, mode} => {
                event_visualizer::EventSetReactionFireModeVisualizer::new(
                    state,
                    unit_id,
                    mode,
                    &mut self.map_text_manager,
                )
            },
            CoreEvent::SectorOwnerChanged{sector_id, new_owner_id} => {
                event_visualizer::EventSectorOwnerChangedVisualizer::new(
                    scene,
                    state,
                    sector_id,
                    new_owner_id,
                    &mut self.map_text_manager,
                )
            }
            CoreEvent::VictoryPoint{pos, count, ..} => {
                event_visualizer::EventVictoryPointVisualizer::new(
                    pos,
                    count,
                    &mut self.map_text_manager,
                )
            }
            CoreEvent::Smoke{pos, unit_id, id} => {
                event_visualizer::EventSmokeVisualizer::new(
                    scene,
                    pos,
                    unit_id,
                    id,
                    self.mesh_ids.smoke_mesh_id,
                    &mut self.map_text_manager,
                )
            }
            CoreEvent::RemoveSmoke{id} => {
                event_visualizer::EventRemoveSmokeVisualizer::new(
                    state,
                    id,
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
        let attack_info = match self.event {
            Some(CoreEvent::AttackUnit{ref attack_info}) => attack_info,
            _ => return,
        };
        let player_info = self.player_info.get(self.core.player_id());
        let state = &player_info.game_state;
        let selected_unit_id = match self.selected_unit_id {
            Some(id) => id,
            None => return,
        };
        let defender = state.unit(attack_info.defender_id);
        if selected_unit_id == attack_info.defender_id
            && defender.count - attack_info.killed <= 0
        {
            self.selected_unit_id = None;
        }
    }

    fn check_game_end(&mut self, context: &mut Context) {
        for score in self.current_state().score().values() {
            if score.n >= target_score().n {
                context.add_command(ScreenCommand::PopScreen);
                let screen = Box::new(GameResultsScreen::new(context, self.current_state()));
                context.add_command(ScreenCommand::PushScreen(screen));
            }
        }
    }

    fn update_score_labels(&mut self, context: &mut Context) {
        let pos = self.gui.button_manager.buttons()[&self.gui.label_score_id].pos();
        let label_score = Button::new_small(context, &score_text(self.current_state()), pos);
        self.gui.button_manager.remove_button(self.gui.label_score_id);
        self.gui.label_score_id = self.gui.button_manager.add_button(label_score);
    }

    // TODO: simplify
    fn switch_wireframe(&mut self) {
        let player_info = self.player_info.get_mut(self.core.player_id());
        let scene = &mut player_info.scene;
        let state = &mut player_info.game_state;
        'object_loop: for (&object_id, object) in state.objects() {
            if object.class != core::ObjectClass::Building {
                continue;
            }
            let is_big = match object.pos.slot_id {
                SlotId::Id(_) => false,
                SlotId::WholeTile => true,
                SlotId::TwoTiles(..) | SlotId::Air => unimplemented!(),
            };
            let normal_mesh_id = if is_big {
                self.mesh_ids.big_building_mesh_id
            } else {
                self.mesh_ids.building_mesh_id
            };
            let wire_mesh_id = if is_big {
                self.mesh_ids.big_building_mesh_w_id
            } else {
                self.mesh_ids.building_mesh_w_id
            };
            let node = {
                let node_ids = scene.object_id_to_node_id(object_id).clone();
                assert_eq!(node_ids.len(), 1);
                let node_id = node_ids.into_iter().next().unwrap();
                scene.node_mut(node_id)
            };
            for unit in state.units().values() {
                if unit.pos == object.pos || (is_big && unit.pos.map_pos == object.pos.map_pos) {
                    node.mesh_id = Some(wire_mesh_id);
                    node.color = [0.0, 0.0, 0.0, 1.0];
                    continue 'object_loop;
                }
            }
            node.mesh_id = Some(normal_mesh_id);
            node.color = [1.0, 1.0, 1.0, 1.0];
        }
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
        self.switch_wireframe();
        if let Some(label_id) = self.gui.label_unit_info_id.take() {
            self.gui.button_manager.remove_button(label_id);
        }
        if let Some(CoreEvent::VictoryPoint{..}) = self.event {
            self.update_score_labels(context);
            self.check_game_end(context);
        }
        self.regenerate_fow();
        self.event_visualizer = None;
        self.event = None;
        if let Some(event) = self.core.get_event() {
            self.start_event_visualization(context, event);
        } else if let Some(unit_id) = self.selected_unit_id {
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
            self.select_unit(context, id);
            return;
        }
        let selected_unit_id = self.selected_unit_id.unwrap();
        match command {
            context_menu_popup::Command::Select{id} => {
                self.select_unit(context, id);
            },
            context_menu_popup::Command::Move{pos} => {
                self.move_unit(pos, core::MoveMode::Fast);
            },
            context_menu_popup::Command::Hunt{pos} => {
                self.move_unit(pos, core::MoveMode::Hunt);
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
                        .unit(selected_unit_id);
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
                    mode: core::ReactionFireMode::Normal,
                });
            },
            context_menu_popup::Command::DisableReactionFire{id} => {
                self.core.do_command(Command::SetReactionFireMode {
                    unit_id: id,
                    mode: core::ReactionFireMode::HoldFire,
                });
            },
            context_menu_popup::Command::Smoke{pos} => {
                self.core.do_command(Command::Smoke {
                    unit_id: selected_unit_id,
                    pos: pos,
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
    fn tick(&mut self, context: &mut Context, dtime: Time) {
        self.logic(context);
        self.draw(context, dtime);
        self.update_fow(dtime);
        self.handle_context_menu_popup_commands(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) -> EventStatus {
        match *event {
            Event::Resized(..) => {
                for (_, player_info) in &mut self.player_info.info {
                    player_info.camera.regenerate_projection_mat(context.win_size);
                }
            },
            Event::MouseMoved(x, y) => {
                let pos = ScreenPos{v: Vector2{x: x as i32, y: y as i32}};
                self.handle_event_mouse_move(context, pos);
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
                        self.handle_event_mouse_move(context, pos);
                    },
                    TouchPhase::Ended => {
                        self.handle_event_mouse_move(context, pos);
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
