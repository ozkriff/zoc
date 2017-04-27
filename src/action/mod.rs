use std::fmt::{Debug};
use core::game_state::{State};
use core::unit::{UnitId};
use core::position::{MapPos};
use core::effect;
use types::{WorldPos, Time, Speed};
use geom;
use context::{Context};
use scene::{Scene, SceneNode, SceneNodeType};
use unit_type_visual_info::{UnitTypeVisualInfoManager};
use mesh_manager::{MeshIdManager, MeshManager};
use camera::{Camera};

mod remove_child;
mod add_object;
mod remove_object;
mod create_unit;
mod remove_unit;
mod remove_mesh;
mod sleep;
mod rotate_to;
mod set_color;
mod change_color;
mod move_to;
mod try_fix_attached_unit;
mod detach;
mod create_text_mesh;
mod create_node;
mod remove_node;

pub use self::remove_child::RemoveChild;
pub use self::add_object::AddObject;
pub use self::remove_object::RemoveObject;
pub use self::create_unit::CreateUnit;
pub use self::remove_unit::RemoveUnit;
pub use self::remove_mesh::RemoveMesh;
pub use self::sleep::Sleep;
pub use self::rotate_to::RotateTo;
pub use self::set_color::SetColor;
pub use self::change_color::ChangeColor;
pub use self::move_to::MoveTo;
pub use self::try_fix_attached_unit::TryFixAttachedUnit;
pub use self::detach::Detach;
pub use self::create_text_mesh::CreateTextMesh;
pub use self::create_node::CreateNode;
pub use self::remove_node::RemoveNode;

// TODO: Move to some other place
pub const WRECKS_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];

// TODO: RENAME
// TODO: Move to tactical_screen.rs?
pub struct ActionContext<'a> {
    pub camera: &'a Camera,
    pub mesh_ids: &'a MeshIdManager,
    pub scene: &'a mut Scene,
    pub context: &'a mut Context,
    pub meshes: &'a mut MeshManager,
    pub visual_info: &'a UnitTypeVisualInfoManager,

    // TODO: pub state: &State, // ???
}

pub trait Action: Debug {
    fn is_finished(&self) -> bool { true }

    // TODO: I'm not sure that `begin\end` must mutate the scene
    // TODO: Can I get rid of begin and end somehow? Should I?
    fn begin(&mut self, _: &mut ActionContext) {}
    fn update(&mut self, _: &mut ActionContext, _: Time) {}
    fn end(&mut self, _: &mut ActionContext) {}
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
//
// TODO: is a visualize_** function? In what file should I put it?
//
// TODO: action::Chain?
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
    let node = SceneNode {
        pos: from,
        rot: context.camera.get_z_angle(), // TODO: !?
        mesh_id: Some(mesh_id),
        color: [0.0, 0.0, 1.0, 1.0],
        node_type: SceneNodeType::Transparent,
        .. Default::default()
    };
    vec![
        Box::new(CreateTextMesh::new(text.into(), mesh_id)),
        Box::new(CreateNode::new(node_id, node)),
        Box::new(MoveTo::new(node_id, Speed{n: 1.0}, to)),
        Box::new(RemoveNode::new(node_id)),
        Box::new(RemoveMesh::new(mesh_id)),
    ]
}

// TODO: split this effect into many
// TODO: move to event_visualizer.rs
pub fn visualize_effect_attacked(
    state: &State,
    context: &mut ActionContext,
    target_id: UnitId,
    effect: &effect::Attacked,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    let target = state.unit(target_id);
    if effect.killed > 0 {
        actions.extend(visualize_show_text(
            context, target.pos.map_pos, &format!("killed: {}", effect.killed)));
    } else {
        actions.extend(visualize_show_text(
            context, target.pos.map_pos, "miss")); // TODO: check position
    }
    // TODO: helicopters?
    // TODO: loaded units?
    // TODO: attached units?
    let target_node_id = context.scene.unit_id_to_node_id(target_id);
    if effect.killed > 0 {
        let children = context.scene.node_mut(target_node_id)
            .children.clone(); // TODO: remove clone
        let killed = effect.killed as usize;
        assert!(killed <= children.len());
        for i in 0 .. killed {
            if effect.leave_wrecks {
                actions.push(Box::new(SetColor::new(
                    children[i], WRECKS_COLOR)));
            } else {
                {
                    let pos = context.scene.node(children[i]).pos;
                    let to = WorldPos{v: pos.v - geom::vec3_z(geom::HEX_EX_RADIUS / 2.0)};
                    actions.push(Box::new(MoveTo::new(
                        children[i], Speed{n: 1.0}, to)));
                }
                actions.push(Box::new(RemoveChild::new(
                    target_node_id, 0)));
            }
        }
        let is_target_destroyed = target.count - effect.killed <= 0;
        if is_target_destroyed {
            if target.attached_unit_id.is_some() {
                actions.push(Box::new(RemoveChild::new(
                    target_node_id, 0)));
            }
            let marker_child_id = if effect.leave_wrecks {
                children.len() as i32
            } else {
                (children.len() - killed) as i32
            } - 1;
            actions.push(Box::new(RemoveChild::new(
                target_node_id, marker_child_id)));
            if !effect.leave_wrecks {
                // assert_eq!(children.len(), 0); // TODO: how can i check this now?
                actions.push(Box::new(RemoveUnit::new(target_id)));
            }
        } else {
            actions.extend(visualize_show_text(
                context,
                target.pos.map_pos,
                &format!("morale: -{}", effect.suppression),
            ));
        }
    }
    let is_target_suppressed = target.morale >= 50
        && target.morale - effect.suppression < 50;
    if is_target_suppressed {
        actions.extend(visualize_show_text(
            context, target.pos.map_pos, "suppressed"));
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
