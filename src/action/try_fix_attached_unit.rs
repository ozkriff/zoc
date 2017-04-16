use std::f32::consts::{PI};
use cgmath::{Rad};
use core::unit::{UnitId};
use action::{Action, ActionContext};

// TODO: try to remove this hack
// TODO: rename?
#[derive(Debug)]
pub struct TryFixAttachedUnit {
    unit_id: UnitId,
    attached_unit_id: UnitId,
}

impl TryFixAttachedUnit {
    pub fn new(unit_id: UnitId, attached_unit_id: UnitId) -> Box<Action> {
        Box::new(Self {
            unit_id: unit_id,
            attached_unit_id: attached_unit_id,
        })
    }
}

impl Action for TryFixAttachedUnit {
    fn begin(&mut self, context: &mut ActionContext) {
        let scene = &mut context.scene;
        let transporter_node_id = scene.unit_id_to_node_id(self.unit_id);
        let attached_unit_node_id
            = match scene.unit_id_to_node_id_opt(self.attached_unit_id)
        {
            Some(id) => id,
            // this unit's scene node is already
            // attached to transporter's scene node
            None => return,
        };
        let mut attached_unit_node
            = scene.node_mut(attached_unit_node_id).children.remove(0);
        scene.remove_unit(self.attached_unit_id);
        attached_unit_node.pos.v.y = -0.5; // TODO: get from UnitTypeVisualInfo
        attached_unit_node.rot += Rad(PI);
        let transporter_node = scene.node_mut(transporter_node_id);
        transporter_node.children.push(attached_unit_node);
        transporter_node.children[0].pos.v.y = 0.5;
    }
}
