use std::collections::{VecDeque};
use types::{Time};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Sequence {
    actions: VecDeque<Box<Action>>,
}

impl Sequence {
    // TODO: Maybe I should receive not Vec if
    // I convert it to VecDeque later anyway?
    pub fn new(actions: Vec<Box<Action>>) -> Self {
        Self {
            actions: actions.into(),
        }
    }
}

impl Action for Sequence {
    fn begin(&mut self, context: &mut ActionContext) {
        if !self.actions.is_empty() {
            self.actions.front_mut().unwrap().begin(context);
        }
    }

    // TODO: SIMPLIFY
    // TODO: Use some cycle to skip instant actions
    fn update(&mut self, context: &mut ActionContext, dtime: Time) {
        if !self.actions.is_empty() {
            self.actions.front_mut().unwrap().update(context, dtime);
            if self.actions.front_mut().unwrap().is_finished() {
                self.actions.front_mut().unwrap().end(context);
                self.actions.pop_front().unwrap();
                if !self.actions.is_empty() {
                    self.actions.front_mut().unwrap().begin(context);
                }
            }
        }
    }

    fn is_finished(&self) -> bool {
        self.actions.is_empty()
    }

    fn end(&mut self, _: &mut ActionContext) {
        assert!(self.actions.is_empty());
    }

    fn fork(&mut self) -> Option<Box<Action>> {
        if !self.actions.is_empty() {
            self.actions.front_mut().unwrap().fork()
        } else {
            None
        }
    }
}
