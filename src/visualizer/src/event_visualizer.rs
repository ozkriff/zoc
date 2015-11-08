// See LICENSE file for copyright and license details.

use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, Vector, rad};
use common::types::{ZFloat, UnitId, ZInt, MapPos};
use core::game_state::GameState;
use core::state::{State};
use core::{self, UnitInfo, AttackInfo, ReactionFireMode};
use core::unit::{UnitTypeId};
use core::pathfinder::{MapPath};
use core::db::{Db};
use zgl::mesh::{MeshId};
use zgl::types::{Time, WorldPos};
use geom;
use scene::{
    Scene,
    SceneNode,
    NodeId,
    MIN_MARKER_NODE_ID,
    SHELL_NODE_ID,
};
use unit_type_visual_info::{UnitTypeVisualInfo};
use move_helper::{MoveHelper};
use map_text::{MapTextManager};

fn unit_id_to_node_id(unit_id: &UnitId) -> NodeId {
    NodeId{id: unit_id.id}
}

fn marker_id(unit_id: &UnitId) -> NodeId {
    NodeId{id: MIN_MARKER_NODE_ID.id + unit_id.id}
}

pub trait EventVisualizer {
    fn is_finished(&self) -> bool;
    fn draw(&mut self, scene: &mut Scene, dtime: &Time);
    fn end(&mut self, scene: &mut Scene, state: &GameState);
}

// TODO: store CoreEvent
pub struct EventMoveVisualizer {
    unit_id: UnitId,
    path: Vec<WorldPos>,
    move_helper: MoveHelper,
    speed: ZFloat,
}

impl EventVisualizer for EventMoveVisualizer {
    fn is_finished(&self) -> bool {
        self.path.len() == 1
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let pos = self.move_helper.step(dtime);
        {
            let marker_node = scene.node_mut(&marker_id(&self.unit_id));
            marker_node.pos.v = pos.v.add_v(&vec3_z(geom::HEX_EX_RADIUS / 2.0));
        }
        let node_id = unit_id_to_node_id(&self.unit_id);
        let node = scene.node_mut(&node_id);
        node.pos = pos;
        if self.move_helper.is_finished() {
            let _ = self.path.remove(0);
            if self.path.len() > 1 {
                self.update_waypoint(node);
            }
            node.pos = self.current_waypoint().clone();
        }
    }

    fn end(&mut self, scene: &mut Scene, _: &GameState) {
        assert!(self.path.len() == 1);
        let node_id = unit_id_to_node_id(&self.unit_id);
        let node = scene.node_mut(&node_id);
        node.pos = self.current_waypoint().clone();
    }
}

impl EventMoveVisualizer {
    pub fn new(
        scene: &mut Scene,
        unit_id: UnitId,
        unit_type_visual_info: &UnitTypeVisualInfo,
        path: MapPath,
    ) -> Box<EventVisualizer> {
        let mut world_path = Vec::new();
        for path_node in path.nodes() {
            let world_pos = geom::map_pos_to_world_pos(&path_node.pos);
            world_path.push(world_pos);
        }
        let speed = unit_type_visual_info.move_speed;
        let node_id = unit_id_to_node_id(&unit_id);
        let node = scene.node_mut(&node_id);
        node.rot = geom::get_rot_angle(
            &world_path[0], &world_path[1]);
        let move_helper = MoveHelper::new(
            &world_path[0], &world_path[1], speed);
        let mut vis = Box::new(EventMoveVisualizer {
            unit_id: unit_id.clone(),
            path: world_path,
            move_helper: move_helper,
            speed: speed,
        });
        vis.update_waypoint(node);
        vis
    }

    fn update_waypoint(&mut self, node: &mut SceneNode) {
        self.move_helper = MoveHelper::new(
            self.current_waypoint(),
            self.next_waypoint(),
            self.speed,
        );
        node.rot = geom::get_rot_angle(
            self.current_waypoint(),
            self.next_waypoint()
        );
    }

    fn current_waypoint(&self) -> &WorldPos {
        assert!(self.path.len() >= 1);
        &self.path[0]
    }

    fn next_waypoint(&self) -> &WorldPos {
        assert!(self.path.len() >= 2);
        &self.path[1]
    }
}

pub struct EventEndTurnVisualizer;

impl EventEndTurnVisualizer {
    pub fn new() -> Box<EventVisualizer> {
        Box::new(EventEndTurnVisualizer)
    }
}

impl EventVisualizer for EventEndTurnVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: &Time) {}

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}

fn show_unit_at(
    db: &Db,
    scene: &mut Scene,
    unit_info: &UnitInfo,
    mesh_id: &MeshId,
    marker_mesh_id: &MeshId,
) {
    let node_id = unit_id_to_node_id(&unit_info.unit_id);
    let world_pos = geom::map_pos_to_world_pos(&unit_info.pos);
    let to = world_pos;
    let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
    scene.nodes.insert(node_id, SceneNode {
        pos: to.clone(),
        rot: rot,
        mesh_id: None,
        children: get_unit_scene_nodes(db, &unit_info.type_id, mesh_id),
    });
    scene.nodes.insert(marker_id(&unit_info.unit_id), SceneNode {
        pos: WorldPos{v: to.v.add_v(&vec3_z(geom::HEX_EX_RADIUS / 2.0))},
        rot: rad(0.0),
        mesh_id: Some(marker_mesh_id.clone()),
        children: Vec::new(),
    });
}

pub struct EventCreateUnitVisualizer {
    id: UnitId,
    move_helper: MoveHelper,
}

fn get_unit_scene_nodes(
    db: &Db,
    type_id: &UnitTypeId,
    mesh_id: &MeshId,
) -> Vec<SceneNode> {
    let count = db.unit_type(type_id).count;
    let mut vec = Vec::new();
    if count == 1 {
        vec![SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}},
            rot: rad(0.0),
            mesh_id: Some(mesh_id.clone()),
            children: vec![],
        }]
    } else {
        for i in 0 .. count {
            let pos = geom::index_to_circle_vertex(count, i).v.mul_s(0.3);
            vec.push(SceneNode {
                pos: WorldPos{v: pos},
                rot: rad(0.0),
                mesh_id: Some(mesh_id.clone()),
                children: vec![],
            });
        }
        vec
    }
}

impl EventCreateUnitVisualizer {
    pub fn new(
        db: &Db,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: &MeshId,
        marker_mesh_id: &MeshId,
    ) -> Box<EventVisualizer> {
        let node_id = unit_id_to_node_id(&unit_info.unit_id);
        let to = geom::map_pos_to_world_pos(&unit_info.pos);
        let from = WorldPos{v: to.v.sub_v(&vec3_z(geom::HEX_EX_RADIUS / 2.0))};
        show_unit_at(db, scene, unit_info, mesh_id, marker_mesh_id);
        let move_helper = MoveHelper::new(&from, &to, 1.0);
        let new_node = scene.node_mut(&node_id);
        new_node.pos = from.clone();
        Box::new(EventCreateUnitVisualizer {
            id: unit_info.unit_id.clone(),
            move_helper: move_helper,
        })
    }
}

impl EventVisualizer for EventCreateUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node_id = unit_id_to_node_id(&self.id);
        let node = scene.node_mut(&node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}

fn vec3_z(z: ZFloat) -> Vector3<ZFloat> {
    Vector3{x: 0.0, y: 0.0, z: z}
}

pub struct EventAttackUnitVisualizer {
    defender_id: UnitId,
    killed: ZInt,
    is_target_destroyed: bool,
    move_helper: MoveHelper,
    shell_move: Option<MoveHelper>,
}

impl EventAttackUnitVisualizer {
    pub fn new(
        state: &GameState,
        scene: &mut Scene,
        attack_info: &AttackInfo,
        shell_mesh_id: &MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let defender = state.unit(&attack_info.defender_id);
        let defender_node_id = unit_id_to_node_id(&attack_info.defender_id);
        let defender_pos = scene.nodes.get(&defender_node_id)
            .expect("Can not find defender")
            .pos.clone();
        let from = defender_pos.clone();
        let to = WorldPos{v: from.v.sub_v(&vec3_z(geom::HEX_EX_RADIUS / 2.0))};
        let move_helper = MoveHelper::new(&from, &to, 1.0);
        let shell_move = if let Some(ref attacker_id) = attack_info.attacker_id {
            let attacker_pos = scene.nodes.get(&unit_id_to_node_id(&attacker_id))
                .expect("Can not find attacker")
                .pos.clone();
            let attacker_map_pos = state.unit(&attacker_id).pos.clone();
            if let core::FireMode::Reactive = attack_info.mode {
                map_text.add_text(&attacker_map_pos, "reaction fire");
            }
            let shell_move = {
                scene.nodes.insert(SHELL_NODE_ID, SceneNode {
                    pos: from.clone(),
                    rot: rad(0.0),
                    mesh_id: Some(shell_mesh_id.clone()),
                    children: Vec::new(),
                });
                MoveHelper::new(&attacker_pos, &defender_pos, 10.0)
            };
            Some(shell_move)
        } else {
            None
        };
        if attack_info.is_ambush {
            map_text.add_text(&defender.pos, "Ambushed");
        };
        let is_target_destroyed = defender.count - attack_info.killed <= 0;
        if attack_info.killed > 0 {
            map_text.add_text(&defender.pos, &format!("-{}", attack_info.killed));
        } else {
            map_text.add_text(&defender.pos, "miss");
        }
        let is_target_suppressed = defender.morale < 50
            && defender.morale + attack_info.suppression >= 50;
        if !is_target_destroyed {
            map_text.add_text(
                &defender.pos,
                &format!("morale: -{}", attack_info.suppression),
            );
            if is_target_suppressed {
                map_text.add_text(&defender.pos, "suppressed");
            }
        }
        Box::new(EventAttackUnitVisualizer {
            defender_id: attack_info.defender_id.clone(),
            killed: attack_info.killed.clone(),
            is_target_destroyed: is_target_destroyed,
            move_helper: move_helper,
            shell_move: shell_move,
        })
    }
}

impl EventVisualizer for EventAttackUnitVisualizer {
    fn is_finished(&self) -> bool {
        if self.killed > 0 {
            self.move_helper.is_finished()
        } else {
            if let Some(ref shell_move) = self.shell_move {
                shell_move.is_finished()
            } else {
                true
            }
        }
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        if let Some(ref mut shell_move) = self.shell_move {
            scene.node_mut(&SHELL_NODE_ID).pos = shell_move.step(dtime);
        }
        let node_id = unit_id_to_node_id(&self.defender_id);
        let is_shell_ok = if let Some(ref shell_move) = self.shell_move {
            shell_move.is_finished()
        } else {
            true
        };
        if is_shell_ok && self.killed > 0 {
            let step = self.move_helper.step_diff(dtime);
            let children = &mut scene.node_mut(&node_id).children;
            for i in 0 .. self.killed as usize {
                let child = children.get_mut(i)
                    .expect("draw: no child");
                child.pos.v.add_self_v(&step);
            }
        }
    }

    fn end(&mut self, scene: &mut Scene, _: &GameState) {
        let node_id = unit_id_to_node_id(&self.defender_id);
        if self.killed > 0 {
            let children = &mut scene.node_mut(&node_id).children;
            assert!(self.killed as usize <= children.len());
            for _ in 0 .. self.killed {
                let _ = children.remove(0);
            }
        }
        if self.is_target_destroyed {
            scene.nodes.remove(&node_id);
            scene.nodes.remove(&marker_id(&self.defender_id));
        }
        scene.nodes.remove(&SHELL_NODE_ID);
    }
}

pub struct EventShowUnitVisualizer;

impl EventShowUnitVisualizer {
    pub fn new(
        db: &Db,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: &MeshId,
        marker_mesh_id: &MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        map_text.add_text(&unit_info.pos, "spotted");
        show_unit_at(db, scene, unit_info, mesh_id, marker_mesh_id);
        Box::new(EventShowUnitVisualizer)
    }
}

impl EventVisualizer for EventShowUnitVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: &Time) {}

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}

pub struct EventHideUnitVisualizer;

impl EventHideUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &GameState,
        unit_id: &UnitId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let pos = state.unit(unit_id).pos.clone();
        map_text.add_text(&pos, "lost");
        let unit_node_id = unit_id_to_node_id(&unit_id);
        scene.nodes.remove(&unit_node_id);
        scene.nodes.remove(&marker_id(&unit_id));
        Box::new(EventHideUnitVisualizer)
    }
}

impl EventVisualizer for EventHideUnitVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: &Time) {}

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}

pub struct EventUnloadUnitVisualizer {
    id: UnitId,
    move_helper: MoveHelper,
}

impl EventUnloadUnitVisualizer {
    pub fn new(
        db: &Db,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: &MeshId,
        marker_mesh_id: &MeshId,
        transporter_pos: &MapPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        map_text.add_text(&unit_info.pos, "unloaded");
        let node_id = unit_id_to_node_id(&unit_info.unit_id);
        let to = geom::map_pos_to_world_pos(&unit_info.pos);
        let from = geom::map_pos_to_world_pos(transporter_pos);
        show_unit_at(db, scene, unit_info, mesh_id, marker_mesh_id);
        let unit_node = scene.node_mut(&node_id);
        unit_node.pos = from.clone();
        unit_node.rot = geom::get_rot_angle(&from, &to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventUnloadUnitVisualizer {
            id: unit_info.unit_id.clone(),
            move_helper: MoveHelper::new(&from, &to, move_speed),
        })
    }
}

impl EventVisualizer for EventUnloadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node_id = unit_id_to_node_id(&self.id);
        let node = scene.node_mut(&node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}

pub struct EventLoadUnitVisualizer {
    passanger_id: UnitId,
    move_helper: MoveHelper,
}

impl EventLoadUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &GameState,
        unit_id: &UnitId,
        transporter_pos: &MapPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let unit_pos = &state.unit(unit_id).pos;
        map_text.add_text(unit_pos, "loaded");
        let from = geom::map_pos_to_world_pos(unit_pos);
        let to = geom::map_pos_to_world_pos(transporter_pos);
        let node_id = unit_id_to_node_id(unit_id);
        let unit_node = scene.node_mut(&node_id);
        unit_node.rot = geom::get_rot_angle(&from, &to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventLoadUnitVisualizer {
            passanger_id: unit_id.clone(),
            move_helper: MoveHelper::new(&from, &to, move_speed),
        })
    }
}

impl EventVisualizer for EventLoadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node_id = unit_id_to_node_id(&self.passanger_id);
        let node = scene.node_mut(&node_id);
        node.pos = self.move_helper.step(dtime);
    }


    fn end(&mut self, scene: &mut Scene, _: &GameState) {
        scene.nodes.remove(&unit_id_to_node_id(&self.passanger_id));
        scene.nodes.remove(&marker_id(&self.passanger_id));
    }
}

pub struct EventSetReactionFireModeVisualizer;

impl EventSetReactionFireModeVisualizer {
    pub fn new(
        state: &GameState,
        unit_id: &UnitId,
        mode: &ReactionFireMode,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let unit_pos = &state.unit(unit_id).pos;
        match *mode {
            ReactionFireMode::Normal => {
                map_text.add_text(unit_pos, "Normal fire mode");
            },
            ReactionFireMode::HoldFire => {
                map_text.add_text(unit_pos, "Hold fire");
            },
        }
        Box::new(EventSetReactionFireModeVisualizer)
    }
}

impl EventVisualizer for EventSetReactionFireModeVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: &Time) {}

    fn end(&mut self, _: &mut Scene, _: &GameState) {}
}


// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
