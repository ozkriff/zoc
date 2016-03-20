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
use common::types::{Size2, ZInt, ZFloat};
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
use zgl::texture::{Texture};
use zgl::obj;
use zgl::font_stash::{FontStash};
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
use geom;
use screen::{Screen, ScreenCommand, EventStatus};
use context_menu_popup::{self, ContextMenuPopup};
use end_turn_screen::{EndTurnScreen};

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

fn build_walkable_mesh(
    zgl: &Zgl,
    pf: &Pathfinder,
    map: &Map<Terrain>,
    move_points: &MovePoints,
) -> Mesh {
    let mut vertex_data = Vec::new();
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
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_from.v)});
            vertex_data.push(VertexCoord{v: geom::lift(world_pos_to.v)});
        }
    }
    let mut mesh = Mesh::new(zgl, &vertex_data);
    mesh.set_mode(MeshRenderMode::Lines);
    mesh
}

fn build_targets_mesh(db: &Db, zgl: &Zgl, state: &PartialState, unit_id: &UnitId) -> Mesh {
    let mut vertex_data = Vec::new();
    let unit = state.unit(unit_id);
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
        vertex_data.push(VertexCoord{v: geom::lift(world_pos_from.v)});
        vertex_data.push(VertexCoord{v: geom::lift(world_pos_to.v)});
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

fn load_object_mesh(zgl: &Zgl, name: &str) -> Mesh {
    let obj = obj::Model::new(&format!("{}.obj", name));
    let mut mesh = Mesh::new(zgl, &obj.build());
    if obj.is_wire() {
        mesh.set_mode(MeshRenderMode::Lines);
        // TODO: fix ugly color hack
        mesh.add_texture(zgl, Texture::new(zgl, "black.png"), &[]);
    } else {
        let tex = Texture::new(zgl, &format!("{}.png", name));
        mesh.add_texture(zgl, tex, &obj.build_tex_coord());
    }
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
    zgl: &Zgl,
    meshes: &mut Vec<Mesh>,
) -> UnitTypeVisualInfoManager {
    let mut manager = UnitTypeVisualInfoManager::new();
    let mortar_id = db.unit_type_id("mortar");
    let mortar_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "mortar"));
    manager.add_info(&mortar_id, UnitTypeVisualInfo {
        mesh_id: mortar_mesh_id,
        move_speed: 1.5,
    });
    let mammoth_tank_id = db.unit_type_id("mammoth tank");
    let mammoth_tank_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "mammoth"));
    manager.add_info(&mammoth_tank_id, UnitTypeVisualInfo {
        mesh_id: mammoth_tank_mesh_id,
        move_speed: 2.0,
    });
    let tank_id = db.unit_type_id("tank");
    let tank_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "tank"));
    manager.add_info(&tank_id, UnitTypeVisualInfo {
        mesh_id: tank_mesh_id,
        move_speed: 3.8,
    });
    let truck_id = db.unit_type_id("truck");
    let truck_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "truck"));
    manager.add_info(&truck_id, UnitTypeVisualInfo {
        mesh_id: truck_mesh_id,
        move_speed: 4.8,
    });
    let soldier_id = db.unit_type_id("soldier");
    let soldier_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "soldier"));
    manager.add_info(&soldier_id, UnitTypeVisualInfo {
        mesh_id: soldier_mesh_id,
        move_speed: 2.0,
    });
    let scout_id = db.unit_type_id("scout");
    let scout_mesh_id = add_mesh(meshes, load_object_mesh(zgl, "scout"));
    manager.add_info(&scout_id, UnitTypeVisualInfo {
        mesh_id: scout_mesh_id,
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
        let floor_tex = Texture::new(&context.zgl, "floor.png"); // TODO: !!!
        let mut meshes = Vec::new();
        let visible_map_mesh = generate_visible_tiles_mesh(
            &context.zgl, &player_info.get(core.player_id()).game_state, &floor_tex);
        let fow_map_mesh = generate_fogged_tiles_mesh(
            &context.zgl, &player_info.get(core.player_id()).game_state, &floor_tex);
        let big_building_mesh_w_id = add_mesh(
            &mut meshes, load_object_mesh(&context.zgl, "big_building_wire"));
        let building_mesh_w_id = add_mesh(
            &mut meshes, load_object_mesh(&context.zgl, "building_wire"));
        let trees_mesh_id = add_mesh(
            &mut meshes, load_object_mesh(&context.zgl, "trees"));
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
            big_building_mesh_w_id: big_building_mesh_w_id,
            building_mesh_w_id: building_mesh_w_id,
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
    fn select_unit(&mut self, context: &Context, unit_id: &UnitId) {
        self.selected_unit_id = Some(unit_id.clone());
        let mut i = self.player_info.get_mut(self.core.player_id());
        let state = &i.game_state;
        let pf = &mut i.pathfinder;
        pf.fill_map(self.core.db(), state, state.unit(unit_id));
        self.walkable_mesh = Some(build_walkable_mesh(
            &context.zgl, pf, state.map(), &state.unit(unit_id).move_points));
        self.targets_mesh = Some(build_targets_mesh(
            self.core.db(), &context.zgl, state, unit_id));
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
        for (_, node) in self.scene().nodes() {
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
        if let Some(ref targets_mesh) = self.targets_mesh {
            context.set_basic_color(&zgl::RED);
            targets_mesh.draw(&context.zgl, &context.shader);
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

    fn pick_tile(&mut self, context: &Context) -> Option<MapPos> {
        let p = self.pick_world_pos(context);
        let origin = MapPos{v: Vector2 {
            x: (p.v.x / (geom::HEX_IN_RADIUS * 2.0)) as ZInt,
            y: (p.v.y / (geom::HEX_EX_RADIUS * 1.5)) as ZInt,
        }};
        let origin_world_pos = geom::map_pos_to_world_pos(&origin);
        let mut closest_map_pos = origin.clone();
        let mut min_dist = (origin_world_pos.v - p.v).length();
        for map_pos in spiral_iter(&origin, 1) {
            let pos = geom::map_pos_to_world_pos(&map_pos);
            let d = (pos.v - p.v).length();
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

    fn end_event_visualization(&mut self, context: &Context) {
        self.attacker_died_from_reaction_fire();
        {
            let i = self.player_info.get_mut(self.core.player_id());
            let scene = &mut i.scene;
            let state = &mut i.game_state;
            self.event_visualizer.as_mut().unwrap().end(scene, state);
            state.apply_event(self.core.db(), self.event.as_ref().unwrap());
            self.visible_map_mesh = generate_visible_tiles_mesh(
                &context.zgl, state, &self.floor_tex);
            self.fow_map_mesh = generate_fogged_tiles_mesh(
                &context.zgl, state, &self.floor_tex);
        }
        self.event_visualizer = None;
        self.event = None;
        if let Some(event) = self.core.get_event() {
            self.start_event_visualization(context, event);
        } else if let Some(ref unit_id) = self.selected_unit_id.clone() {
            self.select_unit(context, unit_id);
        }
    }

    fn logic(&mut self, context: &Context) {
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
        context: &Context,
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
