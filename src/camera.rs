// See LICENSE file for copyright and license details.

use std::num::FloatMath;
use cgmath::{perspective, deg, Matrix4, Vector3};
use core_types::{MInt, Size2};
use core_misc::{clamp, deg_to_rad};
use mgl::Mgl;
use visualizer_types::{MFloat, WorldPos};

pub struct Camera {
    x_angle: MFloat, // TODO: MFloat -> Angle
    z_angle: MFloat,
    pos: WorldPos,
    max_pos: WorldPos,
    zoom: MFloat,
    projection_mat: Matrix4<MFloat>,
}

fn get_projection_mat(win_size: Size2<MInt>) -> Matrix4<MFloat> {
    let fov = deg(45.0f32);
    let ratio = win_size.w as MFloat / win_size.h as MFloat;
    let display_range_min = 0.1;
    let display_range_max = 100.0;
    perspective(
        fov, ratio, display_range_min, display_range_max)
}

impl Camera {
    pub fn new(win_size: Size2<MInt>) -> Camera {
        Camera {
            x_angle: 45.0,
            z_angle: 0.0,
            pos: WorldPos{v: Vector3::from_value(0.0)},
            max_pos: WorldPos{v: Vector3::from_value(0.0)},
            zoom: 10.0,
            projection_mat: get_projection_mat(win_size),
        }
    }

    pub fn mat(&self, mgl: &Mgl) -> Matrix4<MFloat> {
        let mut m = self.projection_mat;
        m = mgl.tr(m, Vector3{x: 0.0, y: 0.0, z: -self.zoom});
        m = mgl.rot_x(m, -self.x_angle);
        m = mgl.rot_z(m, -self.z_angle);
        m = mgl.tr(m, self.pos.v);
        m
    }

    // TODO: rename to 'add_horizontal_angle'
    pub fn add_z_angle(&mut self, angle: MFloat) {
        self.z_angle += angle;
        while self.z_angle < 0.0 {
            self.z_angle += 360.0;
        }
        while self.z_angle > 360.0 {
            self.z_angle -= 360.0;
        }
    }

    // TODO: rename to 'add_vertical_angle'
    pub fn add_x_angle(&mut self, angle: MFloat) {
        self.x_angle += angle;
        self.x_angle = clamp(self.x_angle, 30.0, 75.0);
    }

    fn clamp_pos(&mut self) {
        self.pos.v.x = clamp(self.pos.v.x, self.max_pos.v.x, 0.0);
        self.pos.v.y = clamp(self.pos.v.y, self.max_pos.v.y, 0.0);
    }

    pub fn set_pos(&mut self, pos: WorldPos) {
        self.pos = pos;
        self.clamp_pos();
    }

    pub fn set_max_pos(&mut self, max_pos: WorldPos) {
        self.max_pos = max_pos;
    }

    pub fn change_zoom(&mut self, ratio: MFloat) {
        self.zoom *= ratio;
        self.zoom = clamp(self.zoom, 5.0, 40.0);
    }

    pub fn move_camera(&mut self, angle: MFloat, speed: MFloat) {
        let speed_in_radians = deg_to_rad(self.z_angle - angle);
        let dx = speed_in_radians.sin();
        let dy = speed_in_radians.cos();
        self.pos.v.x -= dy * speed * self.zoom;
        self.pos.v.y -= dx * speed * self.zoom;
        self.clamp_pos();
    }

    pub fn regenerate_projection_mat(&mut self, win_size: Size2<MInt>) {
        self.projection_mat = get_projection_mat(win_size);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
