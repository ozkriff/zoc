use types::{WorldPos, Time, Speed};
use move_helper::{MoveHelper};
use scene::{NodeId};
use action::{Action, ActionContext};

// TODO: join with MoveHelper?
#[derive(Debug)]
pub struct MoveTo {
    node_id: NodeId,
    speed: Speed,
    to: WorldPos,

    // TODO: use builder pattern?
    // Or maybe I could fix this by changing the MoveHelper's logic.
    // Oooor use move_helper's mutability!
    move_helper: Option<MoveHelper>,
}

impl MoveTo {
    pub fn new(
        node_id: NodeId,
        speed: Speed,
        to: WorldPos,
    ) -> Self {
        Self {
            node_id: node_id,
            speed: speed,
            to: to,
            move_helper: None,
        }
    }
}

impl Action for MoveTo {
    fn begin(&mut self, context: &mut ActionContext) {
        let node = context.scene.node_mut(self.node_id);
        self.move_helper = Some(MoveHelper::new(
            node.pos, self.to, self.speed));

        // TODO: get from MoveHelper?
        //
        // TODO: не факт, что это тут стит делать,
        // пущай отдельное действие поворотом занимается
        // let rot = geom::get_rot_angle(node.pos, self.to);
        // node.rot = rot;
    }

    fn update(&mut self, context: &mut ActionContext, dtime: Time) {
        let pos = self.move_helper.as_mut().unwrap().step(dtime);
        context.scene.node_mut(self.node_id).pos = pos;
    }

    fn is_finished(&self) -> bool {
        self.move_helper.as_ref().unwrap().is_finished()
    }

    fn end(&mut self, context: &mut ActionContext) {
        context.scene.node_mut(self.node_id).pos = self.to;
    }
}
