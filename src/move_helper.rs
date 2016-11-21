use cgmath::{Vector3, InnerSpace};
use geom;
use types::{WorldPos, Time, Speed};

#[derive(Clone, Debug)]
pub struct MoveHelper {
    to: WorldPos,
    current: WorldPos,
    dist: f32,
    current_dist: f32,
    dir: Vector3<f32>,
}

impl MoveHelper {
    pub fn new(from: WorldPos, to: WorldPos, speed: Speed) -> MoveHelper {
        let dir = (to.v - from.v).normalize();
        let dist = geom::dist(from, to);
        MoveHelper {
            to: to,
            current: from,
            dist: dist,
            current_dist: 0.0,
            dir: dir * speed.n,
        }
    }

    pub fn progress(&self) -> f32 {
        self.current_dist / self.dist
    }

    pub fn is_finished(&self) -> bool {
        self.current_dist >= self.dist
    }

    pub fn step(&mut self, dtime: Time) -> WorldPos {
        let _ = self.step_diff(dtime);
        self.current
    }

    pub fn destination(&self) -> WorldPos {
        self.to
    }

    pub fn step_diff(&mut self, dtime: Time) -> Vector3<f32> {
        let step = self.dir * dtime.n as f32;
        self.current_dist += step.magnitude();
        self.current.v += step;
        if self.is_finished() {
            self.current = self.to;
        }
        step
    }
}
