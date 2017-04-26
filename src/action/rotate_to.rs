use types::{WorldPos};
use geom;
use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RotateTo {
    node_id: NodeId,
    to: WorldPos,
}

impl RotateTo {
    pub fn new(node_id: NodeId, to: WorldPos) -> Self {
        Self {
            node_id: node_id,
            to: to,
        }
    }
}

impl Action for RotateTo {
    fn begin(&mut self, context: &mut ActionContext) {
        let node = context.scene.node_mut(self.node_id);
        let rot = geom::get_rot_angle(node.pos, self.to);
        node.rot = rot;
    }
}
