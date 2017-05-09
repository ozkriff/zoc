use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Fork {
    action: Option<Box<Action>>,
}

impl Fork {
    pub fn new(action: Box<Action>) -> Self {
        Self {
            action: Some(action),
        }
    }
}

impl Action for Fork {
    fn fork(&mut self, _: &mut ActionContext) -> Option<Box<Action>> {
        self.action.take()
    }

    fn is_finished(&self) -> bool {
        self.action.is_none()
    }

    fn end(&mut self, _: &mut ActionContext) {
        assert!(self.action.is_none());
    }
}
