// See LICENSE file for copyright and license details.

use std::sync::mpsc::{Sender};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use common::types::{UnitId, MapPos, ZInt, ZFloat};
use zgl::{self, Time, ScreenPos};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap, basic_text_size};
use tactical_screen::{PickResult};

#[derive(Clone)]
pub enum Command {
    Select{id: UnitId},
    Move{pos: MapPos},
    Hunt{pos: MapPos},
    Attack{id: UnitId},
    LoadUnit{passenger_id: UnitId},
    UnloadUnit{pos: MapPos},
    EnableReactionFire{id: UnitId},
    DisableReactionFire{id: UnitId},
}

#[derive(PartialEq)]
pub struct Options {
    pub show_select_button: bool,
    pub show_move_button: bool,
    pub show_hunt_button: bool,
    pub show_attack_button: bool,
    pub show_load_button: bool,
    pub show_unload_button: bool,
    pub show_enable_reaction_fire: bool,
    pub show_disable_reaction_fire: bool,
    pub pick_result: Option<PickResult>, // TODO: remove Option
}

impl Options {
    pub fn new() -> Options {
        Options {
            show_select_button: false,
            show_move_button: false,
            show_hunt_button: false,
            show_attack_button: false,
            show_load_button: false,
            show_unload_button: false,
            show_enable_reaction_fire: false,
            show_disable_reaction_fire: false,
            pick_result: None,
        }
    }
}

pub struct ContextMenuPopup {
    game_screen_tx: Sender<Command>,
    button_manager: ButtonManager,
    select_button_id: Option<ButtonId>,
    move_button_id: Option<ButtonId>,
    hunt_button_id: Option<ButtonId>,
    attack_button_id: Option<ButtonId>,
    load_unit_button_id: Option<ButtonId>,
    unload_unit_button_id: Option<ButtonId>,
    enable_reaction_fire_button_id: Option<ButtonId>,
    disable_reaction_fire_button_id: Option<ButtonId>,
    pick_result: PickResult,
}

impl ContextMenuPopup {
    pub fn new(
        context: &mut Context,
        pos: &ScreenPos,
        options: Options,
        tx: Sender<Command>,
    ) -> ContextMenuPopup {
        assert!(options.pick_result.is_some());
        let mut button_manager = ButtonManager::new();
        let mut select_button_id = None;
        let mut move_button_id = None;
        let mut hunt_button_id = None;
        let mut attack_button_id = None;
        let mut load_unit_button_id = None;
        let mut unload_unit_button_id = None;
        let mut enable_reaction_fire_button_id = None;
        let mut disable_reaction_fire_button_id = None;
        let mut pos = pos.clone();
        // TODO: Simplify
        let baisc_text_size = basic_text_size(context);
        let (_, test_text_size) = context.font_stash
            .get_text_size(&context.zgl, "X");
        pos.v.y -= test_text_size.h * baisc_text_size as ZInt / 2;
        pos.v.x -= test_text_size.w * baisc_text_size as ZInt / 2;
        let vstep = (test_text_size.h as ZFloat * baisc_text_size * 1.2) as ZInt;
        if options.show_select_button {
            select_button_id = Some(button_manager.add_button(
                Button::new(context, "select", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_move_button {
            move_button_id = Some(button_manager.add_button(
                Button::new(context, "move", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_hunt_button {
            hunt_button_id = Some(button_manager.add_button(
                Button::new(context, "hunt", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_attack_button {
            attack_button_id = Some(button_manager.add_button(
                Button::new(context, "attack", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_load_button {
            load_unit_button_id = Some(button_manager.add_button(
                Button::new(context, "load", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_enable_reaction_fire {
            enable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "enable reaction fire", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_disable_reaction_fire {
            disable_reaction_fire_button_id = Some(button_manager.add_button(
                Button::new(context, "disable reaction fire", &pos)));
            pos.v.y -= vstep;
        }
        if options.show_unload_button {
            unload_unit_button_id = Some(button_manager.add_button(
                Button::new(context, "unload", &pos)));
            pos.v.y -= vstep;
        }
        ContextMenuPopup {
            game_screen_tx: tx,
            button_manager: button_manager,
            select_button_id: select_button_id,
            move_button_id: move_button_id,
            hunt_button_id: hunt_button_id,
            attack_button_id: attack_button_id,
            load_unit_button_id: load_unit_button_id,
            unload_unit_button_id: unload_unit_button_id,
            enable_reaction_fire_button_id: enable_reaction_fire_button_id,
            disable_reaction_fire_button_id: disable_reaction_fire_button_id,
            pick_result: options.pick_result.unwrap(),
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
        let id = Some(button_id.clone());
        if id == self.attack_button_id {
            let id = self.pick_result.unit_id();
            self.return_command(context, Command::Attack{id: id});
        } else if id == self.select_button_id {
            let id = self.pick_result.unit_id();
            self.return_command(context, Command::Select{id: id});
        } else if id == self.move_button_id {
            let pos = self.pick_result.pos();
            self.return_command(context, Command::Move{pos: pos});
        } else if id == self.hunt_button_id {
            let pos = self.pick_result.pos();
            self.return_command(context, Command::Hunt{pos: pos});
        } else if id == self.load_unit_button_id {
            let id = self.pick_result.unit_id();
            self.return_command(context, Command::LoadUnit{passenger_id: id});
        } else if id == self.unload_unit_button_id {
            let pos = self.pick_result.pos();
            self.return_command(context, Command::UnloadUnit{pos: pos});
        } else if id == self.enable_reaction_fire_button_id {
            let id = self.pick_result.unit_id();
            self.return_command(context, Command::EnableReactionFire{id: id});
        } else if id == self.disable_reaction_fire_button_id {
            let id = self.pick_result.unit_id();
            self.return_command(context, Command::DisableReactionFire{id: id});
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
        context.set_basic_color(&zgl::BLACK);
        self.button_manager.draw(context);
    }

    fn handle_event(
        &mut self,
        context: &mut Context,
        event: &glutin::Event,
    ) -> EventStatus {
        let mut event_status = EventStatus::Handled;
        match *event {
            Event::MouseMoved(_) => {},
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::Touch(glutin::Touch{phase, ..}) => {
                if let glutin::TouchPhase::Ended = phase {
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

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
