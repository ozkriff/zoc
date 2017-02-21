use std::collections::{HashMap, VecDeque};
use cgmath::{Matrix4, Matrix3};
use core::position::{MapPos};
use types::{Time, Speed};
use camera::Camera;
use geom;
use move_helper::{MoveHelper};
use context::{Context};
use texture::{load_texture_raw};
use mesh::{Mesh};
use text;
use pipeline::{Vertex};

#[derive(Clone, Debug)]
struct ShowTextCommand {
    pos: MapPos,
    text: String,
}

#[derive(Clone, Debug)]
struct MapText {
    move_helper: MoveHelper,
    mesh: Mesh,
    pos: MapPos,
}

#[derive(Clone, Debug)]
pub struct MapTextManager {
    commands: VecDeque<ShowTextCommand>,
    visible_labels_list: HashMap<i32, MapText>,
    last_label_id: i32, // TODO: think about better way of deleting old labels
}

impl MapTextManager {
    pub fn new() -> Self {
        MapTextManager {
            commands: VecDeque::new(),
            visible_labels_list: HashMap::new(),
            last_label_id: 0,
        }
    }

    pub fn add_text(&mut self, pos: MapPos, text: &str) {
        self.commands.push_back(ShowTextCommand {
            pos: pos,
            text: text.to_owned(),
        });
    }

    fn can_show_text_here(&self, pos: MapPos) -> bool {
        let min_progress = 0.3;
        for map_text in self.visible_labels_list.values() {
            let progress = map_text.move_helper.progress();
            if map_text.pos == pos && progress < min_progress {
                return false;
            }
        }
        true
    }

    pub fn do_commands(&mut self, context: &mut Context) {
        let mut postponed_commands = Vec::new();
        while !self.commands.is_empty() {
            let command = self.commands.pop_front()
                .expect("MapTextManager: Can`t get next command");
            if !self.can_show_text_here(command.pos) {
                postponed_commands.push(command);
                continue;
            }
            let mut from = geom::map_pos_to_world_pos(command.pos);
            from.v.z += 0.5;
            let mut to = from;
            to.v.z += 2.0;
            let mesh = {
                let (size, texture_data) = text::text_to_texture(context.font(), 80.0, &command.text);
                let texture = load_texture_raw(context.factory_mut(), size, &texture_data);
                let scale_factor = 200.0; // TODO: take camera zoom into account
                let h_2 = (size.h as f32 / scale_factor) / 2.0;
                let w_2 = (size.w as f32 / scale_factor) / 2.0;
                let vertices = &[
                    Vertex{pos: [-w_2, -h_2, 0.0], uv: [0.0, 1.0]},
                    Vertex{pos: [-w_2, h_2, 0.0], uv: [0.0, 0.0]},
                    Vertex{pos: [w_2, -h_2, 0.0], uv: [1.0, 1.0]},
                    Vertex{pos: [w_2, h_2, 0.0], uv: [1.0, 0.0]},
                ];
                let indices = &[0,  1,  2,  1,  2,  3];
                Mesh::new(context, vertices, indices, texture)
            };
            let move_speed = Speed{n: 1.0};
            self.visible_labels_list.insert(self.last_label_id, MapText {
                pos: command.pos,
                mesh: mesh,
                move_helper: MoveHelper::new(from, to, move_speed),
            });
            self.last_label_id += 1;
        }
        self.commands.extend(postponed_commands);
    }

    fn delete_old(&mut self) {
        let mut bad_keys = Vec::new();
        for (&key, map_text) in &self.visible_labels_list {
            if map_text.move_helper.is_finished() {
                bad_keys.push(key);
            }
        }
        for key in &bad_keys {
            self.visible_labels_list.remove(key);
        }
    }

    pub fn draw(
        &mut self,
        context: &mut Context,
        camera: &Camera,
        dtime: Time,
    ) {
        self.do_commands(context);
        let rot_z_mat = Matrix4::from(Matrix3::from_angle_z(camera.get_z_angle()));
        let rot_x_mat = Matrix4::from(Matrix3::from_angle_x(camera.get_x_angle()));
        for map_text in self.visible_labels_list.values_mut() {
            // TODO: use https://github.com/orhanbalci/rust-easing
            let t = 0.8;
            let p = map_text.move_helper.progress();
            let alpha = if p > t {
                (1.0 - p) / (1.0 - t)
            } else {
                1.0
            };
            context.set_basic_color([0.0, 0.0, 0.0, alpha]);
            let pos = map_text.move_helper.step(dtime);
            let tr_mat = Matrix4::from_translation(pos.v);
            let mvp = camera.mat() * tr_mat * rot_z_mat * rot_x_mat;
            context.set_mvp(mvp);
            context.draw_mesh(&map_text.mesh);
        }
        self.delete_old();
    }
}
