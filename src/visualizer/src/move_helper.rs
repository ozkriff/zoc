// See LICENSE file for copyright and license details.

use cgmath::{Vector3, Vector, EuclideanVector};
use common::types::{ZFloat};
use zgl::types::{Time, WorldPos};
use geom;

pub struct MoveHelper {
    to: WorldPos,
    current: WorldPos,
    dist: ZFloat,
    current_dist: ZFloat,
    dir: Vector3<ZFloat>,
}

impl MoveHelper {
    // TODO: speed: ZFloat -> Speed (add 'Speed' to src/visualizer/types.rs
    pub fn new(from: &WorldPos, to: &WorldPos, speed: ZFloat) -> MoveHelper {
        let dir = to.v.sub_v(&from.v).normalize();
        let dist = geom::dist(from, to);
        MoveHelper {
            to: to.clone(),
            current: from.clone(),
            dist: dist,
            current_dist: 0.0,
            dir: dir.mul_s(speed),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.current_dist >= self.dist
    }

    pub fn step(&mut self, dtime: &Time) -> WorldPos {
        let _ = self.step_diff(dtime);
        self.current.clone()
    }

    pub fn step_diff(&mut self, dtime: &Time) -> Vector3<ZFloat> {
        let dt = dtime.n as ZFloat / 1000000000.0;
        let step = self.dir.mul_s(dt);
        self.current_dist += step.length();
        self.current.v.add_self_v(&step);
        if self.is_finished() {
            self.current = self.to.clone();
        }
        step
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
