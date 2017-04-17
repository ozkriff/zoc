use types::{Time};
use scene::{NodeId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct ChangeColor {
    node_id: NodeId,
    target_color: [f32; 4],
    start_color: [f32; 4],
    duration: Time,
    time: Time,
}

impl ChangeColor {
    pub fn new(node_id: NodeId, color: [f32; 4], duration: Time) -> Box<Action> {
        Box::new(Self {
            node_id: node_id,
            target_color: color,
            start_color: [0.0, 0.0, 0.0, 0.0],
            duration: duration,
            time: Time{n: 0.0},
        })
    }
}

impl Action for ChangeColor {
    fn begin(&mut self, context: &mut ActionContext) {
        let node = context.scene.node(self.node_id);
        self.start_color = node.color;
    }

    fn update(&mut self, context: &mut ActionContext, dtime: Time) {
        self.time.n += dtime.n;
        let k = self.time.n / self.duration.n;
        let node = context.scene.node_mut(self.node_id);
        for i in 0..4 {
            let diff = self.target_color[i] - self.start_color[i];
            node.color[i] = self.start_color[i] + diff * k;
        }
    }

    fn is_finished(&self) -> bool {
        self.time.n > self.duration.n
    }

    fn end(&mut self, context: &mut ActionContext) {
        let node = context.scene.node_mut(self.node_id);
        node.color = self.target_color;
    }
}
