// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, is_tap};
use core::{PlayerId};
use types::{ScreenPos, Time};

pub struct EndTurnScreen {
    button_manager: ButtonManager,
}

impl EndTurnScreen {
    pub fn new(
        context: &mut Context,
        player_id: &PlayerId,
    ) -> EndTurnScreen {
        let mut button_manager = ButtonManager::new();
        let pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let str = format!("Pass the device to Player {}", player_id.id);
        // TODO: button -> label + center on screen
        let _ = button_manager.add_button(Button::new(
            context, &str, &pos));
        EndTurnScreen {
            button_manager: button_manager,
        }
    }

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if is_tap(context) {
            context.add_command(ScreenCommand::PopScreen);
        }
    }

    fn handle_event_key_press(&mut self, context: &mut Context, key: VirtualKeyCode) {
        if key == glutin::VirtualKeyCode::Q
            || key == glutin::VirtualKeyCode::Escape
        {
            context.add_command(ScreenCommand::PopScreen);
        }
    }
}

impl Screen for EndTurnScreen {
    fn tick(&mut self, context: &mut Context, _: &Time) {
        context.data.basic_color = [0.0, 0.0, 0.0, 1.0];
        self.button_manager.draw(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) -> EventStatus {
        match *event {
            Event::MouseInput(Released, MouseButton::Left) => {
                self.handle_event_lmb_release(context);
            },
            Event::Touch(glutin::Touch{phase, ..}) => {
                if glutin::TouchPhase::Ended == phase {
                    self.handle_event_lmb_release(context);
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
