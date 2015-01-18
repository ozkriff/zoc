// See LICENSE file for copyright and license details.

use std::num::Float;
use cgmath::{perspective, deg, Matrix4, Vector3, Deg, ToRad};
use core::types::{ZInt, Size2};
use core::misc::{clamp};
use visualizer::zgl::Zgl;
use visualizer::types::{ZFloat, WorldPos};

pub struct Camera {
    x_angle: Deg<ZFloat>,
    z_angle: Deg<ZFloat>,
    pos: WorldPos,
    max_pos: WorldPos,
    zoom: ZFloat,
    projection_mat: Matrix4<ZFloat>,
}

fn get_projection_mat(win_size: &Size2<ZInt>) -> Matrix4<ZFloat> {
    let fov = deg(45.0f32);
    let ratio = win_size.w as ZFloat / win_size.h as ZFloat;
    let display_range_min = 0.1;
    let display_range_max = 100.0;
    perspective(
        fov, ratio, display_range_min, display_range_max)
}

impl Camera {
    pub fn new(win_size: &Size2<ZInt>) -> Camera {
        Camera {
            x_angle: deg(45.0),
            z_angle: deg(0.0),
            pos: WorldPos{v: Vector3::from_value(0.0)},
            max_pos: WorldPos{v: Vector3::from_value(0.0)},
            zoom: 10.0,
            projection_mat: get_projection_mat(win_size),
        }
    }

    pub fn mat(&self, zgl: &Zgl) -> Matrix4<ZFloat> {
        let mut m = self.projection_mat;
        m = zgl.tr(m, Vector3{x: 0.0, y: 0.0, z: -self.zoom});
        m = zgl.rot_x(m, -self.x_angle);
        m = zgl.rot_z(m, -self.z_angle);
        m = zgl.tr(m, self.pos.v);
        m
    }

    pub fn add_horizontal_angle(&mut self, angle: Deg<ZFloat>) {
        self.z_angle = self.z_angle + angle; // TODO: cgmath: Deg: '+='
        while self.z_angle < deg(0.0) {
            self.z_angle = self.z_angle + deg(360.0);
        }
        while self.z_angle > deg(360.0) {
            self.z_angle = self.z_angle - deg(360.0);
        }
    }

    pub fn add_vertical_angle(&mut self, angle: Deg<ZFloat>) {
        self.x_angle = self.x_angle + angle;
        self.x_angle = clamp(self.x_angle, deg(10.0), deg(50.0));
    }

    fn clamp_pos(&mut self) {
        self.pos.v.x = clamp(self.pos.v.x, self.max_pos.v.x, 0.0);
        self.pos.v.y = clamp(self.pos.v.y, self.max_pos.v.y, 0.0);
    }

    /*
    pub fn set_pos(&mut self, pos: WorldPos) {
        self.pos = pos;
        self.clamp_pos();
    }
    */

    pub fn set_max_pos(&mut self, max_pos: WorldPos) {
        self.max_pos = max_pos;
    }

    pub fn change_zoom(&mut self, ratio: ZFloat) {
        self.zoom *= ratio;
        self.zoom = clamp(self.zoom, 5.0, 40.0);
    }

    pub fn move_camera(&mut self, angle: Deg<ZFloat>, speed: ZFloat) {
        let speed_in_radians = (self.z_angle - angle).to_rad().s;
        let dx = speed_in_radians.sin();
        let dy = speed_in_radians.cos();
        // TODO: handle zoom
        // self.pos.v.x -= dy * speed * self.zoom;
        // self.pos.v.y -= dx * speed * self.zoom;
        self.pos.v.x -= dy * speed;
        self.pos.v.y -= dx * speed;
        self.clamp_pos();
    }

    pub fn regenerate_projection_mat(&mut self, win_size: &Size2<ZInt>) {
        self.projection_mat = get_projection_mat(win_size);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
