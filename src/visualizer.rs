use std::{process, thread, time};
use std::sync::mpsc::{channel, Receiver};
use std::fs::{metadata};
use glutin::Event;
use screen::{Screen, ScreenCommand, EventStatus};
use context::{Context};
use main_menu_screen::{MainMenuScreen};
use types::{Time};

#[cfg(not(target_os = "android"))]
fn check_assets_dir() {
    if let Err(e) = metadata("assets") {
        println!("Can`t find 'assets' dir: {}", e);
        println!("Note: see 'Assets' section of README.rst");
        process::exit(1);
    }
}

#[cfg(target_os = "android")]
fn check_assets_dir() {}

pub struct Visualizer {
    screens: Vec<Box<Screen>>,
    popups: Vec<Box<Screen>>,
    should_close: bool,
    last_time: Time,
    context: Context,
    rx: Receiver<ScreenCommand>,
}

impl Visualizer {
    pub fn new() -> Visualizer {
        check_assets_dir();
        let (tx, rx) = channel();
        let mut context = Context::new(tx);
        let screens = vec![
            Box::new(MainMenuScreen::new(&mut context)) as Box<Screen>,
        ];
        let last_time = context.current_time();
        Visualizer {
            screens: screens,
            popups: Vec::new(),
            should_close: false,
            last_time: last_time,
            context: context,
            rx: rx,
        }
    }

    pub fn tick(&mut self) {
        let max_fps = 60;
        let max_frame_time = time::Duration::from_millis(1000 / max_fps);
        let start_frame_time = time::Instant::now();
        self.draw();
        self.handle_events();
        self.handle_commands();
        let delta_time = start_frame_time.elapsed();
        if max_frame_time > delta_time {
            thread::sleep(max_frame_time - delta_time);
        }
    }

    fn draw(&mut self) {
        let dtime = self.update_time();
        self.context.clear();
        {
            let screen = self.screens.last_mut().unwrap();
            screen.tick(&mut self.context, dtime);
        }
        for popup in &mut self.popups {
            popup.tick(&mut self.context, dtime);
        }
        self.context.flush();
    }

    fn handle_events(&mut self) {
        let events = self.context.poll_events();
        for event in &events {
            let event = match event {
                Event::WindowEvent { ref event, ..} => event,
            };
            self.context.handle_event_pre(event);
            let mut event_status = EventStatus::NotHandled;
            for i in (0 .. self.popups.len()).rev() {
                event_status = self.popups[i].handle_event(
                    &mut self.context, event);
                if event_status == EventStatus::Handled {
                    break;
                }
            }
            if event_status == EventStatus::NotHandled {
                let screen = self.screens.last_mut().unwrap();
                screen.handle_event(&mut self.context, event);
            }
            self.context.handle_event_post(event);
        }
    }

    fn handle_commands(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                ScreenCommand::PushScreen(screen) => {
                    self.screens.push(screen);
                },
                ScreenCommand::PushPopup(popup) => {
                    self.popups.push(popup);
                },
                ScreenCommand::PopScreen => {
                    self.screens.pop().unwrap();
                    if self.screens.is_empty() {
                        self.should_close = true;
                    }
                    self.popups.clear();
                },
                ScreenCommand::PopPopup => {
                    assert!(self.popups.len() > 0);
                    let _ = self.popups.pop();
                },
            }
        }
    }

    pub fn is_running(&self) -> bool {
        !self.should_close && !self.context.should_close()
    }

    fn update_time(&mut self) -> Time {
        let time = self.context.current_time();
        let dtime = Time{n: time.n - self.last_time.n};
        self.last_time = time;
        dtime
    }
}
