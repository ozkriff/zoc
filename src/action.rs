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
use mesh::{MeshId, Mesh};
use geom::{self, vec3_z};
use context::{Context};
use gen;
use scene::{Scene, SceneNode, SceneNodeType, NodeId};
use unit_type_visual_info::{UnitTypeVisualInfoManager};
use move_helper::{MoveHelper};
use mesh_manager::{MeshIdManager, MeshManager};
use text;
use pipeline::{Vertex};
use texture::{load_texture_raw};
use camera::{Camera};

const WRECKS_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

// TODO: RENAME
// TODO: Move to tactical_screen.rs?
pub struct ActionContext<'a> {
    pub camera: &'a Camera,
    pub mesh_ids: &'a MeshIdManager,
    pub scene: &'a mut Scene,
    pub context: &'a mut Context,
    pub meshes: &'a mut MeshManager,
    pub visual_info: &'a UnitTypeVisualInfoManager,
}

pub trait Action: Debug {
    fn is_finished(&self) -> bool { true }

    // TODO: I'm not sure that `begin\end` must mutate the scene
    // TODO: Can I get rid of begin and end somehow? Should I?
    fn begin(&mut self, _: ActionContext) {}
    fn update(&mut self, _: &Context, _: &mut Scene, _: Time) {}
    fn end(&mut self, _: &Context, _: &mut Scene) {}
}

#[derive(Debug)]
pub struct ActionCreateTextMesh {
    text: String,
    mesh_id: MeshId,
}

impl Action for ActionCreateTextMesh {
    fn begin(&mut self, context: ActionContext) {
        let text_size = 80.0; // TODO: ???
        let (size, texture_data) = text::text_to_texture(
            context.context.font(), text_size, &self.text);
        let texture = load_texture_raw(
            context.context.factory_mut(), size, &texture_data);
        let scale_factor = 200.0; // TODO: take camera zoom into account
        let h_2 = (size.h as f32 / scale_factor) / 2.0;
        let w_2 = (size.w as f32 / scale_factor) / 2.0;
        let vertices = &[
            Vertex{pos: [-w_2, -h_2, 0.0], uv: [0.0, 1.0]},
            Vertex{pos: [-w_2, h_2, 0.0], uv: [0.0, 0.0]},
            Vertex{pos: [w_2, -h_2, 0.0], uv: [1.0, 1.0]},
            Vertex{pos: [w_2, h_2, 0.0], uv: [1.0, 0.0]},
        ];
        let indices = &[0,  1,  2,  1,  2,  3];
        let mesh = Mesh::new_nodepth(context.context, vertices, indices, texture);
        context.meshes.set(self.mesh_id, mesh);
    }
}

#[derive(Debug)]
pub struct ActionRemoveMesh {
    mesh_id: MeshId,
}

impl Action for ActionRemoveMesh {
    fn begin(&mut self, context: ActionContext) {
        context.meshes.remove(self.mesh_id);
    }
}

#[derive(Debug)]
pub struct ActionSleep {
    duration: Time,
    time: Time,
}

impl Action for ActionSleep {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > 1.0
    }

    fn update(&mut self, _: &Context, _: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
    }
}

#[derive(Debug)]
pub struct ActionRotateTo {
    node_id: NodeId,
    to: WorldPos,
}

impl Action for ActionRotateTo {
    fn begin(&mut self, context: ActionContext) {
        let node = context.scene.node_mut(self.node_id);
        let rot = geom::get_rot_angle(node.pos, self.to);
        node.rot = rot;
    }
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
    // Or maybe I could fix this by changing the MoveHelper's logic.
    // Oooor use move_helper's mutability!
    move_helper: Option<MoveHelper>,
}

impl Action for ActionMove {
    fn begin(&mut self, context: ActionContext) {
        let node = context.scene.node_mut(self.node_id);
        self.move_helper = Some(MoveHelper::new(
            node.pos, self.to, self.speed));

        // TODO: get from MoveHelper?
        //
        // TODO: не факт, что это тут стит делать,
        // пущай отдельное действие поворотом занимается
        // let rot = geom::get_rot_angle(node.pos, self.to);
        // node.rot = rot;
    }

    fn update(&mut self, _: &Context, scene: &mut Scene, dtime: Time) {
        let pos = self.move_helper.as_mut().unwrap().step(dtime);
        scene.node_mut(self.node_id).pos = pos;
    }

    fn is_finished(&self) -> bool {
        self.move_helper.as_ref().unwrap().is_finished()
    }

    fn end(&mut self, _: &Context, scene: &mut Scene) {
        scene.node_mut(self.node_id).pos = self.to;
    }
}

// TODO Черт, меня бесит что теперь повсюду будут летать
// изменяемые ссылки на ActionContext, в котором ВСЕ.
//
// По хорошему, при создании новых действий,
// ссылка должна быть только на чтение для всего,
// кроме выделение nide_id, mesh_id.
// Тут без имзеняемости, видимо, никак.
//
// Что в Action::begin и т.п. будет изменяемый &mut ActionContext
// меня уже не так волнует.
//
// Может, есть способ избавиться от mut тут?
// Эти айдишники мне нужны только же для связи Action'ов
// между собой. Хмм, могу я что-то другое использовать для этого?
//
pub fn visualize_show_text(
    context: &mut ActionContext,
    destination: MapPos,
    text: &str,
) -> Vec<Box<Action>> {
    let node_id = context.scene.allocate_node_id();
    let mesh_id = context.meshes.allocate_id();
    let mut from = geom::map_pos_to_world_pos(destination);
    from.v.z += 0.3;
    let mut to = geom::map_pos_to_world_pos(destination);
    to.v.z += 1.5;
    vec![
        Box::new(ActionCreateTextMesh {
            text: text.into(),
            mesh_id: mesh_id,
        }),
        Box::new(ActionCreateNode {
            node_id: node_id,
            node: SceneNode {
                pos: from,
                rot: context.camera.get_z_angle(), // TODO: !?
                mesh_id: Some(mesh_id),
                color: [0.0, 0.0, 1.0, 1.0],
                node_type: SceneNodeType::Transparent,
                .. Default::default()
            },
        }),
        Box::new(ActionMove {
            node_id: node_id,
            to: to,
            speed: Speed{n: 1.0},
            move_helper: None,
        }),
        // Box::new(ActionSleep {
        //     duration: Time{n: 0.5},
        //     time: Time{n: 0.0},
        // }),
        Box::new(ActionRemoveNode {
            node_id: node_id,
        }),
        Box::new(ActionRemoveMesh {
            mesh_id: mesh_id,
        }),
    ]
}

pub fn visualize_event_move(
    state: &State,
    context: &mut ActionContext,
    unit_id: UnitId,
    destination: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions = Vec::new();
    let type_id = state.unit(unit_id).type_id;
    let unit_visual_info = context.visual_info.get(type_id);
    let node_id = context.scene.unit_id_to_node_id(unit_id);
    let to = geom::exact_pos_to_world_pos(state, destination);
    actions.push(Box::new(ActionRotateTo {
        node_id: node_id,
        to: to,
    }) as Box<Action>);
    actions.push(Box::new(ActionMove {
        node_id: node_id,
        to: to,
        speed: unit_visual_info.move_speed,
        move_helper: None,
    }) as Box<Action>);
    actions
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
            mesh_id: Some(mesh_id),
            color: color,
            .. Default::default()
        }]
    } else {
        for i in 0 .. unit.count {
            let pos = geom::index_to_circle_vertex(unit.count, i).v * 0.15;
            vec.push(SceneNode {
                pos: WorldPos{v: pos},
                mesh_id: Some(mesh_id),
                color: color,
                .. Default::default()
            });
        }
        vec
    }
}

pub fn visualize_event_create_unit(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
) -> Vec<Box<Action>> {
    let mesh_id = context.visual_info
        .get(unit.type_id).mesh_id;
    let marker_mesh_id = context.mesh_ids.marker_mesh_id;
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = WorldPos{v: to.v - vec3_z(geom::HEX_EX_RADIUS / 2.0)};
    let node_id = context.scene.allocate_node_id();
    vec![
        Box::new(ActionCreateUnit {
            pos: from,
            node_id: node_id,
            unit: unit.clone(),
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
    fn begin(&mut self, context: ActionContext) {
        // TODO: Can I get rid of this `.clone()` somehow?
        context.scene.set_node(self.node_id, self.node.clone());
    }
}

#[derive(Debug)]
pub struct ActionRemoveNode {
    node_id: NodeId,
}

impl Action for ActionRemoveNode {
    fn begin(&mut self, context: ActionContext) {
        // TODO: check something?
        context.scene.remove_node(self.node_id);
    }
}

#[derive(Debug)]
pub struct ActionRemoveUnit {
    unit_id: UnitId,
}

impl Action for ActionRemoveUnit {
    fn begin(&mut self, context: ActionContext) {
        // TODO: check something?
        context.scene.remove_unit(self.unit_id);
    }
}

// TODO: Action::CreateSceneNode?
#[derive(Debug)]
pub struct ActionCreateUnit {
    unit: Unit,
    mesh_id: MeshId,
    marker_mesh_id: MeshId,
    pos: WorldPos,
    node_id: NodeId,
}

impl Action for ActionCreateUnit {
    fn begin(&mut self, context: ActionContext) {
        let rot = Rad(thread_rng().gen_range(0.0, PI * 2.0));
        let mut children = get_unit_scene_nodes(&self.unit, self.mesh_id);
        if self.unit.is_alive {
            children.push(SceneNode {
                pos: WorldPos{v: vec3_z(geom::HEX_EX_RADIUS / 2.0)},
                mesh_id: Some(self.marker_mesh_id),
                color: gen::get_player_color(self.unit.player_id),
                .. Default::default()
            });
        }
        context.scene.add_unit(self.node_id, self.unit.id, SceneNode {
            pos: self.pos,
            rot: rot,
            children: children,
            .. Default::default()
        });
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
        context.text.add_text(
            defender.pos.map_pos,
            &format!("-{}", attack_info.killed),
        );
    } else {
        context.text.add_text(defender.pos.map_pos, "miss");
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
        context.text.add_text(
            defender.pos.map_pos,
            &format!("morale: -{}", attack_info.suppression),
        );
        if is_target_suppressed {
            context.text.add_text(defender.pos.map_pos, "suppressed");
        }
    }
*/

// TODO: split this effect into many
pub fn visualize_effect_attacked(
    state: &State,
    context: &mut ActionContext,
    target_id: UnitId,
    killed: i32,
    leave_wrecks: bool,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let target = state.unit(target_id);
    actions.extend(visualize_show_text(
        context, target.pos.map_pos, "attacked"));
    if killed > 0 {
        actions.extend(visualize_show_text(
            context, target.pos.map_pos, &format!("killed: {}", killed)));
    } else {
        actions.extend(visualize_show_text(
            context, target.pos.map_pos, "miss")); // TODO: check position
    }
    // TODO: вертолеты, прицепы?
    let target_node_id = context.scene.unit_id_to_node_id(target_id);
    if killed > 0 {
        // TODO: ActionMove (node)
        let children = &mut context.scene.node_mut(target_node_id).children;
        let killed = killed as usize;
        assert!(killed <= children.len());
        for i in 0 .. killed {
            if leave_wrecks {
                // TODO: ActionChangeColor
                children[i].color = WRECKS_COLOR;
            } else {
                let _ = children.remove(0);
            }
        }
    }
    let is_target_destroyed = target.count - killed <= 0;
    if is_target_destroyed {
        if target.attached_unit_id.is_some() {
            // TODO: Action???
            context.scene.node_mut(target_node_id).children.pop().unwrap();
        }
        // delete unit's marker
        context.scene.node_mut(target_node_id).children.pop().unwrap();
        if !leave_wrecks {
            // TODO: ActionRemoveNode??
            assert_eq!(context.scene.node(target_node_id).children.len(), 0);
            context.scene.remove_node(target_node_id);
        }
    }
    /*
    let mut text = String::new();
    text += match effect.effect {
        Effect::Immobilized => "Immobilized",
        Effect::WeaponBroken => "WeaponBroken",
        Effect::ReducedMovement => "ReducedMovement",
        Effect::ReducedAttackPoints => "ReducedAttackPoints",
        Effect::Pinned => "Pinned",
    };
    text += ": ";
    text += match effect.time {
        effect::Time::Forever => "Forever",
        // TODO: показать число ходов:
        effect::Time::Turns(_) => "Turns(n)",
        effect::Time::Instant => "Instant",
    };
    context.text.add_text(unit_pos, &text);
    */
    // TODO: визуализировать как-то
    actions
}

pub fn visualize_event_attack(
    state: &State,
    context: &mut ActionContext,
    attack_info: &AttackInfo,
) -> Vec<Box<Action>> {
    let mut actions = Vec::new();
    let target_pos = geom::exact_pos_to_world_pos(state, attack_info.target_pos);
    if let Some(attacker_id) = attack_info.attacker_id {
        let attacker_node_id = context.scene.unit_id_to_node_id(attacker_id);
        let attacker_pos = context.scene.node(attacker_node_id).pos;
        let attacker_map_pos = state.unit(attacker_id).pos.map_pos;
        if attack_info.mode == FireMode::Reactive {
            actions.extend(visualize_show_text(
                context, attacker_map_pos, "reaction fire"));
        }
        let node_id = context.scene.allocate_node_id();
        actions.push(Box::new(ActionCreateNode {
            node_id: node_id,
            node: SceneNode {
                pos: attacker_pos,
                rot: geom::get_rot_angle(attacker_pos, target_pos),
                mesh_id: Some(context.mesh_ids.shell_mesh_id),
                .. Default::default()
            },
        }) as Box<Action>);
        // TODO: simulate arc for inderect fire in ActionMove:
        // if attack_info.is_inderect {
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
        actions.push(Box::new(ActionSleep {
            duration: Time{n: 0.5},
            time: Time{n: 0.0},
        }));
    }
    if attack_info.is_ambush {
        actions.extend(visualize_show_text(
            context, attack_info.target_pos.map_pos, "Ambushed"));
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

    fn update(&mut self, _: &Context, scene: &mut Scene, dtime: Time) {
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

// TODO: try to remove this hack
// TODO: rename?
#[derive(Debug)]
pub struct ActionTryFixAttachedUnit {
    unit_id: UnitId,
    attached_unit_id: UnitId,
}

impl Action for ActionTryFixAttachedUnit {
    fn begin(&mut self, context: ActionContext) {
        let transporter_node_id = context.scene.unit_id_to_node_id(self.unit_id);
        let attached_unit_node_id
            = match context.scene.unit_id_to_node_id_opt(self.attached_unit_id)
        {
            Some(id) => id,
            // this unit's scene node is already
            // attached to transporter's scene node
            None => return,
        };
        let mut node = context.scene.node_mut(attached_unit_node_id)
            .children.remove(0);
        context.scene.remove_unit(self.attached_unit_id);
        node.pos.v.y = -0.5; // TODO: get from UnitTypeVisualInfo
        node.rot += Rad(PI);
        context.scene.node_mut(transporter_node_id).children.push(node);
        context.scene.node_mut(transporter_node_id).children[0].pos.v.y = 0.5;
    }
}

pub fn visualize_event_show(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let marker_mesh_id = context.mesh_ids.marker_mesh_id;
    let mesh_id = context.visual_info.get(unit.type_id).mesh_id;
    actions.extend(visualize_show_text(
        context, unit.pos.map_pos, "spotted"));
    let pos = geom::exact_pos_to_world_pos(state, unit.pos);
    let node_id = context.scene.allocate_node_id();
    actions.push(Box::new(ActionCreateUnit {
        pos: pos,
        node_id: node_id,
        unit: unit.clone(),
        mesh_id: mesh_id,
        marker_mesh_id: marker_mesh_id,
    }) as Box<Action>);
    if let Some(attached_unit_id) = unit.attached_unit_id {
        actions.push(Box::new(ActionTryFixAttachedUnit {
            unit_id: unit.id,
            attached_unit_id: attached_unit_id,
        }));
    }
    for unit in state.units_at(unit.pos.map_pos) {
        if let Some(attached_unit_id) = unit.attached_unit_id {
            actions.push(Box::new(ActionTryFixAttachedUnit {
                unit_id: unit.id,
                attached_unit_id: attached_unit_id,
            }));
        }
    }
    actions
}

pub fn visualize_event_hide(
    context: &mut ActionContext,
    unit_id: UnitId,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    // passenger doesn't have any scene node
    if let Some(node_id) = context.scene.unit_id_to_node_id_opt(unit_id) {
        // We can't read 'pos' from `state.unit(unit_id).pos`
        // because this unit may be in a fogged tile now
        // so State will filter him out.
        let world_pos = context.scene.node(node_id).pos;
        let map_pos = geom::world_pos_to_map_pos(world_pos);
        actions.push(Box::new(ActionRemoveUnit {
            unit_id: unit_id,
        }) as Box<Action>);
        actions.extend(visualize_show_text(context, map_pos, "lost"));
    }
    actions
}

pub fn visualize_event_unload(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
    transporter_pos: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let marker_mesh_id = context.mesh_ids.marker_mesh_id;
    let unit_visual_info = context.visual_info.get(unit.type_id);
    let mesh_id = context.visual_info.get(unit.type_id).mesh_id;
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = geom::exact_pos_to_world_pos(state, transporter_pos);
    let node_id = context.scene.allocate_node_id();
    actions.push(Box::new(ActionCreateUnit {
        pos: from,
        node_id: node_id,
        unit: unit.clone(),
        mesh_id: mesh_id,
        marker_mesh_id: marker_mesh_id,
    }) as Box<Action>);
    //
    // TODO: I need to use `node_id` here
    // let action_move = ActionMove::new(
    //     state, scene, unit.id, visual_info, unit.pos);
    //
    actions.push(Box::new(ActionMove {
        node_id: node_id,
        to: to,
        speed: unit_visual_info.move_speed,
        move_helper: None,
    }));
    actions.extend(visualize_show_text(
        context, unit.pos.map_pos, "unloaded"));
    // unit_node.rot = geom::get_rot_angle(from, to);
    actions
}

pub fn visualize_event_load(
    state: &State,
    context: &mut ActionContext,
    passenger_id: UnitId,
    transporter_pos: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let type_id = state.unit(passenger_id).type_id;
    let visual_info = context.visual_info.get(type_id);
    let passenger_pos = state.unit(passenger_id).pos;
    let to = geom::exact_pos_to_world_pos(state, transporter_pos);
    let unit_node_id = context.scene.unit_id_to_node_id(passenger_id);
    actions.push(Box::new(ActionMove {
        node_id: unit_node_id,
        to: to,
        speed: visual_info.move_speed,
        move_helper: None,
    }) as Box<Action>);
    actions.push(Box::new(ActionRemoveUnit{unit_id: passenger_id}));
    actions.extend(visualize_show_text(context, passenger_pos.map_pos, "loaded"));
    actions
}

pub fn visualize_event_set_reaction_fire_mode(
    state: &State,
    context: &mut ActionContext,
    unit_id: UnitId,
    mode: ReactionFireMode,
) -> Vec<Box<Action>> {
    let pos = state.unit(unit_id).pos.map_pos;
    let text = match mode {
        ReactionFireMode::Normal => "Normal fire mode",
        ReactionFireMode::HoldFire => "Hold fire",
    };
    visualize_show_text(context, pos, text)
}

pub fn visualize_event_sector_owner_changed(
    state: &State,
    context: &mut ActionContext,
    sector_id: SectorId,
    owner_id: Option<PlayerId>,
) -> Vec<Box<Action>> {
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
    let node_id = context.scene.sector_id_to_node_id(sector_id);
    // TODO: ActionChangeColor
    {
        let node = context.scene.node_mut(node_id); // TODO: ActionChangeColor
        node.color = color;
    }
    let sector = &state.sectors()[&sector_id];
    let pos = sector.center();
    let text = match owner_id {
        Some(id) => format!("Sector {}: owner changed: Player {}", sector_id.id, id.id),
        None => format!("Sector {}: owner changed: None", sector_id.id),
    };
    visualize_show_text(context, pos, &text)
}

pub fn visualize_event_victory_point(
    context: &mut ActionContext,
    pos: MapPos,
    count: i32,
) -> Vec<Box<Action>> {
    let text = format!("+{} VP!", count);
    // TODO: Sleep for 1 second
    visualize_show_text(context, pos, &text)
}

const SMOKE_ALPHA: f32 = 0.7;

pub fn visualize_event_smoke(
    context: &mut ActionContext,
    pos: MapPos,
    _: Option<UnitId>, // TODO
    object_id: ObjectId,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let smoke_mesh_id = context.mesh_ids.smoke_mesh_id;
    // TODO: show shell animation: ActionMove
    actions.extend(visualize_show_text(context, pos, "smoke"));
    let z_step = 0.45; // TODO: magic
    let mut node = SceneNode {
        pos: geom::map_pos_to_world_pos(pos),
        mesh_id: Some(smoke_mesh_id),
        node_type: SceneNodeType::Transparent,
        .. Default::default()
    };
    node.pos.v.z += z_step;
    node.rot += Rad(thread_rng().gen_range(0.0, PI * 2.0));
    context.scene.add_object(object_id, node.clone());
    node.pos.v.z += z_step;
    node.rot += Rad(thread_rng().gen_range(0.0, PI * 2.0));
    context.scene.add_object(object_id, node);
    actions.push(Box::new(EventSmokeVisualizer {
        time: Time{n: 0.0},
        duration: Time{n: 1.0},
        object_id: object_id,
    }));
    actions
}

#[derive(Debug)]
pub struct EventSmokeVisualizer {
    duration: Time,
    time: Time,
    object_id: ObjectId,
}

impl Action for EventSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn update(&mut self, _: &Context, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = self.time.n / self.duration.n;
        }
    }
}

pub fn visualize_event_remove_smoke(
    state: &State,
    context: &mut ActionContext,
    object_id: ObjectId,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let pos = state.objects()[&object_id].pos.map_pos;
    actions.push(Box::new(EventRemoveSmokeVisualizer {
        time: Time{n: 0.0},
        duration: Time{n: 1.0},
        object_id: object_id,
    }) as Box<Action>);
    actions.extend(visualize_show_text(context, pos, "smoke cleared"));
    actions
}

#[derive(Debug)]
pub struct EventRemoveSmokeVisualizer {
    duration: Time,
    time: Time,
    object_id: ObjectId,
}

impl Action for EventRemoveSmokeVisualizer {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > SMOKE_ALPHA
    }

    fn update(&mut self, _: &Context, scene: &mut Scene, dtime: Time) {
        self.time.n += dtime.n;
        let node_ids = scene.object_id_to_node_id(self.object_id).clone();
        for node_id in node_ids {
            let node = scene.node_mut(node_id);
            node.color[3] = SMOKE_ALPHA - self.time.n / self.duration.n;
        }
    }

    fn end(&mut self, _: &Context, scene: &mut Scene) {
        scene.remove_object(self.object_id);
    }
}

pub fn visualize_event_attach(
    state: &State,
    context: &mut ActionContext,
    transporter_id: UnitId,
    attached_unit_id: UnitId,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let transporter_type_id = state.unit(transporter_id).type_id;
    let visual_info = context.visual_info.get(transporter_type_id);
    let transporter = state.unit(transporter_id);
    let attached_unit = state.unit(attached_unit_id);
    // let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, attached_unit.pos);
    let text_pos = transporter.pos.map_pos;
    let transporter_node_id = context.scene.unit_id_to_node_id(transporter_id);
    actions.push(Box::new(ActionMove {
        node_id: transporter_node_id,
        to: to,
        speed: visual_info.move_speed,
        move_helper: None,
    }) as Box<Action>);
    actions.extend(visualize_show_text(
        context, text_pos, "attached"));
    actions.push(Box::new(ActionTryFixAttachedUnit {
        unit_id: transporter_id,
        attached_unit_id: attached_unit_id,
    }));
    actions.push(Box::new(ActionRotateTo {
        node_id: transporter_node_id,
        to: to,
    }) as Box<Action>);
    actions
}

pub fn visualize_event_detach(
    state: &State,
    context: &mut ActionContext,
    transporter_id: UnitId,
    pos: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let transporter = state.unit(transporter_id);
    let attached_unit_id = transporter.attached_unit_id.unwrap();
    let attached_unit = state.unit(attached_unit_id);
    let transporter_visual_info
        = context.visual_info.get(transporter.type_id);
    let attached_unit_mesh_id = context.visual_info
        .get(attached_unit.type_id).mesh_id;
    let attached_unit_node_id = context.scene.allocate_node_id();
    let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, pos);
    let transporter_node_id = context.scene.unit_id_to_node_id(transporter_id);
    let speed = transporter_visual_info.move_speed;
    actions.push(Box::new(ActionCreateUnit {
        unit: attached_unit.clone(),
        mesh_id: attached_unit_mesh_id,
        marker_mesh_id: context.mesh_ids.marker_mesh_id,
        pos: geom::exact_pos_to_world_pos(state, attached_unit.pos),
        node_id: attached_unit_node_id,
    }) as Box<Action>);
    actions.push(Box::new(ActionDetach {
        from: from,
        to: to,
        transporter_node_id: transporter_node_id,
    }));
    actions.push(Box::new(ActionMove {
        node_id: transporter_node_id,
        to: to,
        speed: speed,
        move_helper: None,
    }));
    actions.extend(visualize_show_text(context, pos.map_pos, "detached"));
    actions
}

#[derive(Debug)]
pub struct ActionDetach {
    from: WorldPos,
    to: WorldPos,
    transporter_node_id: NodeId,
    // attached_unit_id: UnitId,
}

impl Action for ActionDetach {
    fn begin(&mut self, context: ActionContext) {
        let transporter_node = context.scene.node_mut(self.transporter_node_id);
        transporter_node.rot = geom::get_rot_angle(self.from, self.to);
        transporter_node.children[0].pos.v.y = 0.0;
        transporter_node.children.pop();
    }
}
