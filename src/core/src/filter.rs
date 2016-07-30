// See LICENSE file for copyright and license details.

use std::collections::{HashMap, HashSet};
use internal_state::{InternalState};
use game_state::{GameState};
use unit::{Unit};
use db::{Db};
use fow::{Fow};
use ::{CoreEvent, AttackInfo, UnitInfo, UnitId, PlayerId, unit_to_info};

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
        let unit = units.get(id).expect("Can`t find unit");
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

// TODO: join state and fow into TmpPartialState
pub fn filter_events(
    db: &Db,
    state: &InternalState,
    player_id: &PlayerId,
    fow: &Fow,
    event: &CoreEvent,
) -> (Vec<CoreEvent>, HashSet<UnitId>) {
    let mut active_unit_ids = HashSet::new();
    let mut events = vec![];
    match *event {
        CoreEvent::Move{ref unit_id, ref from, ref to, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == *player_id {
                events.push(event.clone())
            } else {
                let prev_vis = fow.is_visible(db, state, unit, from);
                let next_vis = fow.is_visible(db, state, unit, to);
                if !prev_vis && next_vis {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: UnitInfo {
                            pos: from.clone(),
                            .. unit_to_info(unit)
                        },
                    });
                }
                if prev_vis || next_vis {
                    events.push(event.clone());
                }
                if prev_vis && !next_vis {
                    events.push(CoreEvent::HideUnit {
                        unit_id: unit.id.clone(),
                    });
                }
                active_unit_ids.insert(unit_id.clone());
            }
        },
        CoreEvent::EndTurn{..} => {
            events.push(event.clone());
        },
        CoreEvent::CreateUnit{ref unit_info} => {
            let unit = state.unit(&unit_info.unit_id);
            if *player_id == unit_info.player_id
                || fow.is_visible(db, state, unit, &unit_info.pos)
            {
                events.push(event.clone());
                active_unit_ids.insert(unit_info.unit_id.clone());
            }
        },
        CoreEvent::AttackUnit{ref attack_info} => {
            let attacker_id = attack_info.attacker_id.clone()
                .expect("Core must know about everything");
            let attacker = state.unit(&attacker_id);
            if *player_id != attacker.player_id && !attack_info.is_ambush {
                // show attacker if this is not ambush
                let attacker = state.unit(&attacker_id);
                if !fow.is_visible(db, state, attacker, &attacker.pos) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: unit_to_info(attacker),
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
        CoreEvent::ShowUnit{..} => panic!(),
        CoreEvent::HideUnit{..} => panic!(),
        CoreEvent::LoadUnit{ref passenger_id, ref from, ref to, ref transporter_id} => {
            let passenger = state.unit(passenger_id);
            let transporter = state.unit(transporter_id.as_ref().unwrap());
            let is_transporter_vis = fow.is_visible(
                db, state, transporter, &transporter.pos);
            let is_passenger_vis = fow.is_visible(
                db, state, passenger, from);
            if passenger.player_id == *player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                if !fow.is_visible(db, state, passenger, from) {
                    events.push(CoreEvent::ShowUnit {
                        unit_info: UnitInfo {
                            pos: from.clone(),
                            .. unit_to_info(passenger)
                        },
                    });
                }
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id.clone()
                } else {
                    None
                };
                events.push(CoreEvent::LoadUnit {
                    transporter_id: filtered_transporter_id,
                    passenger_id: passenger_id.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
                active_unit_ids.insert(passenger_id.clone());
            }
        },
        CoreEvent::UnloadUnit{ref unit_info, ref transporter_id, ref from, ref to} => {
            active_unit_ids.insert(unit_info.unit_id.clone());
            let passenger = state.unit(&unit_info.unit_id);
            let transporter = state.unit(transporter_id.as_ref().unwrap());
            let is_transporter_vis = fow.is_visible(
                db, state, transporter, from);
            let is_passenger_vis = fow.is_visible(
                db, state, passenger, to);
            if passenger.player_id == *player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id.clone()
                } else {
                    None
                };
                events.push(CoreEvent::UnloadUnit {
                    transporter_id: filtered_transporter_id,
                    unit_info: unit_info.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
                if !is_passenger_vis {
                    events.push(CoreEvent::HideUnit {
                        unit_id: passenger.id.clone(),
                    });
                }
            }
        },
        CoreEvent::SetReactionFireMode{ref unit_id, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == *player_id {
                events.push(event.clone());
            }
        },
    }
    (events, active_unit_ids)
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
