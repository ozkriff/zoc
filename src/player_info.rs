use std::collections::{HashMap};
use std::default::{Default};
use std::rc::{Rc};
use cgmath::{Vector2, Vector3};
use core::game_state::{State};
use core::movement::{Pathfinder};
use core::map::{Map};
use core::db::{Db};
use core::player::{PlayerId};
use core::options::{Options, GameType};
use core::position::{MapPos};
use context::{Context};
use types::{Size2, WorldPos};
use scene::{Scene, NodeId, SceneNode, SceneNodeType};
use mesh_manager::{MeshIdManager};
use geom;
use camera::Camera;

fn get_initial_camera_pos(map_size: Size2) -> WorldPos {
    let pos = get_max_camera_pos(map_size);
    WorldPos{v: Vector3{x: pos.v.x / 2.0, y: pos.v.y / 2.0, z: 0.0}}
}

fn get_max_camera_pos(map_size: Size2) -> WorldPos {
    let map_pos = MapPos{v: Vector2{x: map_size.w, y: map_size.h - 1}};
    let pos = geom::map_pos_to_world_pos(map_pos);
    WorldPos{v: Vector3{x: -pos.v.x, y: -pos.v.y, z: 0.0}}
}

#[derive(Clone, Debug)]
pub struct FowInfoTile {
    pub is_visible: bool,
    pub node_id: NodeId,
}

fn make_fow_info_map(
    map_size: Size2,
    scene: &mut Scene,
    mesh_ids: &MeshIdManager,
) -> Map<FowInfoTile> {
    Map::from_callback(map_size, &mut |map_pos| {
        let node_id = scene.allocate_node_id();
        let mut pos = geom::map_pos_to_world_pos(map_pos);
        pos.v.z += 0.02; // TODO: magic
        let node = SceneNode {
            mesh_id: Some(mesh_ids.fow_tile_mesh_id),

            // TODO: prepare separate texture for FoW tiles!
            // mesh_id: Some(mesh_ids.shadow_mesh_id),

            color: [0.0, 0.0, 0.0, 0.4], // TODO: use constant
            node_type: SceneNodeType::Transparent,
            // scale: 1.8,
            pos,
            ..Default::default()
        };
        scene.set_node(node_id, node);
        FowInfoTile {
            is_visible: false,
            node_id,
        }
    })
}

#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub game_state: State,
    pub pathfinder: Pathfinder,
    pub scene: Scene,
    pub camera: Camera,
    pub fow_map: Map<FowInfoTile>,
}

#[derive(Clone, Debug)]
pub struct PlayerInfoManager {
    pub info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    pub fn new(
        db: Rc<Db>,
        context: &Context,
        options: &Options,
        mesh_ids: &MeshIdManager,
    ) -> PlayerInfoManager {
        let state = State::new_partial(db.clone(), options, PlayerId{id: 0});
        let map_size = state.map().size();
        let mut m = HashMap::new();
        let mut camera = Camera::new(context.win_size());
        camera.set_max_pos(get_max_camera_pos(map_size));
        // TODO: different camera's starting positions
        camera.set_pos(get_initial_camera_pos(map_size));
        {
            let mut scene = Scene::new();
            let fow_map = make_fow_info_map(map_size, &mut scene, mesh_ids);
            m.insert(PlayerId{id: 0}, PlayerInfo {
                game_state: state,
                pathfinder: Pathfinder::new(db.clone(), map_size),
                scene,
                camera: camera.clone(),
                fow_map,
            });
        }
        if options.game_type == GameType::Hotseat {
            let state2 = State::new_partial(db.clone(), options, PlayerId{id: 1});
            let mut scene = Scene::new();
            // TODO: extract function!
            let fow_map = make_fow_info_map(map_size, &mut scene, mesh_ids);
            m.insert(PlayerId{id: 1}, PlayerInfo {
                game_state: state2,
                pathfinder: Pathfinder::new(db, map_size),
                scene,
                camera,
                fow_map,
            });
        }
        PlayerInfoManager{info: m}
    }

    pub fn get(&self, player_id: PlayerId) -> &PlayerInfo {
        &self.info[&player_id]
    }

    pub fn get_mut(&mut self, player_id: PlayerId) -> &mut PlayerInfo {
        match self.info.get_mut(&player_id) {
            Some(i) => i,
            None => panic!("Can`t find player_info for id={}", player_id.id),
        }
    }
}
