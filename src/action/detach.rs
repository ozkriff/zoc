use types::{WorldPos};
use scene::{NodeId};
use geom;
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Detach {
    from: WorldPos,
    to: WorldPos,
    transporter_node_id: NodeId,
    // attached_unit_id: UnitId,
}

impl Detach {
    pub fn new(
        from: WorldPos,
        to: WorldPos,
        transporter_node_id: NodeId,
    ) -> Box<Action> {
        Box::new(Self {
            from: from,
            to: to,
            transporter_node_id: transporter_node_id,
        })
    }
}

impl Action for Detach {
    fn begin(&mut self, context: &mut ActionContext) {
        let transporter_node = context.scene.node_mut(self.transporter_node_id);
        transporter_node.rot = geom::get_rot_angle(self.from, self.to);
        transporter_node.children[0].pos.v.y = 0.0;
        transporter_node.children.pop();
    }
}
