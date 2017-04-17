use core::object::{ObjectId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RemoveObject {
    object_id: ObjectId,
}

impl RemoveObject {
    pub fn new(object_id: ObjectId) -> Box<Action> {
        Box::new(Self {
            object_id: object_id,
        })
    }
}

impl Action for RemoveObject {
    fn begin(&mut self, context: &mut ActionContext) {
        context.scene.remove_object(self.object_id);
    }
}
