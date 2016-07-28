// See LICENSE file for copyright and license details.

use std::default::{Default};
use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use screen::{Screen, ScreenCommand, EventStatus};
use tactical_screen::{TacticalScreen};
use core;
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap};
use types::{ScreenPos, Time};

pub struct MainMenuScreen {
    button_start_hotseat_id: ButtonId,
    button_start_vs_ai_id: ButtonId,
    button_manager: ButtonManager,
}

impl MainMenuScreen {
    pub fn new(context: &mut Context) -> MainMenuScreen {
        let mut button_manager = ButtonManager::new();
        // TODO: Use relative coords in ScreenPos - x: [0.0, 1.0], y: [0.0, 1.0]
        // TODO: Add analog of Qt::Alignment
        let mut button_pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let button_start_hotseat_id = button_manager.add_button(Button::new(
            context,
            "start hotseat",
            &button_pos,
        ));
        // TODO: Add something like QLayout
        button_pos.v.y += button_manager.buttons()[&button_start_hotseat_id]
            .size().h;
        let button_start_vs_ai_id = button_manager.add_button(Button::new(
            context,
            "start human vs ai",
            &button_pos,
        ));
        MainMenuScreen {
            button_manager: button_manager,
            button_start_hotseat_id: button_start_hotseat_id,
            button_start_vs_ai_id: button_start_vs_ai_id,
        }
    }

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if !is_tap(context) {
            return;
        }
        if let Some(button_id) = self.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, &button_id);
        }
    }

    fn handle_event_button_press(
        &mut self,
        context: &mut Context,
        button_id: &ButtonId
    ) {
        if *button_id == self.button_start_hotseat_id {
            let core_options = Default::default();
            let tactical_screen = Box::new(TacticalScreen::new(context, &core_options));
            context.add_command(ScreenCommand::PushScreen(tactical_screen));
        } else if *button_id == self.button_start_vs_ai_id {
            let core_options = core::Options {
                game_type: core::GameType::SingleVsAi,
                .. Default::default()
            };
            let tactical_screen = Box::new(TacticalScreen::new(context, &core_options));
            context.add_command(ScreenCommand::PushScreen(tactical_screen));
        } else {
            panic!("Bad button id: {}", button_id.id);
        }
    }

    fn handle_event_key_press(&mut self, context: &mut Context, key: VirtualKeyCode) {
        match key {
            glutin::VirtualKeyCode::Q
                | glutin::VirtualKeyCode::Escape =>
            {
                context.add_command(ScreenCommand::PopScreen);
            },
            _ => {},
        }
    }
}

impl Screen for MainMenuScreen {
    fn tick(&mut self, context: &mut Context, _: &Time) {
        context.clear_color = [0.7, 0.7, 0.7, 1.0];
        context.encoder.clear(&context.data.out, context.clear_color);
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
        self.button_manager.draw(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) -> EventStatus {
        match *event {
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::Touch(glutin::Touch{phase, ..}) => {
                match phase {
                    glutin::TouchPhase::Ended => {
                        self.handle_event_lmb_release(context);
                    },
                    _ => {},
                }
            },
            glutin::Event::KeyboardInput(Released, _, Some(key)) => {
                self.handle_event_key_press(context, key);
            },
            _ => {},
        }
        EventStatus::Handled
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
