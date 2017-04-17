use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct SetColor {
    node_id: NodeId,
    color: [f32; 4],
}

impl SetColor {
    pub fn new(node_id: NodeId, color: [f32; 4]) -> Box<Action> {
        Box::new(Self {
            node_id: node_id,
            color: color,
        })
    }
}

impl Action for SetColor {
    fn begin(&mut self, context: &mut ActionContext) {
        context.scene.node_mut(self.node_id).color = self.color;
    }
}
