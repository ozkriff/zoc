use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct ChangeColor {
    node_id: NodeId,
    color: [f32; 4],
}

impl ChangeColor {
    pub fn new(node_id: NodeId, color: [f32; 4]) -> Box<Action> {
        Box::new(Self {
            node_id: node_id,
            color: color,
        })
    }
}

impl Action for ChangeColor {
    fn begin(&mut self, context: &mut ActionContext) {
        context.scene.node_mut(self.node_id).color = self.color;
    }
}
