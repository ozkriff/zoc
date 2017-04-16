use action::{Action, ActionContext};
use types::{Time};

#[derive(Debug)]
pub struct Sleep {
    duration: Time,
    time: Time,
}

impl Sleep {
    pub fn new(duration: Time) -> Box<Action> {
        Box::new(Self {
            duration: duration,
            time: Time{n: 0.0},
        })
    }
}

impl Action for Sleep {
    fn is_finished(&self) -> bool {
        self.time.n / self.duration.n > 1.0
    }

    fn update(&mut self, _: &mut ActionContext, dtime: Time) {
        self.time.n += dtime.n;
    }
}
