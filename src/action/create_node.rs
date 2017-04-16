use scene::{SceneNode, NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct CreateNode {
    node_id: NodeId,
    node: SceneNode,
}

impl CreateNode {
    pub fn new(node_id: NodeId, node: SceneNode) -> Box<Action> {
        Box::new(Self {
            node_id: node_id,
            node: node,
        })
    }
}

impl Action for CreateNode {
    fn begin(&mut self, context: &mut ActionContext) {
        // TODO: Can I get rid of this `.clone()` somehow?
        context.scene.set_node(self.node_id, self.node.clone());
    }
}
