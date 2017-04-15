use std::f32::consts::{PI};
use cgmath::{perspective, Rad, Matrix4, Matrix3, Vector3, Array, Angle};
use core::misc::{clamp};
use types::{WorldPos, Size2};

#[derive(Clone, Debug)]
pub struct Camera {
    x_angle: Rad<f32>,
    z_angle: Rad<f32>,
    pos: WorldPos,
    max_pos: WorldPos,
    zoom: f32,
    projection_mat: Matrix4<f32>,
}

fn get_projection_mat(win_size: Size2) -> Matrix4<f32> {
    let fov = Rad(PI / 4.0);
    let ratio = win_size.w as f32 / win_size.h as f32;
    let display_range_min = 0.1;
    let display_range_max = 100.0;
    perspective(fov, ratio, display_range_min, display_range_max)
}

impl Camera {
    pub fn new(win_size: Size2) -> Camera {
        Camera {
            x_angle: Rad(PI / 4.0),
            z_angle: Rad(PI / 4.0),
            pos: WorldPos{v: Vector3::from_value(0.0)},
            max_pos: WorldPos{v: Vector3::from_value(0.0)},
            zoom: 25.0,
            projection_mat: get_projection_mat(win_size),
        }
    }

    pub fn mat(&self) -> Matrix4<f32> {
        let zoom_m = Matrix4::from_translation(Vector3{x: 0.0, y: 0.0, z: -self.zoom});
        let x_angle_m = Matrix4::from(Matrix3::from_angle_x(-self.x_angle));
        let z_angle_m = Matrix4::from(Matrix3::from_angle_z(-self.z_angle));
        let tr_m = Matrix4::from_translation(self.pos.v);
        self.projection_mat * zoom_m * x_angle_m * z_angle_m * tr_m
    }

    pub fn add_horizontal_angle(&mut self, angle: Rad<f32>) {
        self.z_angle = self.z_angle + angle;
        while self.z_angle < Rad(0.0) {
            self.z_angle = self.z_angle + Rad(PI * 2.0);
        }
        while self.z_angle > Rad(PI * 2.0) {
            self.z_angle = self.z_angle - Rad(PI * 2.0);
        }
    }

    pub fn add_vertical_angle(&mut self, angle: Rad<f32>) {
        self.x_angle = self.x_angle + angle;
        let min = Rad(PI / 18.0);
        let max = Rad(PI / 4.0);
        self.x_angle = clamp(self.x_angle, min, max);
    }

    fn clamp_pos(&mut self) {
        self.pos.v.x = clamp(self.pos.v.x, self.max_pos.v.x, 0.0);
        self.pos.v.y = clamp(self.pos.v.y, self.max_pos.v.y, 0.0);
    }

    pub fn pos(&self) -> WorldPos {
        self.pos
    }

    pub fn set_pos(&mut self, pos: WorldPos) {
        self.pos = pos;
        self.clamp_pos();
    }

    pub fn set_max_pos(&mut self, max_pos: WorldPos) {
        self.max_pos = max_pos;
    }

    pub fn change_zoom(&mut self, ratio: f32) {
        self.zoom *= ratio;
        self.zoom = clamp(self.zoom, 10.0, 40.0);
    }

    pub fn get_z_angle(&self) -> Rad<f32> {
        self.z_angle
    }

    // TODO: Do I need this getter?
    // pub fn get_x_angle(&self) -> Rad<f32> {
    //     self.x_angle
    // }

    pub fn move_in_direction(&mut self, direction: Rad<f32>, speed: f32) {
        let diff = self.z_angle - direction;
        let dx = diff.sin();
        let dy = diff.cos();
        // TODO: handle zoom
        // self.pos.v.x -= dy * speed * self.zoom;
        // self.pos.v.y -= dx * speed * self.zoom;
        self.pos.v.x -= dy * speed;
        self.pos.v.y -= dx * speed;
        self.clamp_pos();
    }

    pub fn regenerate_projection_mat(&mut self, win_size: Size2) {
        self.projection_mat = get_projection_mat(win_size);
    }
}
