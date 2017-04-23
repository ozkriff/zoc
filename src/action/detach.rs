use cgmath::{Rad};
use types::{WorldPos};
use scene::{NodeId};
use geom;
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct Detach {
    rot: Rad<f32>,
    transporter_node_id: NodeId,
}

impl Detach {
    pub fn new_from_to(
        transporter_node_id: NodeId,
        from: WorldPos,
        to: WorldPos,
    ) -> Box<Action> {
        let rot = geom::get_rot_angle(from, to);
        Self::new(transporter_node_id, rot)
    }

    pub fn new(transporter_node_id: NodeId, rot: Rad<f32>) -> Box<Action> {
        Box::new(Self {
            rot: rot,
            transporter_node_id: transporter_node_id,
        })
    }
}

// TODO: It seems to me that this action does too much and can be splitted apart
impl Action for Detach {
    fn begin(&mut self, context: &mut ActionContext) {
        let scene = &mut context.scene;
        let attached_node_id;
        let transporter_model_node_id;
        {
            let transporter_node = scene.node_mut(self.transporter_node_id);
            transporter_node.rot = self.rot; // TODO: use `RotateTo` action?
            transporter_model_node_id = transporter_node.children[0];
            attached_node_id = transporter_node.children[2];
        }
        scene.node_mut(transporter_model_node_id).pos.v.y = 0.0;
        scene.detach_node(self.transporter_node_id, attached_node_id);
        scene.remove_node(attached_node_id);
    }
}
