use core::object::{ObjectId};
use scene::{SceneNode, NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct AddObject {
    node_id: NodeId,
    object_id: ObjectId,
    node: SceneNode,
}

impl AddObject {
    pub fn new(
        object_id: ObjectId,
        node: SceneNode,
        node_id: NodeId,
    ) -> Box<Action> {
        Box::new(Self {
            object_id: object_id,
            node: node,
            node_id: node_id,
        })
    }
}

impl Action for AddObject {
    fn begin(&mut self, context: &mut ActionContext) {
        context.scene.add_object(
            self.node_id, self.object_id, self.node.clone());
    }
}
