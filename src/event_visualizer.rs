use core::game_state::{State};
use core::effect::{self, TimedEffect, Effect};
use core::event::{CoreEvent, Event, FireMode, AttackInfo, ReactionFireMode};
use core::unit::{Unit, UnitId};
use core::position::{ExactPos};
use types::{WorldPos, Time, Speed};
use geom;
use scene::{/*Scene,*/ SceneNode, /*SceneNodeType, NodeId*/};
use action::{self, Action, ActionContext};

// TODO: Make this a standalone function and don't pass `&mut Scene` to
// Action c-tors. There're `update` and `end` methods to do this.
// Maybe I should add a `start` method to `Action` trait.
//
// Actually, I need `&mut Scene` for calling `allocate_node_id` :(
//
pub fn visualize_event(
    state: &State,
    context: &mut ActionContext,
    event: &CoreEvent,
) -> Vec<Box<Action>> {
    println!("visualize_event: event: {:?}\n", event);
    let mut actions = match event.event {
        Event::Move{unit_id, to, ..} => {
            visualize_event_move(state, context, unit_id, to)
        },
        Event::EndTurn{..} => Vec::new(),
        Event::CreateUnit{ref unit_info} => {
            visualize_event_create_unit(state, context, unit_info)
        },
        Event::AttackUnit{ref attack_info} => {
            visualize_event_attack(state, context, attack_info)
        },
        Event::ShowUnit{ref unit_info, ..} => {
            visualize_event_show(state, context, unit_info)
        },
        Event::HideUnit{unit_id} => {
            visualize_event_hide(context, unit_id)
        },
        Event::LoadUnit{passenger_id, to, ..} => {
            visualize_event_load(state, context, passenger_id, to)
        },
        Event::UnloadUnit{ref unit_info, from, ..} => {
            visualize_event_unload(state, context, unit_info, from)
        },
        Event::Attach{transporter_id, attached_unit_id, ..} => {
            action::visualize_event_attach(
                state, context, transporter_id, attached_unit_id)
        },
        Event::Detach{transporter_id, to, ..} => {
            action::visualize_event_detach(
                state, context, transporter_id, to)
        },
        Event::SetReactionFireMode{unit_id, mode} => {
            visualize_event_set_reaction_fire_mode(
                state, context, unit_id, mode)
        },
        Event::SectorOwnerChanged{sector_id, new_owner_id} => {
            action::visualize_event_sector_owner_changed(
                state, context, sector_id, new_owner_id)
        }
        Event::VictoryPoint{pos, count, ..} => {
            action::visualize_event_victory_point(
                context, pos, count)
        }
        Event::Smoke{pos, unit_id, id} => {
            action::visualize_event_smoke(
                context, pos, unit_id, id)
        }
        Event::RemoveSmoke{id} => {
            action::visualize_event_remove_smoke(
                state, context, id)
        }
        Event::Reveal{..} => unreachable!(),
    };
    actions.extend(visualize_effects(state, context, event));
    actions
}
fn visualize_effect(
    state: &State,
    context: &mut ActionContext,
    _: &CoreEvent, // TODO: Do I actually need it?
    target_id: UnitId,
    effect: &TimedEffect,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    if effect.time != effect::Time::Instant {
        println!("visualize_event: long effect"); // TODO: remove print
        return vec![];
    }
    match effect.effect {
        Effect::Attacked {
            killed,
            // suppression, // TODO: print suppression
            leave_wrecks,
            // remove_move_points,
            ..
        } => {
            actions.extend(action::visualize_effect_attacked(
                state,
                context,
                target_id,
                killed,
                leave_wrecks,
            ));
        },
        // TODO: Implement rest of the effects
        Effect::Immobilized => {},
        Effect::WeaponBroken => {},
        Effect::ReducedMovementPoints(_) => {},
        Effect::ReducedAttackPoints(_) => {},
        Effect::Pinned => {},
        Effect::ReducedAccuracy(_) => {},
        Effect::Suppressed(_) => {},
        Effect::SoldierKilled(_) => {},
        Effect::VehicleDestroyed => {},
    }
    actions
}

fn visualize_event_create_unit(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
) -> Vec<Box<Action>> {
    let mesh_id = context.visual_info
        .get(unit.type_id).mesh_id;
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = WorldPos{v: to.v - geom::vec3_z(geom::HEX_EX_RADIUS / 2.0)};
    let node_id = context.scene.allocate_node_id();
    vec![
        action::CreateUnit::new(unit.clone(), mesh_id, from, node_id),
        action::MoveTo::new(node_id, Speed{n: 2.0}, to),
    ]
}

fn visualize_effects(
    state: &State,
    context: &mut action::ActionContext,
    event: &CoreEvent,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    //
    // TODO: How should I visualize delayed effects?
    // Should I show some icon above the unit?
    //
    for (&target_id, target_effects) in &event.effects {
        println!("visualize_event: effect <");
        // let target = state.unit(target_id);
        for effect in target_effects {
            actions.extend(visualize_effect(
                state, context, event, target_id, effect));
        }
    }
    actions
}

fn visualize_event_move(
    state: &State,
    context: &mut action::ActionContext,
    unit_id: UnitId,
    destination: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions = Vec::new();
    let type_id = state.unit(unit_id).type_id;
    let visual_info = context.visual_info.get(type_id);
    let node_id = context.scene.unit_id_to_node_id(unit_id);
    let to = geom::exact_pos_to_world_pos(state, destination);
    let speed = visual_info.move_speed;
    actions.push(action::RotateTo::new(node_id, to));
    actions.push(action::MoveTo::new(node_id, speed, to));
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

        // TODO: do this in some begin method?
        let attacker_node_id = context.scene.unit_id_to_node_id(attacker_id);

        let attacker_pos = context.scene.node(attacker_node_id).pos;
        let attacker_map_pos = state.unit(attacker_id).pos.map_pos;
        if attack_info.mode == FireMode::Reactive {
            actions.extend(action::visualize_show_text(
                context, attacker_map_pos, "reaction fire"));
        }
        let node_id = context.scene.allocate_node_id();
        let node = SceneNode {
            pos: attacker_pos,
            rot: geom::get_rot_angle(attacker_pos, target_pos),
            mesh_id: Some(context.mesh_ids.shell_mesh_id),
            .. Default::default()
        };
        actions.push(action::CreateNode::new(node_id, node));
        // TODO: simulate arc for inderect fire in Move:
        // if attack_info.is_inderect {
        //     pos.v.z += (shell_move.progress() * PI).sin() * 5.0;
        // }
        actions.push(action::MoveTo::new(node_id, Speed{n: 10.0}, target_pos));
        actions.push(action::RemoveNode::new(node_id));
        actions.push(action::Sleep::new(Time{n: 0.5}));
    }
    if attack_info.is_ambush {
        actions.extend(action::visualize_show_text(
            context, attack_info.target_pos.map_pos, "Ambushed"));
    };
    actions
}

pub fn visualize_event_show(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
) -> Vec<Box<Action>> {
    let mut actions = vec![];
    let mesh_id = context.visual_info.get(unit.type_id).mesh_id;
    let pos = geom::exact_pos_to_world_pos(state, unit.pos);
    let node_id = context.scene.allocate_node_id();
    actions.push(action::CreateUnit::new(
        unit.clone(), mesh_id, pos, node_id));
    if let Some(attached_unit_id) = unit.attached_unit_id {
        actions.push(action::TryFixAttachedUnit::new(
            unit.id, attached_unit_id));
    }
    for unit in state.units_at(unit.pos.map_pos) {
        if let Some(attached_unit_id) = unit.attached_unit_id {
            actions.push(action::TryFixAttachedUnit::new(
                unit.id, attached_unit_id));
        }
    }
    actions.extend(action::visualize_show_text(
        context, unit.pos.map_pos, "spotted"));
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
        actions.push(action::RemoveUnit::new(unit_id));
        actions.extend(action::visualize_show_text(context, map_pos, "lost"));
    }
    actions
}

pub fn visualize_event_unload(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
    transporter_pos: ExactPos,
) -> Vec<Box<Action>> {
    let unit = unit.clone();
    let visual_info = context.visual_info.get(unit.type_id);
    let mesh_id = context.visual_info.get(unit.type_id).mesh_id;
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = geom::exact_pos_to_world_pos(state, transporter_pos);
    let node_id = context.scene.allocate_node_id();
    let speed = visual_info.move_speed;
    let text_pos = unit.pos.map_pos;
    let mut actions = vec![];
    actions.push(action::CreateUnit::new(unit, mesh_id, from, node_id));
    actions.push(action::RotateTo::new(node_id, to));
    actions.push(action::MoveTo::new(node_id, speed, to));
    actions.extend(action::visualize_show_text(context, text_pos, "unloaded"));
    actions
}

pub fn visualize_event_load(
    state: &State,
    context: &mut ActionContext,
    passenger_id: UnitId,
    transporter_pos: ExactPos,
) -> Vec<Box<Action>> {
    let passenger = state.unit(passenger_id);
    let type_id = passenger.type_id;
    let visual_info = context.visual_info.get(type_id);
    let text_pos = passenger.pos.map_pos;
    let to = geom::exact_pos_to_world_pos(state, transporter_pos);
    let node_id = context.scene.unit_id_to_node_id(passenger_id);
    let speed = visual_info.move_speed;
    let mut actions = vec![];
    actions.push(action::RotateTo::new(node_id, to));
    actions.push(action::MoveTo::new(node_id, speed, to));
    actions.push(action::RemoveUnit::new(passenger_id));
    actions.extend(action::visualize_show_text(context, text_pos, "loaded"));
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
    action::visualize_show_text(context, pos, text)
}
