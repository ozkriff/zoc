use std::collections::{VecDeque};
use types::{Time};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Sequence {
    actions: VecDeque<Box<Action>>,
}

impl Sequence {
    // TODO: Раз я все равно преобразую к VecDeque, может мне не вектор принимать?
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

    // TODO: косяк реализации в том, что некоторые действия могут быть мгновенными,
    // но займут все равно целый кадр.
    // Надо бы как-то тут цикл вставить.
    //
    // TODO: заменить логику в TacticalScreen на вот эту
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
}
