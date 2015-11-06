// See LICENSE file for copyright and license details.

use std::sync::mpsc::{Sender};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use zgl::{self, Time, ScreenPos};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap};

#[derive(Clone)]
pub enum Command {
    Test,
}

#[derive(Default)]
pub struct Options {
    pub test: bool,
}

pub struct TestPopup {
    game_screen_tx: Sender<Command>,
    button_manager: ButtonManager,
    test_button_id: ButtonId,
}

impl TestPopup {
    pub fn new(
        context: &mut Context,
        pos: &ScreenPos,
        _: Options,
        tx: Sender<Command>,
    ) -> TestPopup {
        let mut button_manager = ButtonManager::new();
        let test_button_id = button_manager.add_button(
            Button::new(context, "test", pos));
        TestPopup {
            game_screen_tx: tx,
            button_manager: button_manager,
            test_button_id: test_button_id,
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

    fn return_command(&self, context: &mut Context, command: Command) {
        self.game_screen_tx.send(command).unwrap();
        context.add_command(ScreenCommand::PopPopup);
    }

    fn handle_event_button_press(
        &mut self,
        context: &mut Context,
        button_id: &ButtonId
    ) {
        let id = button_id.clone();
        if id == self.test_button_id {
            self.return_command(context, Command::Test);
        } else {
            panic!("Bad button id: {}", id.id);
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

impl Screen for TestPopup {
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
            _ => event_status = EventStatus::NotHandled,
        }
        event_status
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
