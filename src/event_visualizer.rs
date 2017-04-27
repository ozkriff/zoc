use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Rad};
use core::game_state::{State};
use core::sector::{SectorId};
use core::effect::{self, TimedEffect, Effect};
use core::player::{PlayerId};
use core::event::{CoreEvent, Event, FireMode, AttackInfo, ReactionFireMode};
use core::unit::{Unit, UnitId};
use core::position::{ExactPos, MapPos};
use core::object::{ObjectId};
use types::{WorldPos, Time, Speed};
use geom;
use scene::{SceneNode, SceneNodeType};
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
            visualize_event_attach(
                state, context, transporter_id, attached_unit_id)
        },
        Event::Detach{transporter_id, to, ..} => {
            visualize_event_detach(
                state, context, transporter_id, to)
        },
        Event::SetReactionFireMode{unit_id, mode} => {
            visualize_event_set_reaction_fire_mode(
                state, context, unit_id, mode)
        },
        Event::SectorOwnerChanged{sector_id, new_owner_id} => {
            visualize_event_sector_owner_changed(
                state, context, sector_id, new_owner_id)
        }
        Event::VictoryPoint{pos, count, ..} => {
            visualize_event_victory_point(
                context, pos, count)
        }
        Event::Smoke{pos, unit_id, id} => {
            visualize_event_smoke(
                context, pos, unit_id, id)
        }
        Event::RemoveSmoke{id} => {
            visualize_event_remove_smoke(
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
        Effect::Attacked(ref e) => {
            actions.extend(action::visualize_effect_attacked(
                state, context, target_id, e));
        },
        // TODO: Implement the rest of the effects
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
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = WorldPos{v: to.v - geom::vec3_z(geom::HEX_EX_RADIUS / 2.0)};
    let node_id = context.scene.allocate_node_id();
    vec![
        Box::new(action::CreateUnit::new(unit.clone(), from, node_id)),
        Box::new(action::MoveTo::new(node_id, Speed{n: 2.0}, to)),
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
    let mut actions: Vec<Box<Action>> = vec![];
    let type_id = state.unit(unit_id).type_id;
    let visual_info = context.visual_info.get(type_id);
    let node_id = context.scene.unit_id_to_node_id(unit_id);
    let to = geom::exact_pos_to_world_pos(state, destination);
    let speed = visual_info.move_speed;
    actions.push(Box::new(action::RotateTo::new(node_id, to)));
    actions.push(Box::new(action::MoveTo::new(node_id, speed, to)));
    actions
}

fn visualize_event_attack(
    state: &State,
    context: &mut ActionContext,
    attack_info: &AttackInfo,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = Vec::new();
    let target_pos = geom::exact_pos_to_world_pos(
        state, attack_info.target_pos);
    if let Some(attacker_id) = attack_info.attacker_id {
        let attacker_pos = state.unit(attacker_id).pos;
        let attacker_world_pos = geom::exact_pos_to_world_pos(
            state, attacker_pos);
        if attack_info.mode == FireMode::Reactive {
            actions.extend(action::visualize_show_text(
                context, attacker_pos.map_pos, "reaction fire"));
        }
        let node_id = context.scene.allocate_node_id();
        let node = SceneNode {
            pos: attacker_world_pos,
            rot: geom::get_rot_angle(attacker_world_pos, target_pos),
            mesh_id: Some(context.mesh_ids.shell_mesh_id),
            .. Default::default()
        };
        actions.push(Box::new(action::CreateNode::new(node_id, node)));
        // TODO: simulate arc for inderect fire in Move:
        // if attack_info.is_inderect {
        //     pos.v.z += (shell_move.progress() * PI).sin() * 5.0;
        // }
        actions.push(Box::new(action::MoveTo::new(node_id, Speed{n: 10.0}, target_pos)));
        actions.push(Box::new(action::RemoveNode::new(node_id)));
        actions.push(Box::new(action::Sleep::new(Time{n: 0.5})));
    }
    if attack_info.is_ambush {
        actions.extend(action::visualize_show_text(
            context, attack_info.target_pos.map_pos, "Ambushed"));
    };
    actions
}

fn visualize_event_show(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    let pos = geom::exact_pos_to_world_pos(state, unit.pos);
    let node_id = context.scene.allocate_node_id();
    actions.push(Box::new(
        action::CreateUnit::new(unit.clone(), pos, node_id)));
    if let Some(attached_unit_id) = unit.attached_unit_id {
        actions.push(Box::new(
            action::TryFixAttachedUnit::new(unit.id, attached_unit_id)));
    }
    for unit in state.units_at(unit.pos.map_pos) {
        if let Some(attached_unit_id) = unit.attached_unit_id {
            actions.push(Box::new(action::TryFixAttachedUnit::new(
                unit.id, attached_unit_id)));
        }
    }
    actions.extend(action::visualize_show_text(
        context, unit.pos.map_pos, "spotted"));
    actions
}

fn visualize_event_hide(
    context: &mut ActionContext,
    unit_id: UnitId,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    // passenger doesn't have any scene node
    if let Some(_ /*node_id*/) = context.scene.unit_id_to_node_id_opt(unit_id) { // TODO
        // We can't read 'pos' from `state.unit(unit_id).pos`
        // because this unit may be in a fogged tile now
        // so State will filter him out.
        //
        // Но и из узла прямо сейчас такое читать тоже нельзя,
        // потому что позиция узла будет действительной только
        // на момент `Action::begin`!
        //
        // Я хз что делать.
        //
        // Все что приходит в голову - вместо MapPos иметь возможность передать
        // NodeId, у которого в момент исполнения уже и будет взята позиция.
        //
        // TODO: Read the position in begin action!
        //
        // TODO: disabled for now (не забудь починить)
        // let world_pos = context.scene.node(node_id).pos;
        // let map_pos = geom::world_pos_to_map_pos(world_pos);
        actions.push(Box::new(action::RemoveUnit::new(unit_id)));
        // actions.extend(action::visualize_show_text(context, map_pos, "lost"));
    }
    actions
}

fn visualize_event_unload(
    state: &State,
    context: &mut ActionContext,
    unit: &Unit,
    transporter_pos: ExactPos,
) -> Vec<Box<Action>> {
    let unit = unit.clone();
    let visual_info = context.visual_info.get(unit.type_id);
    let to = geom::exact_pos_to_world_pos(state, unit.pos);
    let from = geom::exact_pos_to_world_pos(state, transporter_pos);
    let node_id = context.scene.allocate_node_id();
    let speed = visual_info.move_speed;
    let text_pos = unit.pos.map_pos;
    let mut actions: Vec<Box<Action>> = vec![];
    actions.push(Box::new(action::CreateUnit::new(unit, from, node_id)));
    actions.push(Box::new(action::RotateTo::new(node_id, to)));
    actions.push(Box::new(action::MoveTo::new(node_id, speed, to)));
    actions.extend(action::visualize_show_text(context, text_pos, "unloaded"));
    actions
}

fn visualize_event_load(
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
    let mut actions: Vec<Box<Action>> = vec![];
    actions.push(Box::new(action::RotateTo::new(node_id, to)));
    actions.push(Box::new(action::MoveTo::new(node_id, speed, to)));
    actions.push(Box::new(action::RemoveUnit::new(passenger_id)));
    actions.extend(action::visualize_show_text(context, text_pos, "loaded"));
    actions
}

fn visualize_event_set_reaction_fire_mode(
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

fn visualize_event_victory_point(
    context: &mut ActionContext,
    pos: MapPos,
    count: i32,
) -> Vec<Box<Action>> {
    let text = format!("+{} VP!", count);
    // TODO: Sleep for 1 second
    action::visualize_show_text(context, pos, &text)
}

fn visualize_event_smoke(
    context: &mut ActionContext,
    pos: MapPos,

    // TODO: I would be glad to show shell from the unit,
    // BUT there should be only one shell for multiple events...
    //
    // Should I convert EventSmoke to effect? What whould be the event then? 
    _: Option<UnitId>, // TODO

    object_id: ObjectId,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    let smoke_mesh_id = context.mesh_ids.smoke_mesh_id;
    // TODO: show shell animation: MoveTo
    actions.extend(action::visualize_show_text(context, pos, "smoke"));
    let z_step = 0.30; // TODO: magic
    let mut node = SceneNode {
        pos: geom::map_pos_to_world_pos(pos),
        mesh_id: Some(smoke_mesh_id),
        node_type: SceneNodeType::Transparent,
        color: [1.0, 1.0, 1.0, 0.0],
        .. Default::default()
    };
    let final_color = [1.0, 1.0, 1.0, 0.7];
    let time = Time{n: 0.5};
    for _ in 0..2 {
        node.pos.v.z += z_step;
        node.rot += Rad(thread_rng().gen_range(0.0, PI * 2.0));
        let node_id = context.scene.allocate_node_id();
        actions.push(Box::new(action::AddObject::new(
            object_id, node.clone(), node_id)));
        actions.push(Box::new(action::ChangeColor::new(
            node_id, final_color, time)));
    }
    actions
}

fn visualize_event_remove_smoke(
    state: &State,
    context: &mut ActionContext,
    object_id: ObjectId,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    let pos = state.objects()[&object_id].pos.map_pos;
    let node_ids = context.scene.object_id_to_node_id(object_id).clone();
    let final_color = [1.0, 1.0, 1.0, 0.0];
    let time = Time{n: 0.5};
    for node_id in node_ids {
        actions.push(Box::new(action::ChangeColor::new(
            node_id, final_color, time)));
    }
    actions.push(Box::new(action::RemoveObject::new(object_id)));
    actions.extend(action::visualize_show_text(context, pos, "smoke cleared"));
    actions
}

fn visualize_event_attach(
    state: &State,
    context: &mut ActionContext,
    transporter_id: UnitId,
    attached_unit_id: UnitId,
) -> Vec<Box<Action>> {
    let transporter_type_id = state.unit(transporter_id).type_id;
    let visual_info = context.visual_info.get(transporter_type_id);
    let transporter = state.unit(transporter_id);
    let attached_unit = state.unit(attached_unit_id);
    // let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, attached_unit.pos);
    let text_pos = transporter.pos.map_pos;
    let transporter_node_id = context.scene.unit_id_to_node_id(transporter_id);
    let speed = visual_info.move_speed;
    let mut actions: Vec<Box<Action>> = vec![];
    actions.push(Box::new(action::RotateTo::new(
        transporter_node_id, to)));
    actions.push(Box::new(action::MoveTo::new(
        transporter_node_id, speed, to)));
    actions.push(Box::new(action::TryFixAttachedUnit::new(
        transporter_id, attached_unit_id)));
    actions.extend(action::visualize_show_text(
        context, text_pos, "attached"));
    actions
}

fn visualize_event_detach(
    state: &State,
    context: &mut ActionContext,
    transporter_id: UnitId,
    pos: ExactPos,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
    let transporter = state.unit(transporter_id);
    let attached_unit_id = transporter.attached_unit_id.unwrap();
    let attached_unit = state.unit(attached_unit_id);
    let transporter_visual_info
        = context.visual_info.get(transporter.type_id);
    let attached_unit_node_id = context.scene.allocate_node_id();
    let from = geom::exact_pos_to_world_pos(state, transporter.pos);
    let to = geom::exact_pos_to_world_pos(state, pos);
    let transporter_node_id = context.scene.unit_id_to_node_id(transporter_id);
    let speed = transporter_visual_info.move_speed;
    actions.push(Box::new(action::CreateUnit::new(
        attached_unit.clone(),
        geom::exact_pos_to_world_pos(state, attached_unit.pos),
        attached_unit_node_id,
    )));
    actions.push(Box::new(action::Detach::new_from_to(
        transporter_node_id, from, to)));
    // TODO: action::RotateTo?
    actions.push(Box::new(action::MoveTo::new(
        transporter_node_id, speed, to)));
    actions.extend(action::visualize_show_text(
        context, pos.map_pos, "detached"));
    actions
}

fn visualize_event_sector_owner_changed(
    state: &State,
    context: &mut ActionContext,
    sector_id: SectorId,
    owner_id: Option<PlayerId>,
) -> Vec<Box<Action>> {
    let mut actions: Vec<Box<Action>> = vec![];
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
    actions.push(Box::new(action::SetColor::new(node_id, color)));
    let sector = &state.sectors()[&sector_id];
    let pos = sector.center();
    let text = match owner_id {
        Some(id) => format!("Sector {}: owner changed: Player {}", sector_id.id, id.id),
        None => format!("Sector {}: owner changed: None", sector_id.id),
    };
    actions.extend(action::visualize_show_text(context, pos, &text));
    actions
}
