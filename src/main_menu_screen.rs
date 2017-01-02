use cgmath::{Vector2};
use glutin::{self, Event, MouseButton, VirtualKeyCode};
use glutin::ElementState::{Released};
use screen::{Screen, ScreenCommand, EventStatus};
use tactical_screen::{TacticalScreen};
use core;
use context::{Context};
use gui::{ButtonManager, Button, ButtonId, is_tap};
use types::{ScreenPos, Time};

#[derive(Clone, Debug)]
pub struct MainMenuScreen {
    button_start_hotseat_id: ButtonId,
    button_start_vs_ai_id: ButtonId,
    button_map_id: ButtonId,
    button_manager: ButtonManager,
    map_names: Vec<&'static str>,
    selected_map_index: usize,
}

impl MainMenuScreen {
    pub fn new(context: &mut Context) -> MainMenuScreen {
        let map_names = vec!["map01", "map02", "map03", "map04", "map05"];
        let selected_map_index = 0;
        let mut button_manager = ButtonManager::new();
        // TODO: Use relative coords in ScreenPos - x: [0.0, 1.0], y: [0.0, 1.0]
        // TODO: Add analog of Qt::Alignment
        let mut button_pos = ScreenPos{v: Vector2{x: 10, y: 10}};
        let button_start_hotseat_id = button_manager.add_button(Button::new(
            context,
            "start hotseat",
            button_pos,
        ));
        // TODO: Add something like QLayout
        let vstep = button_manager.buttons()[&button_start_hotseat_id].size().h;
        button_pos.v.y += vstep;
        let button_start_vs_ai_id = button_manager.add_button(Button::new(
            context,
            "start human vs ai",
            button_pos,
        ));
        button_pos.v.y += vstep * 2;
        let button_map_id = button_manager.add_button(Button::new(
            context,
            &format!("map: {}", map_names[selected_map_index]),
            button_pos,
        ));
        MainMenuScreen {
            button_manager: button_manager,
            button_start_hotseat_id: button_start_hotseat_id,
            button_start_vs_ai_id: button_start_vs_ai_id,
            button_map_id: button_map_id,
            map_names: map_names,
            selected_map_index: selected_map_index,
        }
    }

    fn handle_event_lmb_release(&mut self, context: &mut Context) {
        if !is_tap(context) {
            return;
        }
        if let Some(button_id) = self.button_manager.get_clicked_button_id(context) {
            self.handle_event_button_press(context, button_id);
        }
    }

    fn handle_event_button_press(
        &mut self,
        context: &mut Context,
        button_id: ButtonId
    ) {
        let map_name = self.map_names[self.selected_map_index].to_string();
        let mut core_options = core::Options {
            game_type: core::GameType::Hotseat,
            map_name: map_name,
            players_count: 2,
        };
        if button_id == self.button_start_hotseat_id {
            let tactical_screen = Box::new(
                TacticalScreen::new(context, &core_options));
            context.add_command(ScreenCommand::PushScreen(tactical_screen));
        } else if button_id == self.button_start_vs_ai_id {
            core_options.game_type = core::GameType::SingleVsAi;
            let tactical_screen = Box::new(
                TacticalScreen::new(context, &core_options));
            context.add_command(ScreenCommand::PushScreen(tactical_screen));
        } else if button_id == self.button_map_id {
            self.selected_map_index += 1;
            if self.selected_map_index == self.map_names.len() {
                self.selected_map_index = 0;
            }
            let text = &format!("map: {}", self.map_names[self.selected_map_index]);
            let pos = self.button_manager.buttons()[&self.button_map_id].pos();
            let button_map = Button::new(context, text, pos);
            self.button_manager.remove_button(self.button_map_id);
            self.button_map_id = self.button_manager.add_button(button_map);
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
    fn tick(&mut self, context: &mut Context, _: Time) {
        context.clear();
        context.set_basic_color([0.0, 0.0, 0.0, 1.0]);
        self.button_manager.draw(context);
    }

    fn handle_event(&mut self, context: &mut Context, event: &Event) -> EventStatus {
        match *event {
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
            _ => {},
        }
        EventStatus::Handled
    }
}
