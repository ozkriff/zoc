use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RemoveNode {
    node_id: NodeId,
}

impl RemoveNode {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id: node_id,
        }
    }
}

impl Action for RemoveNode {
    fn begin(&mut self, context: &mut ActionContext) {
        // TODO: check something?
        context.scene.remove_node(self.node_id);
    }
}
