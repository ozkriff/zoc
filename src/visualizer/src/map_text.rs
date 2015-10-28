// See LICENSE file for copyright and license details.

use std::collections::{HashMap, VecDeque};
use common::types::{ZInt, ZFloat, MapPos};
use zgl::{Zgl};
use zgl::mesh::{Mesh};
use zgl::camera::Camera;
use zgl::font_stash::{FontStash};
use zgl::types::{Time};
use geom;
use move_helper::{MoveHelper};
use context::{Context};

struct ShowTextCommand {
    pos: MapPos,
    text: String,
}

struct MapText {
    move_helper: MoveHelper,
    mesh: Mesh,
    pos: MapPos,
}

pub struct MapTextManager {
    commands: VecDeque<ShowTextCommand>,
    visible_labels_list: HashMap<ZInt, MapText>,
    scale: ZFloat,
    last_label_id: ZInt, // TODO: think about better way of deleting old labels
}

impl MapTextManager {
    pub fn new(font_stash: &mut FontStash) -> Self {
        MapTextManager {
            commands: VecDeque::new(),
            visible_labels_list: HashMap::new(),
            scale: 0.5 / font_stash.get_size(),
            last_label_id: 0,
        }
    }

    pub fn add_text(&mut self, pos: &MapPos, text: &str) {
        self.commands.push_back(ShowTextCommand {
            pos: pos.clone(),
            text: text.to_string(),
        });
    }

    fn can_show_text_here(&self, pos: &MapPos) -> bool {
        let min_progress = 0.3;
        for (_, map_text) in &self.visible_labels_list {
            let progress = map_text.move_helper.progress();
            if map_text.pos == *pos && progress < min_progress {
                return false;
            }
        }
        true
    }

    pub fn do_commands(&mut self, zgl: &Zgl, font_stash: &mut FontStash) {
        let mut postponed_commands = Vec::new();
        while !self.commands.is_empty() {
            let command = self.commands.pop_front()
                .expect("MapTextManager: Can`t get next command");
            if !self.can_show_text_here(&command.pos) {
                postponed_commands.push(command);
                continue;
            }
            let from = geom::map_pos_to_world_pos(&command.pos);
            let mut to = from.clone();
            to.v.z += 2.0;
            let mesh = font_stash.get_mesh(zgl, &command.text, 1.0, true);
            self.visible_labels_list.insert(self.last_label_id, MapText {
                pos: command.pos.clone(),
                mesh: mesh,
                move_helper: MoveHelper::new(&from, &to, 1.0),
            });
            self.last_label_id += 1;
        }
        self.commands.extend(postponed_commands);
    }

    fn delete_old(&mut self) {
        let mut bad_keys = Vec::new();
        for (key, map_text) in &self.visible_labels_list {
            if map_text.move_helper.is_finished() {
                bad_keys.push(*key);
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
        dtime: &Time,
    ) {
        self.do_commands(&context.zgl, &mut context.font_stash);
        // TODO: I'm not sure that disabling depth test is correct solution
        context.zgl.set_depth_test(false);
        for (_, map_text) in self.visible_labels_list.iter_mut() {
            let pos = map_text.move_helper.step(dtime);
            let m = camera.mat(&context.zgl);
            let m = context.zgl.tr(m, &pos.v);
            let m = context.zgl.scale(m, self.scale);
            let m = context.zgl.rot_z(m, camera.get_z_angle());
            let m = context.zgl.rot_x(m, camera.get_x_angle());
            context.shader.set_uniform_mat4f(
                &context.zgl, context.shader.get_mvp_mat(), &m);
            map_text.mesh.draw(&context.zgl, &context.shader);
        }
        context.zgl.set_depth_test(true);
        self.delete_old();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
