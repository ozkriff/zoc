use std::f32::consts::{PI};
use std::fmt::{Debug};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, Rad};
use core::game_state::{State};
use core::unit::{Unit, UnitId};
use core::sector::{SectorId};
use core::position::{MapPos, ExactPos};
use core::event::{FireMode, AttackInfo, ReactionFireMode};
use core::player::{PlayerId};
use core::object::{ObjectId};
// use core::effect::{self, Effect, TimedEffect};
use types::{WorldPos, Time, Speed};
use mesh::{MeshId};
use geom::{self, vec3_z};
use gen;
use scene::{Scene, SceneNode, NodeId};
use unit_type_visual_info::{UnitTypeVisualInfo, UnitTypeVisualInfoManager};
use move_helper::{MoveHelper};
use map_text::{MapTextManager};
use mesh_manager::{MeshIdManager};

// TODO: Move to some other place
pub const WRECKS_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

// TODO: rename to Action? It's not direcly connected to `CoreEvent` anymore
//
// TODO: Remove default impl. Or not?
//
pub trait Action: Debug {
    fn is_finished(&self) -> bool { true }

    // TODO: I'm not sure that `begin\end` must mutate the scene
    // TODO: Can I get rid of begin and end somehow? Should I?
    fn begin(&mut self, _: &mut Scene) {}
    fn update(&mut self, _: &mut Scene, _: Time) {} // TODO: fix arg (what is wrong with the args?)
    fn end(&mut self, _: &mut Scene) {}
}

// TODO: rename to `Move` and use as `action::Move`
//
// TODO: join with MoveHelper?
//
#[derive(Debug)]
pub struct ActionMove {
    node_id: NodeId,
    speed: Speed,
    to: WorldPos,

    // TODO: use builder pattern?
    move_helper: Option<MoveHelper>,
}

impl Action for ActionMove {
    fn begin(&mut self, scene: &mut Scene) {
        let node = scene.node_mut(self.node_id);
        self.move_helper = Some(MoveHelper::new(
            node.pos, self.to, self.speed));

        // TODO: get from MoveHelper?
        let rot = geom::get_rot_angle(node.pos, self.to);
        node.rot = rot;
    }

    fn update(&mut self, scene: &mut Scene, dtime: Time) {
        let pos = self.move_helper.as_mut().unwrap().step(dtime);
        scene.node_mut(self.node_id).pos = pos;
    }

    fn is_finished(&self) -> bool {
        self.move_helper.as_ref().unwrap().is_finished()
    }

    fn end(&mut self, scene: &mut Scene) {
        scene.node_mut(self.node_id).pos = self.to;
    }
}

pub fn visualize_event_move(
    state: &State,
    scene: &Scene,
    unit_id: UnitId,
    visual_info: &UnitTypeVisualInfo,
    destination: ExactPos,
) -> Vec<Box<Action>> {
    vec![
        Box::new(ActionMove {
            node_id: scene.unit_id_to_node_id(unit_id),
            to: geom::exact_pos_to_world_pos(state, destination),
            speed: visual_info.move_speed,
            move_helper: None,
        }),
    ]
}

fn try_to_fix_attached_unit_pos(
    scene: &mut Scene,
    transporter_id: UnitId,
    attached_unit_id: UnitId,
) {
    let transporter_node_id = scene.unit_id_to_node_id(transporter_id);
    let attached_unit_node_id
        = match scene.unit_id_to_node_id_opt(attached_unit_id)
    {
        Some(id) => id,
        // this unit's scene node is already attached to transporter's scene node
        None => return,
    };
    let mut node = scene.node_mut(attached_unit_node_id)
        .children.remove(0);
    scene.remove_unit(attached_unit_id);
    node.pos.v.y = -0.5; // TODO: get from UnitTypeVisualInfo
    node.rot += Rad(PI);
    scene.node_mut(transporter_node_id).children.push(node);
    scene.node_mut(transporter_node_id).children[0].pos.v.y = 0.5;
}

// TODO: remove
fn show_unit_at(
    pos: WorldPos,
    scene: &mut Scene,
    unit: &Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
) {
    let node_id = scene.allocate_node_id();
    show_unit_with_node_id(node_id, pos, scene, unit, mesh_id, marker_mesh_id);
}

// TODO: rename
fn show_unit_with_node_id(
    node_id: NodeId,
    pos: WorldPos,
    scene: &mut Scene,
    unit: &Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
) {
    let rot = Rad(thread_rng().gen_range(0.0, PI * 2.0));
    let mut children = get_unit_scene_nodes(unit, mesh_id);
    if unit.is_alive {
        children.push(SceneNode {
            pos: WorldPos{v: vec3_z(geom::HEX_EX_RADIUS / 2.0)},
            rot: Rad(0.0),
            mesh_id: Some(marker_mesh_id),
            color: gen::get_player_color(unit.player_id),
            children: Vec::new(),
        });
    }
    scene.add_unit(node_id, unit.id, SceneNode {
        pos: pos,
        rot: rot,
        mesh_id: None,
        color: [1.0, 1.0, 1.0, 1.0],
        children: children,
    });
}

fn get_unit_scene_nodes(unit: &Unit, mesh_id: MeshId) -> Vec<SceneNode> {
    let color = if unit.is_alive {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        WRECKS_COLOR
    };
    let mut vec = Vec::new();
    if unit.count == 1 {
        vec![SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}},
            rot: Rad(0.0),
            mesh_id: Some(mesh_id),
            color: color,
            children: vec![],
        }]
    } else {
        for i in 0 .. unit.count {
            let pos = geom::index_to_circle_vertex(unit.count, i).v * 0.15;
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

pub fn visualize_event_create_unit(
    state: &State,
    scene: &mut Scene,
    unit_info: &Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
) -> Vec<Box<Action>> {
    let to = geom::exact_pos_to_world_pos(state, unit_info.pos);
    let from = WorldPos{v: to.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
    let node_id = scene.allocate_node_id();
    vec![
        Box::new(ActionCreateUnit {
            pos: from,
            node_id: node_id,
            unit_info: unit_info.clone(),
            mesh_id: mesh_id,
            marker_mesh_id: marker_mesh_id,
        }),
        Box::new(ActionMove {
            node_id: node_id,
            to: to,
            speed: Speed{n: 2.0},
            move_helper: None,
        }),

    ]
}

#[derive(Debug)]
pub struct ActionCreateNode {
    node_id: NodeId,
    node: SceneNode,
}

impl Action for ActionCreateNode {
    fn begin(&mut self, scene: &mut Scene) {
        // TODO: Can I get rid of this `.clone()` somehow?
        scene.set_node(self.node_id, self.node.clone());
    }
}

#[derive(Debug)]
pub struct ActionRemoveNode {
    node_id: NodeId,
}

impl Action for ActionRemoveNode {
    fn begin(&mut self, scene: &mut Scene) {
        // TODO: check something?
        scene.remove_node(self.node_id);
    }
}

#[derive(Debug)]
pub struct ActionRemoveUnit {
    unit_id: UnitId,
}

impl Action for ActionRemoveUnit {
    fn begin(&mut self, scene: &mut Scene) {
        // TODO: check something?
        scene.remove_unit(self.unit_id);
    }
}

// TODO: Action::CreateSceneNode?
#[derive(Debug)]
pub struct ActionCreateUnit {
    unit_info: Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
    pos: WorldPos,
    node_id: NodeId,
}

impl Action for ActionCreateUnit {
    fn begin(&mut self, scene: &mut Scene) {
        show_unit_with_node_id(
            self.node_id,
            self.pos,
            scene,
            &self.unit_info,
            self.mesh_id,
            self.marker_mesh_id,
        );
    }
}

// TODO: Remove
/*
#[derive(Debug)]
pub struct EventAttackUnitVisualizer {
    shell_move: Option<MoveHelper>,
    shell_node_id: Option<NodeId>,
    attack_info: AttackInfo,
}
*/

// this code was removed from `visualize_event_attack`
/*
    let attack_info = attack_info.clone();
    let to = WorldPos{v: from.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
    let speed = Speed{n: 1.0};
    let move_helper = MoveHelper::new(from, to, speed);
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
    if is_target_destroyed {
        if let Some(attached_unit_id) = defender.attached_unit_id {
            let attached_unit = state.unit(attached_unit_id);
            let attached_unit_mesh_id = visual_info
                .get(attached_unit.type_id).mesh_id;
            show_unit_at(
                state,
                scene,
                attached_unit,
                attached_unit_mesh_id,
                mesh_ids.marker_mesh_id,
            );
            // TODO: fix attached unit pos
        }
    } else {
        map_text.add_text(
            defender.pos.map_pos,
            &format!("morale: -{}", attack_info.suppression),
        );
        if is_target_suppressed {
            map_text.add_text(defender.pos.map_pos, "suppressed");
        }
    }
*/

pub fn visualize_event_attack(
    state: &State,
    scene: &mut Scene,
    attack_info: &AttackInfo,
    mesh_ids: &MeshIdManager,
    map_text: &mut MapTextManager,
) -> Vec<Box<Action>> {
    let mut actions = Vec::new();
    let target_pos = geom::exact_pos_to_world_pos(state, attack_info.target_pos);
    if let Some(attacker_id) = attack_info.attacker_id {
        let attacker_node_id = scene.unit_id_to_node_id(attacker_id);
        let attacker_pos = scene.node(attacker_node_id).pos;
        let attacker_map_pos = state.unit(attacker_id).pos.map_pos;
        if attack_info.mode == FireMode::Reactive {
            // TODO: ActionShowText
            map_text.add_text(attacker_map_pos, "reaction fire");
        }
        let node_id = scene.allocate_node_id();
        actions.push(Box::new(ActionCreateNode {
            node_id: node_id,
            node: SceneNode {
                pos: attacker_pos,
                rot: geom::get_rot_angle(attacker_pos, target_pos),
                mesh_id: Some(mesh_ids.shell_mesh_id),
                color: [1.0, 1.0, 1.0, 1.0],
                children: Vec::new(),
            },
        }) as Box<Action>);

        // TODO: simulate arc for inderect fire:
        // if self.attack_info.is_inderect {
        //     pos.v.z += (shell_move.progress() * PI).sin() * 5.0;
        // }

        actions.push(Box::new(ActionMove {
            node_id: node_id,
            to: target_pos,
            speed: Speed{n: 10.0},
            move_helper: None,
        }));

        actions.push(Box::new(ActionRemoveNode {
            node_id: node_id,
        }));
    }
    if attack_info.is_ambush {
        map_text.add_text(attack_info.target_pos.map_pos, "Ambushed");
    };
    actions
}

/*
impl Action for EventAttackUnitVisualizer {
    fn is_finished(&self) -> bool {
        if self.attack_info.killed > 0 && !self.attack_info.leave_wrecks {
            self.move_helper.is_finished()
        } else if let Some(ref shell_move) = self.shell_move {
            shell_move.is_finished()
        } else {
            true
        }

        // TODO: воскреси старую логику
        // 
        // Вообще, это странный момент: как визуализировать событие атаки,
        // если оно из засады и я вообще не могу рисовать снаряд?
        //
        // Может, надо как-то обозначать район, из которого "прилетело"?
        // В духе "случайно сдвинутый круг из 7 клеток,
        // из одной из которых и стреляли".
        //
        if let Some(ref shell_move) = self.shell_move {
            shell_move.is_finished()
        } else {
            true
        }
    }

    fn update(&mut self, scene: &mut Scene, dtime: Time) {
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
                    .expect("update: no child");
                if !self.attack_info.leave_wrecks {
                    child.pos.v += step;
                }
            }
        }
    }

    fn end(&mut self, _: &mut Scene) {
        if self.attack_info.killed > 0 {
            let children = &mut scene.node_mut(self.defender_node_id).children;
            let killed = self.attack_info.killed as usize;
            assert!(killed <= children.len());
            for i in 0 .. killed {
                if self.attack_info.leave_wrecks {
                    children[i].color = WRECKS_COLOR;
                } else {
                    let _ = children.remove(0);
                }
            }
        }
        if self.is_target_destroyed {
            if self.attached_unit_id.is_some() {
                scene.node_mut(self.defender_node_id).children.pop().unwrap();
            }
            // delete unit's marker
            scene.node_mut(self.defender_node_id).children.pop().unwrap();
            if !self.attack_info.leave_wrecks {
                assert_eq!(scene.node(self.defender_node_id).children.len(), 0);
                scene.remove_node(self.defender_node_id);
            }
        }
    }
}
*/

#[derive(Debug)]
pub struct EventShowUnitVisualizer;

impl EventShowUnitVisualizer {
    pub fn new(
        state: &State,
        scene: &mut Scene,
        unit_info: &Unit,
        mesh_id: MeshId,
        marker_mesh_id: MeshId,
        map_text: &mut MapTextManager,
    ) -> Vec<Box<Action>> {
        map_text.add_text(unit_info.pos.map_pos, "spotted");
        let pos = geom::exact_pos_to_world_pos(state, unit_info.pos);
        show_unit_at(pos, scene, unit_info, mesh_id, marker_mesh_id);
        if let Some(attached_unit_id) = unit_info.attached_unit_id {
            try_to_fix_attached_unit_pos(
                scene, unit_info.id, attached_unit_id);
        }
        for unit in state.units_at(unit_info.pos.map_pos) {
            if let Some(attached_unit_id) = unit.attached_unit_id {
                try_to_fix_attached_unit_pos(
                    scene, unit.id, attached_unit_id);
            }
        }
        vec![Box::new(EventShowUnitVisualizer)]
    }
}

impl Action for EventShowUnitVisualizer {}

#[derive(Debug)]
pub struct EventHideUnitVisualizer;

impl EventHideUnitVisualizer {
    pub fn new(
        scene: &mut Scene,
        _: &State,
        unit_id: UnitId,
        map_text: &mut MapTextManager,
    ) -> Vec<Box<Action>> {
        // passenger doesn't have any scene node
        if let Some(node_id) = scene.unit_id_to_node_id_opt(unit_id) {
            // We can't read 'pos' from `state.unit(unit_id).pos`
            // because this unit may be in a fogged tile now
            // so State will filter him out.
            let world_pos = scene.node(node_id).pos;
            let map_pos = geom::world_pos_to_map_pos(world_pos);
            map_text.add_text(map_pos, "lost");
            scene.remove_unit(unit_id);
        }
        vec![Box::new(EventHideUnitVisualizer)]
    }
}

impl Action for EventHideUnitVisualizer {}

pub fn visualize_event_unload(
    state: &State,
    scene: &mut Scene,
    unit: &Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
    transporter_pos: ExactPos,
    visual_info: &UnitTypeVisualInfo,
    map_text: &mut MapTextManager,
) -> Vec<Box<Action>> {
    map_text.add_text(unit.pos.map_pos, "unloaded"); // TODO ActionShowText
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = geom::exact_pos_to_world_pos(state, transporter_pos);
    let node_id = scene.allocate_node_id();
    let action_create = Box::new(ActionCreateUnit {
        pos: from,
        node_id: node_id,
        unit_info: unit.clone(),
        mesh_id: mesh_id,
        marker_mesh_id: marker_mesh_id,
    });

    // TODO: I need to use `node_id` here
    // let action_move = ActionMove::new(
    //     state, scene, unit.id, visual_info, unit.pos);

    let speed = visual_info.move_speed;

    let action_move = Box::new(ActionMove {
        node_id: node_id,
        to: to,
        speed: speed,
        move_helper: None,
    });
    // unit_node.rot = geom::get_rot_angle(from, to);
    vec![action_create, action_move]
}

pub fn visualize_event_load(
    scene: &mut Scene,
    state: &State,
    unit_id: UnitId,
    transporter_pos: ExactPos,
    visual_info: &UnitTypeVisualInfo,
    map_text: &mut MapTextManager,
) -> Vec<Box<Action>> {
    let unit_pos = state.unit(unit_id).pos;
    map_text.add_text(unit_pos.map_pos, "loaded");
    // let from = geom::exact_pos_to_world_pos(state, unit_pos);
    let to = geom::exact_pos_to_world_pos(state, transporter_pos);
    let unit_node_id = scene.unit_id_to_node_id(unit_id);
    let speed = visual_info.move_speed;
    vec![
        Box::new(ActionMove {
            node_id: unit_node_id,
            to: to,
            speed: speed,
            move_helper: None,
        }),
        Box::new(ActionRemoveUnit{unit_id: unit_id}),
    ]
}

#[derive(Debug)]
pub struct EventSetReactionFireModeVisualizer;

impl EventSetReactionFireModeVisualizer {
    pub fn new(
        state: &State,
        unit_id: UnitId,
        mode: ReactionFireMode,
        map_text: &mut MapTextManager,
    ) -> Box<Action> {
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

impl Action for EventSetReactionFireModeVisualizer{}

#[derive(Debug)]
pub struct EventSectorOwnerChangedVisualizer;

impl EventSectorOwnerChangedVisualizer {
    pub fn new(
        scene: &mut Scene,
        state: &State,
        sector_id: SectorId,
        owner_id: Option<PlayerId>,
        map_text: &mut MapTextManager,
    ) -> Box<Action> {
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

impl Action for EventSectorOwnerChangedVisualizer{}

#[derive(Debug)]
pub struct EventVictoryPointVisualizer {
    time: Time,
    duration: Time,
}

impl EventVictoryPointVisualizer {
    pub fn new(
        pos: MapPos,
        count: i32,
        map_text: &mut MapTextManager,
    ) -> Box<Action> {
        let text = format!("+{} VP!", count);
        map_text.add_text(pos, &text);
        Box::new(EventVictoryPointVisualizer{
            time: Time{n: 0.0},
            duration: Time{n: 1.0},
        })
    }
}

impl Action for EventVictoryPointVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n >= self.duration.n
    }

    fn update(&mut self, _: &mut Scene, dt: Time) {
        self.time.n += dt.n;
    }
}

const SMOKE_ALPHA: f32 = 0.7;

#[derive(Debug)]
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
    ) -> Box<Action> {
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

impl Action for EventSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn update(&mut self, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = self.time.n / self.duration.n;
        }
    }
}

#[derive(Debug)]
pub struct EventRemoveSmokeVisualizer {
    duration: Time,
    time: Time,
    object_id: ObjectId,
}

impl EventRemoveSmokeVisualizer {
    pub fn new(
        state: &State,
        object_id: ObjectId,
        map_text: &mut MapTextManager,
    ) -> Box<Action> {
        let pos = state.objects()[&object_id].pos.map_pos;
        map_text.add_text(pos, "smoke cleared");
        Box::new(EventRemoveSmokeVisualizer {
            time: Time{n: 0.0},
            duration: Time{n: 1.0},
            object_id: object_id,
        })
    }
}

impl Action for EventRemoveSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn update(&mut self, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = SMOKE_ALPHA - self.time.n / self.duration.n;
        }
    }

    fn end(&mut self, scene: &mut Scene) {
        scene.remove_object(self.object_id);
    }
}

pub fn visualize_event_attach(
    state: &State,
    scene: &mut Scene,
    transporter_id: UnitId,
    attached_unit_id: UnitId,
    visual_info: &UnitTypeVisualInfo,
    map_text: &mut MapTextManager,
) -> Vec<Box<Action>> {
    let transporter = state.unit(transporter_id);
    let attached_unit = state.unit(attached_unit_id);
    map_text.add_text(transporter.pos.map_pos, "attached"); // TODO: ActionShowText
    let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, attached_unit.pos);
    let transporter_node_id = scene.unit_id_to_node_id(transporter_id);
    let unit_node = scene.node_mut(transporter_node_id);
    unit_node.rot = geom::get_rot_angle(from, to);
    let speed = visual_info.move_speed;
    vec![
        Box::new(ActionMove {
            node_id: transporter_node_id,
            to: to,
            speed: speed,
            move_helper: None,
        }),
        Box::new(ActionAttach {
            transporter_id: transporter_id,
            attached_unit_id: attached_unit_id,
        })
    ]
}

#[derive(Debug)]
pub struct ActionAttach {
    transporter_id: UnitId,
    attached_unit_id: UnitId,
}

impl Action for ActionAttach {
    fn begin(&mut self, scene: &mut Scene) {
        try_to_fix_attached_unit_pos(
            scene, self.transporter_id, self.attached_unit_id);
    }
}

pub fn visualize_event_detach(
    state: &State,
    scene: &mut Scene,
    transporter_id: UnitId,
    pos: ExactPos,
    mesh_ids: &MeshIdManager,
    visual_info: &UnitTypeVisualInfoManager,
    map_text: &mut MapTextManager,
) -> Vec<Box<Action>> {
    let transporter = state.unit(transporter_id);
    map_text.add_text(transporter.pos.map_pos, "detached");
    let attached_unit_id = transporter.attached_unit_id.unwrap();
    let attached_unit = state.unit(attached_unit_id);
    let transporter_visual_info
        = visual_info.get(transporter.type_id);
    let attached_unit_mesh_id = visual_info
        .get(attached_unit.type_id).mesh_id;
    let attached_unit_node_id = scene.allocate_node_id();
    let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, pos);
    let transporter_node_id = scene.unit_id_to_node_id(transporter_id);
    let speed = transporter_visual_info.move_speed;
    vec![
        Box::new(ActionCreateUnit {
            unit_info: attached_unit.clone(),
            mesh_id: attached_unit_mesh_id,
            marker_mesh_id: mesh_ids.marker_mesh_id,
            pos: geom::exact_pos_to_world_pos(state, attached_unit.pos),
            node_id: attached_unit_node_id,
        }),
        Box::new(ActionDetach {
            from: from,
            to: to,
            transporter_node_id: transporter_node_id,
        }),
        Box::new(ActionMove {
            node_id: transporter_node_id,
            to: to,
            speed: speed,
            move_helper: None,
        }),
    ]
}

#[derive(Debug)]
pub struct ActionDetach {
    from: WorldPos,
    to: WorldPos,
    transporter_node_id: NodeId,
    // attached_unit_id: UnitId,
}

impl Action for ActionDetach {
    fn begin(&mut self, scene: &mut Scene) {
        let transporter_node = scene.node_mut(self.transporter_node_id);
        transporter_node.rot = geom::get_rot_angle(self.from, self.to);
        transporter_node.children[0].pos.v.y = 0.0;
        transporter_node.children.pop();
    }
}
