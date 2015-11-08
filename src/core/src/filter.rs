// See LICENSE file for copyright and license details.

use std::collections::{HashMap, HashSet};
use common::types::{UnitId, PlayerId};
use internal_state::{InternalState};
use state::{State};
use pathfinder::{MapPath, PathNode, MoveCost};
use unit::{Unit};
use db::{Db};
use fow::{Fow};
use ::{MoveMode, CoreEvent, AttackInfo, UnitInfo, unit_to_info};

pub fn get_visible_enemies(
    db: &Db,
    state: &InternalState,
    fow: &Fow,
    player_id: &PlayerId,
) -> HashSet<UnitId> {
    let mut visible_enemies = HashSet::new();
    for (id, unit) in state.units() {
        if unit.player_id != *player_id
            && fow.is_visible(db, state, unit, &unit.pos)
        {
            visible_enemies.insert(id.clone());
        }
    }
    visible_enemies
}

pub fn show_or_hide_passive_enemies(
    units: &HashMap<UnitId, Unit>,
    active_unit_ids: &HashSet<UnitId>,
    old: &HashSet<UnitId>,
    new: &HashSet<UnitId>,
) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let located_units = new.difference(old);
    for id in located_units {
        if active_unit_ids.contains(id) {
            continue;
        }
        let unit = units.get(&id).expect("Can`t find unit");
        events.push(CoreEvent::ShowUnit {
            unit_info: unit_to_info(unit),
        });
    }
    let lost_units = old.difference(new);
    for id in lost_units {
        if active_unit_ids.contains(id) {
            continue;
        }
        events.push(CoreEvent::HideUnit{unit_id: id.clone()});
    }
    events
}

fn filter_move_event(
    db: &Db,
    state: &InternalState,
    fow: &Fow,
    unit_id: &UnitId,
    path: &MapPath,
    move_mode: &MoveMode,
) -> Vec<CoreEvent> {
    let mut events = vec![];
    let unit = state.unit(unit_id);
    let len = path.nodes().len();
    let mut sub_path = Vec::new();
    let first_pos = path.nodes()[0].pos.clone();
    if fow.is_visible(db, state, unit, &first_pos) {
        sub_path.push(PathNode {
            cost: MoveCost{n: 0},
            pos: first_pos,
        });
    }
    for i in 1 .. len {
        let prev_node = path.nodes()[i - 1].clone();
        let next_node = path.nodes()[i].clone();
        let prev_vis = fow.is_visible(db, state, unit, &prev_node.pos);
        let next_vis = fow.is_visible(db, state, unit, &next_node.pos);
        if !prev_vis && next_vis {
            events.push(CoreEvent::ShowUnit {
                unit_info: UnitInfo {
                    pos: prev_node.pos.clone(),
                    .. unit_to_info(unit)
                },
            });
            sub_path.push(PathNode {
                cost: MoveCost{n: 0},
                pos: prev_node.pos.clone(),
            });
        }
        if prev_vis || next_vis {
            sub_path.push(PathNode {
                cost: MoveCost{n: 0},
                pos: next_node.pos.clone(),
            });
        }
        if prev_vis && !next_vis {
            events.push(CoreEvent::Move {
                unit_id: unit.id.clone(),
                path: MapPath::new(sub_path.clone()),
                mode: move_mode.clone(),
            });
            sub_path.clear();
            events.push(CoreEvent::HideUnit {
                unit_id: unit.id.clone(),
            });
        }
    }
    if sub_path.len() != 0 {
        events.push(CoreEvent::Move {
            unit_id: unit.id.clone(),
            path: MapPath::new(sub_path),
            mode: move_mode.clone(),
        });
    }
    events
}

pub fn filter_events(
    db: &Db,
    state: &InternalState,
    player_id: &PlayerId,
    fow: &Fow,
    event: &CoreEvent,
) -> (Vec<CoreEvent>, HashSet<UnitId>) {
    let mut active_unit_ids = HashSet::new();
    let mut events = vec![];
    match event {
        &CoreEvent::Move{ref unit_id, ref path, ref mode} => {
            let unit = state.unit(unit_id);
            if unit.player_id == *player_id {
                events.push(event.clone())
            } else {
                let filtered_events = filter_move_event(
                    db, state, fow, unit_id, path, mode);
                events.extend(filtered_events);
                active_unit_ids.insert(unit_id.clone());
            }
        },
        &CoreEvent::EndTurn{..} => {
            events.push(event.clone());
        },
        &CoreEvent::CreateUnit{ref unit_info} => {
            let unit = state.unit(&unit_info.unit_id);
            if *player_id == unit_info.player_id
                || fow.is_visible(db, state, unit, &unit_info.pos)
            {
                events.push(event.clone());
                active_unit_ids.insert(unit_info.unit_id.clone());
            }
        },
        &CoreEvent::AttackUnit{ref attack_info} => {
            let attacker_id = attack_info.attacker_id.clone()
                .expect("Core must know about everything");
            let attacker = state.unit(&attacker_id);
            if *player_id != attacker.player_id && !attack_info.is_ambush {
                // show attacker if this is not ambush
                let attacker = state.unit(&attacker_id);
                if !fow.is_visible(db, state, attacker, &attacker.pos) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: unit_to_info(&attacker),
                    });
                }
                active_unit_ids.insert(attacker_id.clone());
            }
            active_unit_ids.insert(attack_info.defender_id.clone()); // if defender is killed
            let is_attacker_visible = *player_id == attacker.player_id
                || !attack_info.is_ambush;
            let attack_info = AttackInfo {
                attacker_id: if is_attacker_visible {
                    Some(attacker_id)
                } else {
                    None
                },
                .. attack_info.clone()
            };
            events.push(CoreEvent::AttackUnit{attack_info: attack_info});
        },
        &CoreEvent::ShowUnit{..} => panic!(),
        &CoreEvent::HideUnit{..} => panic!(),
        &CoreEvent::LoadUnit{ref passanger_id, ..} => {
            let passanger = state.unit(passanger_id);
            if passanger.player_id == *player_id {
                events.push(event.clone());
            } else if fow.is_visible(db, state, passanger, &passanger.pos) {
                events.push(event.clone());
            }
        },
        &CoreEvent::UnloadUnit{ref unit_info, ref transporter_id} => {
            active_unit_ids.insert(unit_info.unit_id.clone());
            let passanger = state.unit(&unit_info.unit_id);
            if passanger.player_id == *player_id {
                events.push(event.clone());
            } else if fow.is_visible(db, state, passanger, &unit_info.pos) {
                let transporter = state.unit(transporter_id);
                if !fow.is_visible(db, state, transporter, &transporter.pos) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: unit_to_info(transporter),
                    });
                    active_unit_ids.insert(transporter_id.clone());
                }
                events.push(event.clone());
            }
        },
        &CoreEvent::SetReactionFireMode{ref unit_id, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == *player_id {
                events.push(event.clone());
            }
        },
    }
    (events, active_unit_ids)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
