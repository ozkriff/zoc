use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, Rad};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use core::{self, UnitInfo, AttackInfo, ReactionFireMode, UnitId, ExactPos, PlayerId, SectorId, MapPos, ObjectId};
use core::db::{Db};
use types::{WorldPos, Time};
use mesh::{MeshId};
use geom;
use gen;
use scene::{Scene, SceneNode, NodeId};
use unit_type_visual_info::{UnitTypeVisualInfo};
use move_helper::{MoveHelper};
use map_text::{MapTextManager};

static WRECKS_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

pub trait EventVisualizer {
    fn is_finished(&self) -> bool;
    fn draw(&mut self, scene: &mut Scene, dtime: Time);
    fn end(&mut self, scene: &mut Scene, state: &PartialState);
}

#[derive(Clone, Debug)]
pub struct EventMoveVisualizer {
    node_id: NodeId,
    move_helper: MoveHelper,
}

impl EventVisualizer for EventMoveVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        let pos = self.move_helper.step(dtime);
        scene.node_mut(self.node_id).pos = pos;
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        let node = scene.node_mut(self.node_id);
        node.pos = self.move_helper.destination();
    }
}

impl EventMoveVisualizer {
    pub fn new(
        state: &PartialState,
        scene: &mut Scene,
        unit_id: UnitId,
        unit_type_visual_info: &UnitTypeVisualInfo,
        destination: ExactPos,
    ) -> Box<EventVisualizer> {
        let speed = unit_type_visual_info.move_speed;
        let node_id = scene.unit_id_to_node_id(unit_id);
        let node = scene.node_mut(node_id);
        let from = node.pos;
        let to = geom::exact_pos_to_world_pos(state, destination);
        node.rot = geom::get_rot_angle(from, to);
        let move_helper = MoveHelper::new(from, to, speed);
        Box::new(EventMoveVisualizer {
            node_id: node_id,
            move_helper: move_helper,
        })
    }
}

#[derive(Clone, Debug)]
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

    fn draw(&mut self, _: &mut Scene, _: Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

fn show_unit_at(
    db: &Db,
    state: &PartialState,
    scene: &mut Scene,
    unit_info: &UnitInfo,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
) {
    let world_pos = geom::exact_pos_to_world_pos(state, unit_info.pos);
    let to = world_pos;
    let rot = Rad(thread_rng().gen_range(0.0, PI * 2.0));
    let mut children = get_unit_scene_nodes(db, unit_info, mesh_id);
    if unit_info.is_alive {
        children.push(SceneNode {
            pos: WorldPos{v: vec3_z(geom::HEX_EX_RADIUS / 2.0)},
            rot: Rad(0.0),
            mesh_id: Some(marker_mesh_id),
            color: gen::get_player_color(unit_info.player_id),
            children: Vec::new(),
        });
    }
    scene.add_unit(unit_info.unit_id, SceneNode {
        pos: to,
        rot: rot,
        mesh_id: None,
        color: [1.0, 1.0, 1.0, 1.0],
        children: children,
    });
}

#[derive(Clone, Debug)]
pub struct EventCreateUnitVisualizer {
    node_id: NodeId,
    move_helper: MoveHelper,
}

fn get_unit_scene_nodes(
    db: &Db,
    unit_info: &UnitInfo,
    mesh_id: MeshId,
) -> Vec<SceneNode> {
    let color = if unit_info.is_alive {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        WRECKS_COLOR
    };
    let count = db.unit_type(unit_info.type_id).count;
    let mut vec = Vec::new();
    if count == 1 {
        vec![SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}},
            rot: Rad(0.0),
            mesh_id: Some(mesh_id),
            color: color,
            children: vec![],
        }]
    } else {
        for i in 0 .. count {
            let pos = geom::index_to_circle_vertex(count, i).v * 0.15;
            vec.push(SceneNode {
                pos: WorldPos{v: pos},
                rot: Rad(0.0),
                mesh_id: Some(mesh_id),
                color: color,
                children: vec![],
            });
        }
        vec
    }
}

impl EventCreateUnitVisualizer {
    pub fn new(
        db: &Db,
        state: &PartialState,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: MeshId,
        marker_mesh_id: MeshId,
    ) -> Box<EventVisualizer> {
        let to = geom::exact_pos_to_world_pos(state, unit_info.pos);
        let from = WorldPos{v: to.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
        show_unit_at(db, state, scene, unit_info, mesh_id, marker_mesh_id);
        let move_helper = MoveHelper::new(from, to, 2.0);
        let node_id = scene.unit_id_to_node_id(unit_info.unit_id);
        let new_node = scene.node_mut(node_id);
        new_node.pos = from;
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

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        let node = scene.node_mut(self.node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

fn vec3_z(z: f32) -> Vector3<f32> {
    Vector3{x: 0.0, y: 0.0, z: z}
}

#[derive(Clone, Debug)]
pub struct EventAttackUnitVisualizer {
    defender_node_id: NodeId,
    is_target_destroyed: bool,
    move_helper: MoveHelper,
    shell_move: Option<MoveHelper>,
    shell_node_id: Option<NodeId>,
    attack_info: AttackInfo,
}

impl EventAttackUnitVisualizer {
    pub fn new(
        state: &PartialState,
        scene: &mut Scene,
        attack_info: &AttackInfo,
        shell_mesh_id: MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let attack_info = attack_info.clone();
        let defender = state.unit(attack_info.defender_id);
        let defender_node_id = scene.unit_id_to_node_id(attack_info.defender_id);
        let defender_pos = scene.node(defender_node_id).pos;
        let from = defender_pos;
        let to = WorldPos{v: from.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
        let move_helper = MoveHelper::new(from, to, 1.0);
        let mut shell_move = None;
        let mut shell_node_id = None;
        if let Some(attacker_id) = attack_info.attacker_id {
            let attacker_node_id = scene.unit_id_to_node_id(attacker_id);
            let attacker_pos = scene.node(attacker_node_id).pos;
            let attacker_map_pos = state.unit(attacker_id).pos.map_pos;
            if attack_info.mode == core::FireMode::Reactive {
                map_text.add_text(attacker_map_pos, "reaction fire");
            }
            shell_node_id = Some(scene.add_node(SceneNode {
                pos: from,
                rot: geom::get_rot_angle(attacker_pos, defender_pos),
                mesh_id: Some(shell_mesh_id),
                color: [1.0, 1.0, 1.0, 1.0],
                children: Vec::new(),
            }));
            let shell_speed = 10.0;
            shell_move = Some(MoveHelper::new(
                attacker_pos, defender_pos, shell_speed));
        }
        if attack_info.is_ambush {
            map_text.add_text(defender.pos.map_pos, "Ambushed");
        };
        let is_target_destroyed = defender.count - attack_info.killed <= 0;
        if attack_info.killed > 0 {
            map_text.add_text(
                defender.pos.map_pos,
                &format!("-{}", attack_info.killed),
            );
        } else {
            map_text.add_text(defender.pos.map_pos, "miss");
        }
        let is_target_suppressed = defender.morale < 50
            && defender.morale + attack_info.suppression >= 50;
        if !is_target_destroyed {
            map_text.add_text(
                defender.pos.map_pos,
                &format!("morale: -{}", attack_info.suppression),
            );
            if is_target_suppressed {
                map_text.add_text(defender.pos.map_pos, "suppressed");
            }
        }
        Box::new(EventAttackUnitVisualizer {
            defender_node_id: defender_node_id,
            attack_info: attack_info,
            is_target_destroyed: is_target_destroyed,
            move_helper: move_helper,
            shell_move: shell_move,
            shell_node_id: shell_node_id,
        })
    }
}

impl EventVisualizer for EventAttackUnitVisualizer {
    fn is_finished(&self) -> bool {
        if self.attack_info.killed > 0 && !self.attack_info.leave_wrecks {
            self.move_helper.is_finished()
        } else if let Some(ref shell_move) = self.shell_move {
            shell_move.is_finished()
        } else {
            true
        }
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        if let Some(ref mut shell_move) = self.shell_move {
            let shell_node_id = self.shell_node_id.unwrap();
            let mut pos = shell_move.step(dtime);
            if self.attack_info.is_inderect {
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
            if let Some(shell_node_id) = self.shell_node_id {
                scene.remove_node(shell_node_id);
            }
            self.shell_move = None;
            self.shell_node_id = None;
        }
        if is_shell_ok && self.attack_info.killed > 0 {
            let step = self.move_helper.step_diff(dtime);
            let children = &mut scene.node_mut(self.defender_node_id).children;
            for i in 0 .. self.attack_info.killed as usize {
                let child = children.get_mut(i)
                    .expect("draw: no child");
                if !self.attack_info.leave_wrecks {
                    child.pos.v += step;
                }
            }
        }
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        if self.attack_info.killed > 0 {
            let children = &mut scene.node_mut(self.defender_node_id).children;
            assert!(self.attack_info.killed as usize <= children.len());
            for i in 0 .. self.attack_info.killed as usize {
                if self.attack_info.leave_wrecks {
                    let child = children.get_mut(i).unwrap();
                    child.color = WRECKS_COLOR;
                } else {
                    let _ = children.remove(0);
                }
            }
        }
        if self.is_target_destroyed {
            // delete unit's marker
            scene.node_mut(self.defender_node_id).children.pop().unwrap();
            if !self.attack_info.leave_wrecks {
                let len = scene.node(self.defender_node_id).children.len();
                if len != 0 {
                    println!("warning: deleting node with {} children", len); // TODO
                }
                scene.remove_node(self.defender_node_id);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct EventShowUnitVisualizer;

impl EventShowUnitVisualizer {
    pub fn new(
        db: &Db,
        state: &PartialState,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: MeshId,
        marker_mesh_id: MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        map_text.add_text(unit_info.pos.map_pos, "spotted");
        show_unit_at(db, state, scene, unit_info, mesh_id, marker_mesh_id);
        Box::new(EventShowUnitVisualizer)
    }
}

impl EventVisualizer for EventShowUnitVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventHideUnitVisualizer;

impl EventHideUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &PartialState,
        unit_id: UnitId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let pos = state.unit(unit_id).pos.map_pos;
        map_text.add_text(pos, "lost");
        scene.remove_unit(unit_id);
        Box::new(EventHideUnitVisualizer)
    }
}

impl EventVisualizer for EventHideUnitVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventUnloadUnitVisualizer {
    node_id: NodeId,
    move_helper: MoveHelper,
}

impl EventUnloadUnitVisualizer {
    pub fn new(
        db: &Db,
        state: &PartialState,
        scene: &mut Scene,
        unit_info: &UnitInfo,
        mesh_id: MeshId,
        marker_mesh_id: MeshId,
        transporter_pos: ExactPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        map_text.add_text(unit_info.pos.map_pos, "unloaded");
        let to = geom::exact_pos_to_world_pos(state, unit_info.pos);
        let from = geom::exact_pos_to_world_pos(state, transporter_pos);
        show_unit_at(db, state, scene, unit_info, mesh_id, marker_mesh_id);
        let node_id = scene.unit_id_to_node_id(unit_info.unit_id);
        let unit_node = scene.node_mut(node_id);
        unit_node.pos = from;
        unit_node.rot = geom::get_rot_angle(from, to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventUnloadUnitVisualizer {
            node_id: node_id,
            move_helper: MoveHelper::new(from, to, move_speed),
        })
    }
}

impl EventVisualizer for EventUnloadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        let node = scene.node_mut(self.node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventLoadUnitVisualizer {
    passenger_id: UnitId,
    move_helper: MoveHelper,
}

impl EventLoadUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &PartialState,
        unit_id: UnitId,
        transporter_pos: ExactPos,
        unit_type_visual_info: &UnitTypeVisualInfo,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let unit_pos = state.unit(unit_id).pos;
        map_text.add_text(unit_pos.map_pos, "loaded");
        let from = geom::exact_pos_to_world_pos(state, unit_pos);
        let to = geom::exact_pos_to_world_pos(state, transporter_pos);
        let passenger_node_id = scene.unit_id_to_node_id(unit_id);
        let unit_node = scene.node_mut(passenger_node_id);
        unit_node.rot = geom::get_rot_angle(from, to);
        let move_speed = unit_type_visual_info.move_speed;
        Box::new(EventLoadUnitVisualizer {
            passenger_id: unit_id,
            move_helper: MoveHelper::new(from, to, move_speed),
        })
    }
}

impl EventVisualizer for EventLoadUnitVisualizer {
    fn is_finished(&self) -> bool {
        self.move_helper.is_finished()
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        let node_id = scene.unit_id_to_node_id(self.passenger_id);
        let node = scene.node_mut(node_id);
        node.pos = self.move_helper.step(dtime);
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        scene.remove_unit(self.passenger_id);
    }
}

#[derive(Clone, Debug)]
pub struct EventSetReactionFireModeVisualizer;

impl EventSetReactionFireModeVisualizer {
    pub fn new(
        state: &PartialState,
        unit_id: UnitId,
        mode: ReactionFireMode,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let unit_pos = state.unit(unit_id).pos.map_pos;
        match mode {
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

    fn draw(&mut self, _: &mut Scene, _: Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventSectorOwnerChangedVisualizer;

impl EventSectorOwnerChangedVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &PartialState,
        sector_id: SectorId,
        owner_id: Option<PlayerId>,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        // TODO: fix msg
        // "Sector {} secured by an enemy"
        // "Sector {} secured"
        // "Sector {} lost" ??
        let color = match owner_id {
            None => [1.0, 1.0, 1.0, 0.5],
            Some(PlayerId{id: 0}) => [0.0, 0.0, 0.8, 0.5],
            Some(PlayerId{id: 1}) => [0.0, 0.8, 0.0, 0.5],
            Some(_) => unimplemented!(),
        };
        let node_id = scene.sector_id_to_node_id(sector_id);
        let node = scene.node_mut(node_id);
        node.color = color;
        let sector = &state.sectors()[&sector_id];
        let pos = sector.center();
        let text = match owner_id {
            Some(id) => format!("Sector {}: owner changed: Player {}", sector_id.id, id.id),
            None => format!("Sector {}: owner changed: None", sector_id.id),
        };
        map_text.add_text(pos, &text);
        Box::new(EventSectorOwnerChangedVisualizer)
    }
}

impl EventVisualizer for EventSectorOwnerChangedVisualizer {
    fn is_finished(&self) -> bool {
        true
    }

    fn draw(&mut self, _: &mut Scene, _: Time) {}

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventVictoryPointVisualizer {
    time: Time,
    duration: Time,
}

impl EventVictoryPointVisualizer {
    pub fn new(
        pos: MapPos,
        count: i32,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let text = format!("+{} VP!", count);
        map_text.add_text(pos, &text);
        Box::new(EventVictoryPointVisualizer{
            time: Time{n: 0.0},
            duration: Time{n: 1.0},
        })
    }
}

impl EventVisualizer for EventVictoryPointVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n >= self.duration.n
    }

    fn draw(&mut self, _: &mut Scene, dt: Time) {
        self.time.n += dt.n;
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

const SMOKE_ALPHA: f32 = 0.7;

#[derive(Clone, Debug)]
pub struct EventSmokeVisualizer {
    duration: Time,
    time: Time,
    object_id: ObjectId,
}

impl EventSmokeVisualizer {
    pub fn new(
        scene: &mut Scene,
        pos: MapPos,
        _: Option<UnitId>, // TODO
        object_id: ObjectId,
        smoke_mesh_id: MeshId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        // println!("unit_id: {:?}", unit_id); // TODO
        // TODO: show shell animation
        map_text.add_text(pos, "smoke");
        let z_step = 0.45; // TODO: magic
        let mut node = SceneNode {
            pos: geom::map_pos_to_world_pos(pos),
            rot: Rad(0.0),
            mesh_id: Some(smoke_mesh_id),
            color: [1.0, 1.0, 1.0, 0.0],
            children: Vec::new(),
        };
        node.pos.v.z += z_step;
        node.rot += Rad(thread_rng().gen_range(0.0, PI * 2.0));
        scene.add_object(object_id, node.clone());
        node.pos.v.z += z_step;
        node.rot += Rad(thread_rng().gen_range(0.0, PI * 2.0));
        scene.add_object(object_id, node);
        Box::new(EventSmokeVisualizer {
            time: Time{n: 0.0},
            duration: Time{n: 1.0},
            object_id: object_id,
        })
    }
}

impl EventVisualizer for EventSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = self.time.n / self.duration.n;
        }
    }

    fn end(&mut self, _: &mut Scene, _: &PartialState) {}
}

#[derive(Clone, Debug)]
pub struct EventRemoveSmokeVisualizer {
    duration: Time,
    time: Time,
    object_id: ObjectId,
}

impl EventRemoveSmokeVisualizer {
    pub fn new(
        state: &PartialState,
        object_id: ObjectId,
        map_text: &mut MapTextManager,
    ) -> Box<EventVisualizer> {
        let pos = state.objects()[&object_id].pos.map_pos;
        map_text.add_text(pos, "smoke cleared");
        Box::new(EventRemoveSmokeVisualizer {
            time: Time{n: 0.0},
            duration: Time{n: 1.0},
            object_id: object_id,
        })
    }
}

impl EventVisualizer for EventRemoveSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn draw(&mut self, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = SMOKE_ALPHA - self.time.n / self.duration.n;
        }
    }

    fn end(&mut self, scene: &mut Scene, _: &PartialState) {
        scene.remove_object(self.object_id);
    }
}
