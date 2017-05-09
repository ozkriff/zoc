use std::collections::{VecDeque};
use types::{Time};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Sequence {
    actions: VecDeque<Box<Action>>,
}

impl Sequence {
    // TODO: Maybe I should receive not Vec but VecDeque
    // if I convert this arg to VecDeque later anyway?
    pub fn new(actions: Vec<Box<Action>>) -> Self {
        Self {
            actions: actions.into(),
        }
    }

    /// Current action
    fn action(&mut self) -> &mut Action {
        &mut **self.actions.front_mut().unwrap() // TODO: Can this be simplified?
    }

    fn end_current_action_and_start_next(&mut self, context: &mut ActionContext) {
        assert!(!self.actions.is_empty());
        assert!(self.action().is_finished());
        self.action().end(context);
        self.actions.pop_front().unwrap();
        if !self.actions.is_empty() {
            self.action().begin(context);
        }
    }
}

impl Action for Sequence {
    fn begin(&mut self, context: &mut ActionContext) {
        if !self.actions.is_empty() {
            self.action().begin(context);
        }
    }

    fn update(&mut self, context: &mut ActionContext, dtime: Time) {
        if self.actions.is_empty() {
            return;
        }
        self.action().update(context, dtime);
        // Skipping instant actions
        while !self.actions.is_empty() && self.action().is_finished() {
            self.end_current_action_and_start_next(context);
        }
    }

    fn is_finished(&self) -> bool {
        self.actions.is_empty()
    }

    fn end(&mut self, _: &mut ActionContext) {
        assert!(self.actions.is_empty());
    }

    fn fork(&mut self, context: &mut ActionContext) -> Option<Box<Action>> {
        if self.actions.is_empty() {
            return None;
        }
        let forked_action = self.action().fork(context);
        if forked_action.is_some() && self.action().is_finished() {
            self.end_current_action_and_start_next(context);
        }
        forked_action
    }
}
