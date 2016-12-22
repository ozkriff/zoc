use std::sync::mpsc::{Sender};
use std::collections::{HashMap};
use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use core::{self, ObjectClass, UnitId, MapPos, ExactPos, HitChance};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use core::db::{Db};
use core::check::{check_command};
use types::{Time, ScreenPos};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap, basic_text_size};
use player_info::{PlayerInfo};
use reinforcements_popup;

fn can_unload_unit(
    db: &Db,
    state: &PartialState,
    transporter_id: UnitId,
    pos: MapPos,
) -> Option<ExactPos> {
    let transporter = state.unit(transporter_id);
    let passenger_id = match transporter.passenger_id {
        Some(id) => id,
        None => return None,
    };
    let type_id = state.unit(passenger_id).type_id;
    let exact_pos = match core::get_free_exact_pos(db, state, type_id, pos) {
        Some(pos) => pos,
        None => return None,
    };
    let player_id = transporter.player_id;
    let command = core::Command::UnloadUnit {
        transporter_id: transporter_id,
        passenger_id: passenger_id,
        pos: exact_pos,
    };
    if check_command(db, player_id, state, &command).is_ok() {
        Some(exact_pos)
    } else {
        None
    }
}

fn can_detach_unit(
    db: &Db,
    state: &PartialState,
    transporter_id: UnitId,
    pos: MapPos,
) -> Option<ExactPos> {
    let transporter = state.unit(transporter_id);
    if transporter.attached_unit_id.is_none() {
        return None;
    }
    let type_id = transporter.type_id;
    let exact_pos = match core::get_free_exact_pos(db, state, type_id, pos) {
        Some(pos) => pos,
        None => return None,
    };
    let command = core::Command::Detach {
        transporter_id: transporter_id,
        pos: exact_pos,
    };
    if check_command(db, transporter.player_id, state, &command).is_ok() {
        Some(exact_pos)
    } else {
        None
    }
}

pub fn get_options(
    core: &core::Core,
    player_info: &PlayerInfo,
    selected_unit_id: Option<UnitId>,
    pos: MapPos,
) -> Options {
    let state = &player_info.game_state;
    let pathfinder = &player_info.pathfinder;
    let db = core.db();
    let mut options = Options::new();
    let player_id = core.player_id();
    let unit_ids = core::get_unit_ids_at(state, pos);
    for object in state.objects_at(pos) {
        if object.class != ObjectClass::ReinforcementSector {
            continue;
        }
        let owner_id = match object.owner_id {
            Some(id) => id,
            None => continue,
        };
        if owner_id != player_id {
            continue;
        }
        let reinforcement_options = reinforcements_popup::get_options(
            db, state, player_id, pos);
        if reinforcement_options == reinforcements_popup::Options::new() {
            continue;
        }
        options.reinforcements_pos = Some(pos);
    }
    let selected_unit_id = match selected_unit_id {
        Some(id) => id,
        None => {
            for unit_id in unit_ids {
                let unit = state.unit(unit_id);
                if unit.is_alive && unit.player_id == player_id {
                    options.selects.push(unit_id);
                }
            }
            return options;
        }
    };
    for unit_id in unit_ids {
        let unit = state.unit(unit_id);
        let unit_type = db.unit_type(unit.type_id);
        if unit.player_id == player_id {
            if unit_id == selected_unit_id {
                if unit_type.attack_points.n != 0
                    || unit_type.reactive_attack_points.n != 0
                {
                    if unit.reaction_fire_mode == core::ReactionFireMode::HoldFire {
                        options.enable_reaction_fire = Some(selected_unit_id);
                    } else {
                        options.disable_reaction_fire = Some(selected_unit_id);
                    }
                }
            } else {
                // TODO: attach dead enemies
                if unit.is_alive {
                    options.selects.push(unit_id);
                    let load_command = core::Command::LoadUnit {
                        transporter_id: selected_unit_id,
                        passenger_id: unit_id,
                    };
                    if check_command(db, player_id, state, &load_command).is_ok() {
                        options.loads.push(unit_id);
                    }
                }
                let attach_command = core::Command::Attach {
                    transporter_id: selected_unit_id,
                    attached_unit_id: unit_id,
                };
                if check_command(db, player_id, state, &attach_command).is_ok() {
                    options.attaches.push(unit_id);
                }
            }
        } else {
            let attacker = state.unit(selected_unit_id);
            let defender = state.unit(unit_id);
            let hit_chance = core::hit_chance(db, state, attacker, defender);
            let attack_command = core::Command::AttackUnit {
                attacker_id: attacker.id,
                defender_id: defender.id,
            };
            if check_command(db, player_id, state, &attack_command).is_ok() {
                options.attacks.push((unit_id, hit_chance));
            }
        }
    }
    if check_command(db, player_id, state, &core::Command::Smoke {
        unit_id: selected_unit_id,
        pos: pos,
    }).is_ok() {
        options.smoke_pos = Some(pos);
    }
    if let Some(pos) = can_unload_unit(db, state, selected_unit_id, pos) {
        options.unload_pos = Some(pos);
    }
    if let Some(pos) = can_detach_unit(db, state, selected_unit_id, pos) {
        options.detach_pos = Some(pos);
    }
    let selected_unit = state.unit(selected_unit_id);
    let selected_unit_type = db.unit_type(selected_unit.type_id);
    if let Some(destination) = core::get_free_exact_pos(
        db, state, state.unit(selected_unit_id).type_id, pos,
    ) {
        if let Some(path) = pathfinder.get_path(destination) {
            if check_command(db, player_id, state, &core::Command::Move {
                unit_id: selected_unit_id,
                path: path.clone(),
                mode: core::MoveMode::Fast,
            }).is_ok() {
                options.move_pos = Some(destination);
            }
            let hunt_command = core::Command::Move {
                unit_id: selected_unit_id,
                path: path.clone(),
                mode: core::MoveMode::Hunt,
            };
            if !selected_unit_type.is_air
                && check_command(db, player_id, state, &hunt_command).is_ok()
            {
                options.hunt_pos = Some(destination);
            }
        }
    }
    options
}

#[derive(Clone, Debug)]
pub enum Command {
    Select{id: UnitId},
    Move{pos: ExactPos},
    Hunt{pos: ExactPos},
    Attack{id: UnitId},
    LoadUnit{passenger_id: UnitId},
    Attach{attached_unit_id: UnitId},
    UnloadUnit{pos: ExactPos},
    Detach{pos: ExactPos},
    EnableReactionFire{id: UnitId},
    DisableReactionFire{id: UnitId},
    Smoke{pos: MapPos},
    CallReiforcements{pos: MapPos},
}

#[derive(PartialEq, Debug, Clone)]
pub struct Options {
    selects: Vec<UnitId>,
    attacks: Vec<(UnitId, HitChance)>,
    loads: Vec<UnitId>,
    attaches: Vec<UnitId>,
    move_pos: Option<ExactPos>,
    hunt_pos: Option<ExactPos>,
    unload_pos: Option<ExactPos>,
    detach_pos: Option<ExactPos>,
    smoke_pos: Option<MapPos>,
    enable_reaction_fire: Option<UnitId>,
    disable_reaction_fire: Option<UnitId>,
    reinforcements_pos: Option<MapPos>,
}

impl Options {
    pub fn new() -> Options {
        Options {
            selects: Vec::new(),
            attacks: Vec::new(),
            loads: Vec::new(),
            attaches: Vec::new(),
            move_pos: None,
            hunt_pos: None,
            unload_pos: None,
            detach_pos: None,
            smoke_pos: None,
            enable_reaction_fire: None,
            disable_reaction_fire: None,
            reinforcements_pos: None,
        }
    }
}

fn max_width(button_manager: &ButtonManager) -> i32 {
    button_manager.buttons().values()
        .max_by_key(|b| b.size().w)
        .unwrap().size().w
}

#[derive(Clone, Debug)]
pub struct ContextMenuPopup {
    game_screen_tx: Sender<Command>,
    button_manager: ButtonManager,
    options: Options,
    select_button_ids: HashMap<ButtonId, UnitId>,
    attack_button_ids: HashMap<ButtonId, UnitId>,
    load_button_ids: HashMap<ButtonId, UnitId>,
    attach_button_ids: HashMap<ButtonId, UnitId>,
    move_button_id: Option<ButtonId>,
    hunt_button_id: Option<ButtonId>,
    unload_unit_button_id: Option<ButtonId>,
    detach_button_id: Option<ButtonId>,
    smoke_button_id: Option<ButtonId>,
    enable_reaction_fire_button_id: Option<ButtonId>,
    disable_reaction_fire_button_id: Option<ButtonId>,
    call_reinforcements_button_id: Option<ButtonId>,
}

impl ContextMenuPopup {
    pub fn new(
        state: &PartialState,
        db: &Db,
        context: &mut Context,
        pos: ScreenPos,
        options: Options,
        tx: Sender<Command>,
    ) -> ContextMenuPopup {
        let mut button_manager = ButtonManager::new();
        let mut select_button_ids = HashMap::new();
        let mut attack_button_ids = HashMap::new();
        let mut load_button_ids = HashMap::new();
        let mut attach_button_ids = HashMap::new();
        let mut move_button_id = None;
        let mut hunt_button_id = None;
        let mut unload_unit_button_id = None;
        let mut detach_button_id = None;
        let mut smoke_button_id = None;
        let mut enable_reaction_fire_button_id = None;
        let mut disable_reaction_fire_button_id = None;
        let mut call_reinforcements_button_id = None;
        let mut pos = pos;
        let text_size = basic_text_size(context);
        pos.v.y -= text_size as i32 / 2;
        pos.v.x -= text_size as i32 / 2;
        let vstep = (text_size * 0.8) as i32;
        for &unit_id in &options.selects {
            let unit_type = db.unit_type(state.unit(unit_id).type_id);
            let button_id = button_manager.add_button(
                Button::new(context, &format!("select <{}>", unit_type.name), pos));
            select_button_ids.insert(button_id, unit_id);
            pos.v.y -= vstep;
        }
        for &(unit_id, hit_chance) in &options.attacks {
            let unit_type = db.unit_type(state.unit(unit_id).type_id);
            let text = format!("attack <{}> ({}%)", unit_type.name, hit_chance.n);
            let button_id = button_manager.add_button(
                Button::new(context, &text, pos));
            attack_button_ids.insert(button_id, unit_id);
            pos.v.y -= vstep;
        }
        for &unit_id in &options.loads {
            let unit_type = db.unit_type(state.unit(unit_id).type_id);
            let button_id = button_manager.add_button(
                Button::new(context, &format!("load <{}>", unit_type.name), pos));
            load_button_ids.insert(button_id, unit_id);
            pos.v.y -= vstep;
        }
        for &unit_id in &options.attaches {
            let unit_type = db.unit_type(state.unit(unit_id).type_id);
            let button_id = button_manager.add_button(
                Button::new(context, &format!("attach <{}>", unit_type.name), pos));
            attach_button_ids.insert(button_id, unit_id);
            pos.v.y -= vstep;
        }
        if options.move_pos.is_some() {
            move_button_id = Some(button_manager.add_button(
                Button::new(context, "move", pos)));
            pos.v.y -= vstep;
        }
        if options.hunt_pos.is_some() {
            hunt_button_id = Some(button_manager.add_button(
                Button::new(context, "hunt", pos)));
            pos.v.y -= vstep;
        }
        if options.enable_reaction_fire.is_some() {
            enable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "enable reaction fire", pos)));
            pos.v.y -= vstep;
        }
        if options.disable_reaction_fire.is_some() {
            disable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "disable reaction fire", pos)));
            pos.v.y -= vstep;
        }
        if options.reinforcements_pos.is_some() {
            call_reinforcements_button_id = Some(button_manager.add_button(
                Button::new(context, "reinforcements", pos)));
            pos.v.y -= vstep;
        }
        if options.unload_pos.is_some() {
            unload_unit_button_id = Some(button_manager.add_button(
                Button::new(context, "unload", pos)));
            pos.v.y -= vstep;
        }
        if options.detach_pos.is_some() {
            detach_button_id = Some(button_manager.add_button(
                Button::new(context, "detach", pos)));
            pos.v.y -= vstep;
        }
        if options.smoke_pos.is_some() {
            smoke_button_id = Some(button_manager.add_button(
                Button::new(context, "smoke", pos)));
            pos.v.y -= vstep;
        }
        let diff = Vector2 {
            x: pos.v.x + max_width(&button_manager) - context.win_size().w,
            y: pos.v.y + vstep,
        };
        let diff = Vector2 {
            x: if diff.x > 0 { diff.x } else { 0 },
            y: if diff.y < 0 { diff.y } else { 0 },
        };
        for button in button_manager.buttons_mut().values_mut() {
            let new_pos = ScreenPos{v: button.pos().v - diff};
            button.set_pos(new_pos);
        }
        ContextMenuPopup {
            game_screen_tx: tx,
            button_manager: button_manager,
            select_button_ids: select_button_ids,
            attack_button_ids: attack_button_ids,
            load_button_ids: load_button_ids,
            attach_button_ids: attach_button_ids,
            move_button_id: move_button_id,
            hunt_button_id: hunt_button_id,
            unload_unit_button_id: unload_unit_button_id,
            detach_button_id: detach_button_id,
            smoke_button_id: smoke_button_id,
            enable_reaction_fire_button_id: enable_reaction_fire_button_id,
            disable_reaction_fire_button_id: disable_reaction_fire_button_id,
            call_reinforcements_button_id: call_reinforcements_button_id,
            options: options,
        }
    }

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if !is_tap(context) {
            return;
        }
        if let Some(button_id) = self.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, button_id);
        } else {
            context.add_command(ScreenCommand::PopPopup);
        }
    }

    fn return_command(&self, context: &mut Context, command: Command) {
        self.game_screen_tx.send(command).unwrap();
        context.add_command(ScreenCommand::PopPopup);
    }

    fn handle_event_button_press(
        &mut self,
        context: &mut Context,
        button_id: ButtonId
    ) {
        if let Some(&unit_id) = self.select_button_ids.get(&button_id) {
            self.return_command(context, Command::Select {
                id: unit_id,
            });
            return;
        }
        if let Some(&unit_id) = self.attack_button_ids.get(&button_id) {
            self.return_command(context, Command::Attack {
                id: unit_id,
            });
            return;
        }
        if let Some(&unit_id) = self.load_button_ids.get(&button_id) {
            self.return_command(context, Command::LoadUnit {
                passenger_id: unit_id,
            });
            return;
        }
        if let Some(&unit_id) = self.attach_button_ids.get(&button_id) {
            self.return_command(context, Command::Attach {
                attached_unit_id: unit_id,
            });
            return;
        }
        let id = Some(button_id);
        if id == self.move_button_id {
            self.return_command(context, Command::Move {
                pos: self.options.move_pos.unwrap(),
            });
        } else if id == self.hunt_button_id {
            self.return_command(context, Command::Hunt {
                pos: self.options.move_pos.unwrap(),
            });
        } else if id == self.unload_unit_button_id {
            self.return_command(context, Command::UnloadUnit {
                pos: self.options.unload_pos.unwrap(),
            });
        } else if id == self.detach_button_id {
            self.return_command(context, Command::Detach {
                pos: self.options.detach_pos.unwrap(),
            });
        } else if id == self.smoke_button_id {
            self.return_command(context, Command::Smoke {
                pos: self.options.smoke_pos.unwrap(),
            });
        } else if id == self.enable_reaction_fire_button_id {
            self.return_command(context, Command::EnableReactionFire {
                id: self.options.enable_reaction_fire.unwrap(),
            });
        } else if id == self.disable_reaction_fire_button_id {
            self.return_command(context, Command::DisableReactionFire {
                id: self.options.disable_reaction_fire.unwrap(),
            });
        } else if id == self.call_reinforcements_button_id {
            self.return_command(context, Command::CallReiforcements {
                pos: self.options.reinforcements_pos.unwrap(),
            });
        } else {
            panic!("Bad button id: {}", button_id.id);
        }
    }

    fn handle_event_key_press(&mut self, context: &mut Context, key: VirtualKeyCode) {
        match key {
            glutin::VirtualKeyCode::Q
                | glutin::VirtualKeyCode::Escape =>
            {
                context.add_command(ScreenCommand::PopPopup);
            },
            _ => {},
        }
    }
}

impl Screen for ContextMenuPopup {
    fn tick(&mut self, context: &mut Context, _: Time) {
        context.set_basic_color([0.0, 0.0, 0.0, 1.0]);
        self.button_manager.draw(context);
    }

    fn handle_event(
        &mut self,
        context: &mut Context,
        event: &glutin::Event,
    ) -> EventStatus {
        let mut event_status = EventStatus::Handled;
        match *event {
            Event::MouseMoved(..) => {},
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::Touch(glutin::Touch{phase, ..}) => {
                if phase == glutin::TouchPhase::Ended {
                    self.handle_event_lmb_release(context);
                }
            },
            glutin::Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(context, key);
            },
            _ => event_status = EventStatus::NotHandled,
        }
        event_status
    }
}
