// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use zgl::{self, Time, ScreenPos};
use screen::{Screen, ScreenCommand};
use tactical_screen::{TacticalScreen};
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap};

pub struct MainMenuScreen {
    button_start_id: ButtonId,
    button_manager: ButtonManager,
}

impl MainMenuScreen {
    pub fn new(context: &mut Context) -> MainMenuScreen {
        let mut button_manager = ButtonManager::new();
        // TODO: Use relative coords in ScreenPos - x: [0.0, 1.0], y: [0.0, 1.0]
        // TODO: Add analog of Qt::Alignment
        let button_start_id = button_manager.add_button(Button::new(
            context,
            "start",
            ScreenPos{v: Vector2{x: 10, y: 10}})
        );
        MainMenuScreen {
            button_manager: button_manager,
            button_start_id: button_start_id,
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
        if *button_id == self.button_start_id {
            let tactical_screen = Box::new(TacticalScreen::new(context));
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
        context.set_basic_color(&zgl::BLACK);
        self.button_manager.draw(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) {
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
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
