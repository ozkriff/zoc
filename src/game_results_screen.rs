use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use core::{PlayerId, Score};
use core::game_state::{State};
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use gui::{ButtonManager, Button, is_tap};
use types::{ScreenPos, Time};

fn winner_id(state: &State) -> PlayerId {
    // TODO: `CoreEvent::GameEnd` event?
    let mut winner_id = PlayerId{id: 0};
    let mut winner_score = Score{n: 0};
    for (&id, &score) in state.score() {
        if score.n > winner_score.n {
            winner_id = id;
            winner_score = score;
        }
    }
    winner_id
}

#[derive(Clone, Debug)]
pub struct GameResultsScreen {
    button_manager: ButtonManager,
}

impl GameResultsScreen {
    pub fn new(context: &mut Context, state: &State) -> GameResultsScreen {
        let mut button_manager = ButtonManager::new();
        let wh = context.win_size().h;
        let mut pos = ScreenPos{v: Vector2{x: 10, y: wh -10}};
        pos.v.y -= wh / 10; // TODO: magic num
        let winner_index = winner_id(state);
        let str = format!("Player {} wins!", winner_index.id);
        let title_button = Button::new(context, &str, pos);
        pos.v.y -= title_button.size().h; // TODO: autolayout
        let _ = button_manager.add_button(title_button);
        for (player_index, player_score) in state.score() {
            let str = format!("Player {}: {} VPs", player_index.id, player_score.n);
            let button = Button::new(context, &str, pos);
            pos.v.y -= button.size().h;
            let _ = button_manager.add_button(button);
        }
        // TODO: button -> label + center on screen
        GameResultsScreen {
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

impl Screen for GameResultsScreen {
    fn tick(&mut self, context: &mut Context, _: Time) {
        context.set_basic_color([0.0, 0.0, 0.0, 1.0]);
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
