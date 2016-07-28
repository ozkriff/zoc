// See LICENSE file for copyright and license details.

use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, rad};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use core::{self, UnitInfo, AttackInfo, ReactionFireMode, UnitId, ExactPos};
use core::unit::{UnitTypeId};
use core::db::{Db};
use types::{ZFloat, ZInt, WorldPos, Time};
use mesh::{MeshId};
use geom;
use scene::{Scene, SceneNode, NodeId};
use unit_type_visual_info::{UnitTypeVisualInfo};
use move_helper::{MoveHelper};
use map_text::{MapTextManager};

pub trait EventVisualizer {
    fn is_finished(&self) -> bool;
    fn draw(&mut self, scene: &mut Scene, dtime: &Time);
    fn end(&mut self, scene: &mut Scene, state: &PartialState);
}

pub struct EventMoveVisualizer {
    node_id: NodeId,
    move_helper: MoveHelper,
}

impl EventVisualizer for EventMoveVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let pos = self.move_helper.step(dtime);
        scene.node_mut(&self.node_id).pos = pos;
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        let node = scene.node_mut(&self.node_id);
        node.pos = self.move_helper.destination().clone();
    }
}

impl EventMoveVisualizer {
    pub fn new(
        scene: &mut Scene,
        unit_id: &UnitId,
        unit_type_visual_info: &UnitTypeVisualInfo,
        destination: &ExactPos,
    ) -> Box<EventVisualizer> {
        let speed = unit_type_visual_info.move_speed;
        let node_id = scene.unit_id_to_node_id(unit_id);
        let node = scene.node_mut(&node_id);
        let from = node.pos.clone();
        let to = geom::exact_pos_to_world_pos(destination);
        node.rot = geom::get_rot_angle(&from, &to);
        let move_helper = MoveHelper::new(&from, &to, speed);
        Box::new(EventMoveVisualizer {
            node_id: node_id,
            move_helper: move_helper,
        })
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

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

fn show_unit_at(
    db: &Db,
    scene: &mut Scene,
    unit_info: &UnitInfo,
    mesh_id: &MeshId,
    marker_mesh_id: &MeshId,
) {
    let world_pos = geom::exact_pos_to_world_pos(&unit_info.pos);
    let to = world_pos;
    let rot = rad(thread_rng().gen_range(0.0, PI * 2.0));
    let mut children = get_unit_scene_nodes(db, &unit_info.type_id, mesh_id);
    children.push(SceneNode {
        pos: WorldPos{v: vec3_z(geom::HEX_EX_RADIUS / 2.0)},
        rot: rad(0.0),
        mesh_id: Some(marker_mesh_id.clone()),
        children: Vec::new(),
    });
    scene.add_unit(&unit_info.unit_id, SceneNode {
        pos: to.clone(),
        rot: rot,
        mesh_id: None,
        children: children,
    });
}

pub struct EventCreateUnitVisualizer {
    node_id: NodeId,
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
            let pos = geom::index_to_circle_vertex(count, i).v * 0.15;
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
        let to = geom::exact_pos_to_world_pos(&unit_info.pos);
        let from = WorldPos{v: to.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
        show_unit_at(db, scene, unit_info, mesh_id, marker_mesh_id);
        let move_helper = MoveHelper::new(&from, &to, 2.0);
        let node_id = scene.unit_id_to_node_id(&unit_info.unit_id);
        let new_node = scene.node_mut(&node_id);
        new_node.pos = from.clone();
        Box::new(EventCreateUnitVisualizer {
            node_id: node_id,
            move_helper: move_helper,
        })
    }
}

impl EventVisualizer for EventCreateUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node = scene.node_mut(&self.node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

fn vec3_z(z: ZFloat) -> Vector3<ZFloat> {
    Vector3{x: 0.0, y: 0.0, z: z}
}

pub struct EventAttackUnitVisualizer {
    defender_node_id: NodeId,
    killed: ZInt,
    is_target_destroyed: bool,
    move_helper: MoveHelper,
    shell_move: Option<MoveHelper>,
    shell_node_id: Option<NodeId>,
    is_inderect: bool,
}

impl EventAttackUnitVisualizer {
    pub fn new(
        state: &PartialState,
        scene: &mut Scene,
        attack_info: &AttackInfo,
        shell_mesh_id: &MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let defender = state.unit(&attack_info.defender_id);
        let defender_node_id = scene.unit_id_to_node_id(&attack_info.defender_id);
        let defender_pos = scene.node(&defender_node_id).pos.clone();
        let from = defender_pos.clone();
        let to = WorldPos{v: from.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
        let move_helper = MoveHelper::new(&from, &to, 1.0);
        let mut shell_move = None;
        let mut shell_node_id = None;
        if let Some(ref attacker_id) = attack_info.attacker_id {
            let attacker_node_id = scene.unit_id_to_node_id(&attacker_id);
            let attacker_pos = scene.node(&attacker_node_id).pos.clone();
            let attacker_map_pos = state.unit(&attacker_id).pos.clone();
            if let core::FireMode::Reactive = attack_info.mode {
                map_text.add_text(&attacker_map_pos, "reaction fire");
            }
            shell_node_id = Some(scene.add_node(SceneNode {
                pos: from.clone(),
                rot: geom::get_rot_angle(&attacker_pos, &defender_pos),
                mesh_id: Some(shell_mesh_id.clone()),
                children: Vec::new(),
            }));
            let shell_speed = 10.0;
            shell_move = Some(MoveHelper::new(
                &attacker_pos, &defender_pos, shell_speed));
        }
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
            defender_node_id: defender_node_id,
            killed: attack_info.killed.clone(),
            is_inderect: attack_info.is_inderect.clone(),
            is_target_destroyed: is_target_destroyed,
            move_helper: move_helper,
            shell_move: shell_move,
            shell_node_id: shell_node_id,
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
            let shell_node_id = self.shell_node_id.as_ref().unwrap();
            let mut pos = shell_move.step(dtime);
            if self.is_inderect {
                pos.v.z += (shell_move.progress() * PI).sin() * 5.0;
            }
            scene.node_mut(shell_node_id).pos = pos;
        }
        let is_shell_ok = if let Some(ref shell_move) = self.shell_move {
            shell_move.is_finished()
        } else {
            true
        };
        if is_shell_ok && self.shell_move.is_some() {
            if let Some(ref shell_node_id) = self.shell_node_id {
                scene.remove_node(shell_node_id);
            }
            self.shell_move = None;
            self.shell_node_id = None;
        }
        if is_shell_ok && self.killed > 0 {
            let step = self.move_helper.step_diff(dtime);
            let children = &mut scene.node_mut(&self.defender_node_id).children;
            for i in 0 .. self.killed as usize {
                let child = children.get_mut(i)
                    .expect("draw: no child");
                child.pos.v = child.pos.v + step;
            }
        }
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        if self.killed > 0 {
            let children = &mut scene.node_mut(&self.defender_node_id).children;
            assert!(self.killed as usize <= children.len());
            for _ in 0 .. self.killed {
                let _ = children.remove(0);
            }
        }
        if self.is_target_destroyed {
            scene.remove_node(&self.defender_node_id);
        }
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

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

pub struct EventHideUnitVisualizer;

impl EventHideUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &PartialState,
        unit_id: &UnitId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let pos = state.unit(unit_id).pos.clone();
        map_text.add_text(&pos, "lost");
        scene.remove_unit(unit_id);
        Box::new(EventHideUnitVisualizer)
    }
}

impl EventVisualizer for EventHideUnitVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: &Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

pub struct EventUnloadUnitVisualizer {
    node_id: NodeId,
    move_helper: MoveHelper,
}

impl EventUnloadUnitVisualizer {
    pub fn new(
        db: &Db,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: &MeshId,
        marker_mesh_id: &MeshId,
        transporter_pos: &ExactPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        map_text.add_text(&unit_info.pos, "unloaded");
        let to = geom::exact_pos_to_world_pos(&unit_info.pos);
        let from = geom::exact_pos_to_world_pos(transporter_pos);
        show_unit_at(db, scene, unit_info, mesh_id, marker_mesh_id);
        let node_id = scene.unit_id_to_node_id(&unit_info.unit_id);
        let unit_node = scene.node_mut(&node_id);
        unit_node.pos = from.clone();
        unit_node.rot = geom::get_rot_angle(&from, &to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventUnloadUnitVisualizer {
            node_id: node_id,
            move_helper: MoveHelper::new(&from, &to, move_speed),
        })
    }
}

impl EventVisualizer for EventUnloadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node = scene.node_mut(&self.node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

pub struct EventLoadUnitVisualizer {
    passenger_id: UnitId,
    move_helper: MoveHelper,
}

impl EventLoadUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &PartialState,
        unit_id: &UnitId,
        transporter_pos: &ExactPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let unit_pos = &state.unit(unit_id).pos;
        map_text.add_text(unit_pos, "loaded");
        let from = geom::exact_pos_to_world_pos(unit_pos);
        let to = geom::exact_pos_to_world_pos(transporter_pos);
        let passenger_node_id = scene.unit_id_to_node_id(unit_id);
        let unit_node = scene.node_mut(&passenger_node_id);
        unit_node.rot = geom::get_rot_angle(&from, &to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventLoadUnitVisualizer {
            passenger_id: unit_id.clone(),
            move_helper: MoveHelper::new(&from, &to, move_speed),
        })
    }
}

impl EventVisualizer for EventLoadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: &Time) {
        let node_id = scene.unit_id_to_node_id(&self.passenger_id);
        let node = scene.node_mut(&node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        scene.remove_unit(&self.passenger_id);
    }
}

pub struct EventSetReactionFireModeVisualizer;

impl EventSetReactionFireModeVisualizer {
    pub fn new(
        state: &PartialState,
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

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
