use std::collections::{HashMap};
use std::rc::{Rc};
use cgmath::{Vector2, Vector3};
use core::game_state::{State};
use core::pathfinder::{Pathfinder};
use core::map::{Map};
use core::db::{Db};
use core::{self, PlayerId, MapPos};
use context::{Context};
use types::{Size2, Time, WorldPos};
use scene::{Scene, NodeId};
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
pub struct FowInfo {
    pub map: Map<Option<NodeId>>,
    pub vanishing_node_ids: HashMap<NodeId, Time>,
    pub forthcoming_node_ids: HashMap<NodeId, Time>,
}

impl FowInfo {
    pub fn new(map_size: Size2) -> FowInfo {
        FowInfo {
            map: Map::new(map_size),
            vanishing_node_ids: HashMap::new(),
            forthcoming_node_ids: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub game_state: State,
    pub pathfinder: Pathfinder,
    pub scene: Scene,
    pub camera: Camera,
    pub fow_info: FowInfo,
}

#[derive(Clone, Debug)]
pub struct PlayerInfoManager {
    pub info: HashMap<PlayerId, PlayerInfo>,
}

impl PlayerInfoManager {
    pub fn new(db: Rc<Db>, context: &Context, options: &core::Options) -> PlayerInfoManager {
        let state = State::new_partial(db.clone(), options, PlayerId{id: 0});
        let map_size = state.map().size();
        let mut m = HashMap::new();
        let mut camera = Camera::new(context.win_size());
        camera.set_max_pos(get_max_camera_pos(map_size));
        camera.set_pos(get_initial_camera_pos(map_size));
        m.insert(PlayerId{id: 0}, PlayerInfo {
            game_state: state,
            pathfinder: Pathfinder::new(db.clone(), map_size),
            scene: Scene::new(),
            camera: camera.clone(),
            fow_info: FowInfo::new(map_size),
        });
        if options.game_type == core::GameType::Hotseat {
            let state2 = State::new_partial(db.clone(), options, PlayerId{id: 1});
            m.insert(PlayerId{id: 1}, PlayerInfo {
                game_state: state2,
                pathfinder: Pathfinder::new(db, map_size),
                scene: Scene::new(),
                camera: camera,
                fow_info: FowInfo::new(map_size),
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
