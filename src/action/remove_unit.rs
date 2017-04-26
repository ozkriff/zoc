use core::unit::{UnitId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RemoveUnit {
    unit_id: UnitId,
}

impl RemoveUnit {
    pub fn new(unit_id: UnitId) -> Self {
        Self {
            unit_id: unit_id,
        }
    }
}

impl Action for RemoveUnit {
    fn begin(&mut self, context: &mut ActionContext) {
        // TODO: check something?
        context.scene.remove_unit(self.unit_id);
    }
}
