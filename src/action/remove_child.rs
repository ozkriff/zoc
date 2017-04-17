use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RemoveChild {
    parent_id: NodeId,
    child_id: i32,
}

impl RemoveChild {
    pub fn new(parent_id: NodeId, child_id: i32) -> Box<Action> {
        Box::new(Self {
            parent_id: parent_id,
            child_id: child_id,
        })
    }
}

impl Action for RemoveChild {
    fn begin(&mut self, context: &mut ActionContext) {
        // TODO: checks? what could go wrong?
        let children = &mut context.scene.node_mut(self.parent_id).children;
        let _ = children.remove(self.child_id as usize);
    }
}
