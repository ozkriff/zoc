use std::sync::mpsc::{Sender};
use std::collections::{HashMap};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use core::{UnitId, MapPos, ExactPos};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use core::db::{Db};
use types::{Time, ScreenPos};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap, basic_text_size};

#[derive(Clone, Debug)]
pub enum Command {
    Select{id: UnitId},
    Move{pos: ExactPos},
    Hunt{pos: ExactPos},
    Attack{id: UnitId},
    LoadUnit{passenger_id: UnitId},
    UnloadUnit{pos: ExactPos},
    EnableReactionFire{id: UnitId},
    DisableReactionFire{id: UnitId},
    Smoke{pos: MapPos},
}

#[derive(PartialEq, Debug, Clone)]
pub struct Options {
    // TODO: display unit name and/or type, not just IDs
    pub selects: Vec<UnitId>,
    pub attacks: Vec<(UnitId, i32)>,
    pub loads: Vec<UnitId>,
    pub move_pos: Option<ExactPos>,
    pub hunt_pos: Option<ExactPos>,
    pub unload_pos: Option<ExactPos>,
    pub smoke_pos: Option<MapPos>,
    pub enable_reaction_fire: Option<UnitId>,
    pub disable_reaction_fire: Option<UnitId>,
}

impl Options {
    pub fn new() -> Options {
        Options {
            selects: Vec::new(),
            attacks: Vec::new(),
            loads: Vec::new(),
            move_pos: None,
            hunt_pos: None,
            unload_pos: None,
            smoke_pos: None,
            enable_reaction_fire: None,
            disable_reaction_fire: None,
        }
    }
}

pub struct ContextMenuPopup {
    game_screen_tx: Sender<Command>,
    button_manager: ButtonManager,
    options: Options,
    select_button_ids: HashMap<ButtonId, UnitId>,
    attack_button_ids: HashMap<ButtonId, UnitId>,
    load_button_ids: HashMap<ButtonId, UnitId>,
    move_button_id: Option<ButtonId>,
    hunt_button_id: Option<ButtonId>,
    unload_unit_button_id: Option<ButtonId>,
    smoke_button_id: Option<ButtonId>,
    enable_reaction_fire_button_id: Option<ButtonId>,
    disable_reaction_fire_button_id: Option<ButtonId>,
}

impl ContextMenuPopup {
    pub fn new(
        state: &PartialState,
        db: &Db,
        context: &mut Context,
        pos: &ScreenPos,
        options: Options,
        tx: Sender<Command>,
    ) -> ContextMenuPopup {
        let mut button_manager = ButtonManager::new();
        let mut select_button_ids = HashMap::new();
        let mut attack_button_ids = HashMap::new();
        let mut load_button_ids = HashMap::new();
        let mut move_button_id = None;
        let mut hunt_button_id = None;
        let mut unload_unit_button_id = None;
        let mut smoke_button_id = None;
        let mut enable_reaction_fire_button_id = None;
        let mut disable_reaction_fire_button_id = None;
        let mut pos = *pos;
        let text_size = basic_text_size(context);
        pos.v.y -= text_size as i32 / 2;
        pos.v.x -= text_size as i32 / 2;
        let vstep = (text_size * 0.9) as i32;
        for unit_id in &options.selects {
            let unit_type = db.unit_type(&state.unit(unit_id).type_id);
            let button_id = button_manager.add_button(
                Button::new(context, &format!("select <{}>", unit_type.name), &pos));
            select_button_ids.insert(button_id, unit_id.clone());
            pos.v.y -= vstep;
        }
        for &(unit_id, hit_chance) in &options.attacks {
            let unit_type = db.unit_type(&state.unit(&unit_id).type_id);
            let text = format!("attack <{}> ({}%)", unit_type.name, hit_chance);
            let button_id = button_manager.add_button(
                Button::new(context, &text, &pos));
            attack_button_ids.insert(button_id, unit_id);
            pos.v.y -= vstep;
        }
        for unit_id in &options.loads {
            let unit_type = db.unit_type(&state.unit(unit_id).type_id);
            let button_id = button_manager.add_button(
                Button::new(context, &format!("load <{}>", unit_type.name), &pos));
            load_button_ids.insert(button_id, unit_id.clone());
            pos.v.y -= vstep;
        }
        if options.move_pos.is_some() {
            move_button_id = Some(button_manager.add_button(
                Button::new(context, "move", &pos)));
            pos.v.y -= vstep;
        }
        if options.hunt_pos.is_some() {
            hunt_button_id = Some(button_manager.add_button(
                Button::new(context, "hunt", &pos)));
            pos.v.y -= vstep;
        }
        if options.enable_reaction_fire.is_some() {
            enable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "enable reaction fire", &pos)));
            pos.v.y -= vstep;
        }
        if options.disable_reaction_fire.is_some() {
            disable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "disable reaction fire", &pos)));
            pos.v.y -= vstep;
        }
        if options.unload_pos.is_some() {
            unload_unit_button_id = Some(button_manager.add_button(
                Button::new(context, "unload", &pos)));
            pos.v.y -= vstep;
        }
        if options.smoke_pos.is_some() {
            smoke_button_id = Some(button_manager.add_button(
                Button::new(context, "smoke", &pos)));
            pos.v.y -= vstep;
        }
        ContextMenuPopup {
            game_screen_tx: tx,
            button_manager: button_manager,
            select_button_ids: select_button_ids,
            attack_button_ids: attack_button_ids,
            load_button_ids: load_button_ids,
            move_button_id: move_button_id,
            hunt_button_id: hunt_button_id,
            unload_unit_button_id: unload_unit_button_id,
            smoke_button_id: smoke_button_id,
            enable_reaction_fire_button_id: enable_reaction_fire_button_id,
            disable_reaction_fire_button_id: disable_reaction_fire_button_id,
            options: options,
        }
    }

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if !is_tap(context) {
            return;
        }
        if let Some(button_id) = self.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, &button_id);
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
        button_id: &ButtonId
    ) {
        if let Some(unit_id) = self.select_button_ids.get(button_id) {
            self.return_command(context, Command::Select {
                id: unit_id.clone(),
            });
            return;
        }
        if let Some(unit_id) = self.attack_button_ids.get(button_id) {
            self.return_command(context, Command::Attack {
                id: unit_id.clone(),
            });
            return;
        }
        if let Some(unit_id) = self.load_button_ids.get(button_id) {
            self.return_command(context, Command::LoadUnit {
                passenger_id: unit_id.clone(),
            });
            return;
        }
        let id = Some(button_id.clone());
        if id == self.move_button_id {
            self.return_command(context, Command::Move {
                pos: self.options.move_pos.clone().unwrap(),
            });
        } else if id == self.hunt_button_id {
            self.return_command(context, Command::Hunt {
                pos: self.options.move_pos.clone().unwrap(),
            });
        } else if id == self.unload_unit_button_id {
            self.return_command(context, Command::UnloadUnit {
                pos: self.options.unload_pos.clone().unwrap(),
            });
        } else if id == self.smoke_button_id {
            self.return_command(context, Command::Smoke {
                pos: self.options.smoke_pos.unwrap(),
            });
        } else if id == self.enable_reaction_fire_button_id {
            self.return_command(context, Command::EnableReactionFire {
                id: self.options.enable_reaction_fire.clone().unwrap(),
            });
        } else if id == self.disable_reaction_fire_button_id {
            self.return_command(context, Command::DisableReactionFire {
                id: self.options.disable_reaction_fire.clone().unwrap(),
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
    fn tick(&mut self, context: &mut Context, _: &Time) {
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
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
