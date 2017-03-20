use std::collections::{HashSet, HashMap};
use game_state::{State};
use fow::{Fow};
use unit::{Unit, UnitId};
use event::{CoreEvent, Event, MoveMode, AttackInfo};
use player::{PlayerId};
use movement::{MovePoints};

fn filtered_unit(unit: &Unit) -> Unit {
    Unit {
        move_points: None,
        attack_points: None,
        reactive_attack_points: None,
        passenger_id: None,
        .. unit.clone()
    }
}

pub fn get_visible_enemies(
    state: &State,
    fow: &Fow,
    player_id: PlayerId,
) -> HashSet<UnitId> {
    let mut visible_enemies = HashSet::new();
    for (&id, unit) in state.units() {
        if unit.player_id != player_id
            && fow.is_visible(unit)
        {
            visible_enemies.insert(id);
        }
    }
    visible_enemies
}

pub fn show_or_hide_passive_enemies(
    state: &State,
    active_unit_ids: &HashSet<UnitId>,
    old: &HashSet<UnitId>,
    new: &HashSet<UnitId>,
) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    let located_units = new.difference(old);
    for &id in located_units {
        if active_unit_ids.contains(&id) {
            continue;
        }
        let unit = state.unit_opt(id).expect("Can`t find unit");
        events.push(CoreEvent {
            event: Event::ShowUnit {
                unit_info: filtered_unit(unit),
            },
            effects: HashMap::new(),
        });
    }
    let lost_units = old.difference(new);
    for &id in lost_units {
        if active_unit_ids.contains(&id) {
            continue;
        }
        events.push(CoreEvent {
            event: Event::HideUnit{unit_id: id},
            effects: HashMap::new(),
        });
    }
    events
}

pub fn filter_events(
    state: &State,
    player_id: PlayerId,
    fow: &Fow,
    event: &CoreEvent,
) -> (Vec<CoreEvent>, HashSet<UnitId>) {
    println!("filter_event: {:?}\n", event);
    assert!(!state.is_partial());
    let mut active_unit_ids = HashSet::new();
    let mut events = vec![];
    match event.event {
        Event::Move{unit_id, from, to, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == player_id {
                events.push(event.clone());
            } else {
                let prev_vis = fow.is_visible_at(unit, from);
                let next_vis = fow.is_visible_at(unit, to);
                if !prev_vis && next_vis {
                    events.push(CoreEvent {
                        event: Event::ShowUnit {
                            unit_info: Unit {
                                pos: from,
                                .. filtered_unit(unit)
                            },
                        },
                        effects: event.effects.clone(),
                    });
                    if let Some(attached_unit_id) = unit.attached_unit_id {
                        active_unit_ids.insert(attached_unit_id);
                        let attached_unit = state.unit(attached_unit_id);
                        events.push(CoreEvent {
                            event: Event::ShowUnit {
                                unit_info: Unit {
                                    pos: from,
                                    .. filtered_unit(attached_unit)
                                },
                            },
                            effects: event.effects.clone(),
                        });
                    }
                }
                if prev_vis || next_vis {
                    events.push(event.clone());
                }
                if prev_vis && !next_vis {
                    events.push(CoreEvent {
                        event: Event::HideUnit {
                            unit_id: unit.id,
                        },
                        effects: event.effects.clone(),
                    });
                }
                active_unit_ids.insert(unit_id);
            }
        },
        Event::CreateUnit{ref unit_info} => {
            let unit = state.unit(unit_info.id);
            if player_id == unit_info.player_id
                || fow.is_visible_at(unit, unit_info.pos)
            {
                events.push(event.clone());
                active_unit_ids.insert(unit_info.id);
            }
        },
        Event::AttackUnit{ref attack_info} => {
            let attacker_id = attack_info.attacker_id
                .expect("Core must know about everything");
            let attacker = state.unit(attacker_id);
            if player_id != attacker.player_id && !attack_info.is_ambush {
                // show attacker if this is not ambush
                let attacker = state.unit(attacker_id);
                if !fow.is_visible(attacker) {
                    events.push(CoreEvent {
                        event: Event::ShowUnit {
                            unit_info: filtered_unit(attacker),
                        },
                        effects: event.effects.clone(),
                    });
                }
                active_unit_ids.insert(attacker_id);
            }
            for &target_id in event.effects.keys() {
                active_unit_ids.insert(target_id);
            }
            let is_attacker_visible = player_id == attacker.player_id
                || !attack_info.is_ambush;
            let attack_info = AttackInfo {
                attacker_id: if is_attacker_visible {
                    Some(attacker_id)
                } else {
                    None
                },
                .. attack_info.clone()
            };
            events.push(CoreEvent {
                event: Event::AttackUnit{attack_info: attack_info},
                effects: event.effects.clone(),
            });
        },
        Event::Reveal{ref unit_info} => {
            if unit_info.player_id != player_id {
                events.push(CoreEvent {
                    event: Event::ShowUnit {
                        unit_info: filtered_unit(unit_info),
                    },
                    effects: event.effects.clone(),
                });
            }
        },
        Event::ShowUnit{..} |
        Event::HideUnit{..} => panic!(),
        Event::LoadUnit{passenger_id, from, to, transporter_id} => {
            let passenger = state.unit(passenger_id);
            let transporter = state.unit(transporter_id.unwrap());
            let is_transporter_vis = fow.is_visible(transporter);
            let is_passenger_vis = fow.is_visible_at(passenger, from);
            if passenger.player_id == player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                if !fow.is_visible_at(passenger, from) {
                    events.push(CoreEvent {
                        event: Event::ShowUnit {
                            unit_info: Unit {
                                pos: from,
                                .. filtered_unit(passenger)
                            },
                        },
                        effects: event.effects.clone(),
                    });
                }
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id
                } else {
                    None
                };
                events.push(CoreEvent {
                    event: Event::LoadUnit {
                        transporter_id: filtered_transporter_id,
                        passenger_id: passenger_id,
                        from: from,
                        to: to,
                    },
                    effects: event.effects.clone(),
                });
                active_unit_ids.insert(passenger_id);
            }
        },
        Event::UnloadUnit{ref unit_info, transporter_id, from, to} => {
            active_unit_ids.insert(unit_info.id);
            let passenger = state.unit(unit_info.id);
            let transporter = state.unit(transporter_id.unwrap());
            let is_transporter_vis = fow.is_visible_at(transporter, from);
            let is_passenger_vis = fow.is_visible_at(passenger, to);
            if passenger.player_id == player_id {
                events.push(event.clone());
            } else if is_passenger_vis || is_transporter_vis {
                let filtered_transporter_id = if is_transporter_vis {
                    transporter_id
                } else {
                    None
                };
                events.push(CoreEvent {
                    event: Event::UnloadUnit {
                        transporter_id: filtered_transporter_id,
                        unit_info: Unit {
                            move_points: None,
                            attack_points: None,
                            reactive_attack_points: None,
                            passenger_id: None,
                            .. unit_info.clone()
                        },
                        from: from,
                        to: to,
                    },
                    effects: event.effects.clone(),
                });
                if !is_passenger_vis {
                    events.push(CoreEvent {
                        event: Event::HideUnit {
                            unit_id: passenger.id,
                        },
                        effects: event.effects.clone(),
                    });
                }
            }
        },
        Event::Attach{transporter_id, attached_unit_id, from, to} => {
            let transporter = state.unit(transporter_id);
            if transporter.player_id == player_id {
                events.push(event.clone())
            } else {
                active_unit_ids.insert(transporter_id);
                let attached_unit = state.unit(attached_unit_id);
                let is_attached_unit_vis = fow.is_visible_at(attached_unit, to);
                let is_transporter_vis = fow.is_visible_at(transporter, from);
                if is_attached_unit_vis {
                    if !is_transporter_vis {
                        events.push(CoreEvent {
                            event: Event::ShowUnit {
                                unit_info: Unit {
                                    pos: from,
                                    attached_unit_id: None,
                                    .. filtered_unit(transporter)
                                },
                            },
                            effects: event.effects.clone(),
                        });
                    }
                    events.push(event.clone())
                } else if is_transporter_vis {
                    events.push(CoreEvent {
                            event: Event::Move {
                            unit_id: transporter_id,
                            mode: MoveMode::Fast,
                            cost: MovePoints{n: 0},
                            from: from,
                            to: to,
                        },
                        effects: event.effects.clone(),
                    });
                    events.push(CoreEvent {
                        event: Event::HideUnit {
                            unit_id: transporter_id,
                        },
                        effects: event.effects.clone(),
                    });
                }
            }
        },
        Event::Detach{transporter_id, from, to} => {
            let transporter = state.unit(transporter_id);
            if transporter.player_id == player_id {
                events.push(event.clone())
            } else {
                active_unit_ids.insert(transporter_id);
                let is_from_vis = fow.is_visible_at(transporter, from);
                let is_to_vis = fow.is_visible_at(transporter, to);
                if is_from_vis {
                    events.push(event.clone());
                    if !is_to_vis {
                        events.push(CoreEvent {
                            event: Event::HideUnit {
                                unit_id: transporter_id,
                            },
                            effects: event.effects.clone(),
                        });
                    }
               } else if is_to_vis {
                    events.push(CoreEvent {
                        event: Event::ShowUnit {
                            unit_info: Unit {
                                pos: from,
                                attached_unit_id: None,
                                .. filtered_unit(transporter)
                            },
                        },
                        effects: event.effects.clone(),
                    });
                    events.push(CoreEvent {
                        event: Event::Move {
                            unit_id: transporter_id,
                            mode: MoveMode::Fast,
                            cost: MovePoints{n: 0},
                            from: from,
                            to: to,
                        },
                        effects: event.effects.clone(),
                    });
                }
            }
        },
        Event::SetReactionFireMode{unit_id, ..} => {
            let unit = state.unit(unit_id);
            if unit.player_id == player_id {
                events.push(event.clone());
            }
        },
        Event::Smoke{id, pos, unit_id} => {
            let unit_id = unit_id.expect("Core must know about everything");
            let unit = state.unit(unit_id);
            if fow.is_visible(unit) {
                events.push(event.clone());
            } else {
                events.push(CoreEvent {
                    event: Event::Smoke {
                        id: id,
                        pos: pos,
                        unit_id: None,
                    },
                    effects: event.effects.clone(),
                });
            }
        },
        Event::EndTurn{..} |
        Event::RemoveSmoke{..} |
        Event::VictoryPoint{..} |
        Event::SectorOwnerChanged{..} => {
            events.push(event.clone());
        },
    }
    (events, active_unit_ids)
}
