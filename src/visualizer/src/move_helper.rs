// See LICENSE file for copyright and license details.

use cgmath::{Vector3, InnerSpace};
use geom;
use types::{WorldPos, Time};

pub struct MoveHelper {
    to: WorldPos,
    current: WorldPos,
    dist: f32,
    current_dist: f32,
    dir: Vector3<f32>,
}

impl MoveHelper {
    // TODO: speed: f32 -> Speed (add 'Speed' to src/visualizer/types.rs
    pub fn new(from: &WorldPos, to: &WorldPos, speed: f32) -> MoveHelper {
        let dir = (to.v - from.v).normalize();
        let dist = geom::dist(from, to);
        MoveHelper {
            to: *to,
            current: *from,
            dist: dist,
            current_dist: 0.0,
            dir: dir * speed,
        }
    }

    pub fn progress(&self) -> f32 {
        self.current_dist / self.dist
    }

    pub fn is_finished(&self) -> bool {
        self.current_dist >= self.dist
    }

    pub fn step(&mut self, dtime: &Time) -> WorldPos {
        let _ = self.step_diff(dtime);
        self.current
    }

    pub fn destination(&self) -> &WorldPos {
        &self.to
    }

    pub fn step_diff(&mut self, dtime: &Time) -> Vector3<f32> {
        let dt = dtime.n as f32 / 1000000000.0;
        let step = self.dir * dt;
        self.current_dist += step.magnitude();
        self.current.v = self.current.v + step; // TODO: update cgmath-rs version and replace to `+=`
        if self.is_finished() {
            self.current = self.to;
        }
        step
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
