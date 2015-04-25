// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use common::types::{ZInt, ZFloat, MapPos};
use zgl::zgl::{Zgl};
use zgl::mesh::{Mesh};
use zgl::camera::Camera;
use zgl::shader::{Shader};
use zgl::font_stash::{FontStash};
use zgl::types::{Time, WorldPos};
use geom;
use move_helper::{MoveHelper};

struct MapText {
    move_helper: MoveHelper,
    mesh: Mesh,
}

pub struct MapTextManager {
    meshes: HashMap<ZInt, MapText>,
    scale: ZFloat,
    i: ZInt,
}

impl MapTextManager {
    pub fn new(font_stash: &mut FontStash) -> Self {
        MapTextManager {
            meshes: HashMap::new(),
            scale: 0.5 / font_stash.get_size(),
            i: 0,
        }
    }

    pub fn add_text(
        &mut self,
        zgl: &Zgl,
        font_stash: &mut FontStash,
        text: &str,
        pos: &MapPos,
    ) {
        let pos = geom::map_pos_to_world_pos(pos);
        self.add_text_to_world_pos(zgl, font_stash, text, &pos);
    }

    pub fn add_text_to_world_pos(
        &mut self,
        zgl: &Zgl,
        font_stash: &mut FontStash,
        text: &str,
        pos: &WorldPos,
    ) {
        let from = pos;
        let mut to = from.clone();
        to.v.z += 2.0;
        let mesh = font_stash.get_mesh(zgl, text, 1.0, true);
        self.meshes.insert(self.i, MapText {
            mesh: mesh,
            move_helper: MoveHelper::new(from, &to, 1.0),
        });
        self.i += 1;
    }


    fn delete_old(&mut self) {
        let mut bad_keys = Vec::new();
        for (key, map_text) in &self.meshes {
            if map_text.move_helper.is_finished() {
                bad_keys.push(*key);
            }
        }
        for key in &bad_keys {
            self.meshes.remove(key);
        }
    }

    pub fn draw(
        &mut self,
        zgl: &Zgl,
        camera: &Camera,
        shader: &Shader,
        dtime: &Time,
    ) {
        // use gl; // TODO: ???
        // unsafe {
        //     zgl.gl.Disable(gl::DEPTH_TEST);
        // }
        for (_, map_text) in self.meshes.iter_mut() {
            let pos = map_text.move_helper.step(dtime);
            let m = camera.mat(zgl);
            let m = zgl.tr(m, &pos.v);
            let m = zgl.scale(m, self.scale);
            let m = zgl.rot_z(m, camera.get_z_angle());
            let m = zgl.rot_x(m, camera.get_x_angle());
            shader.set_uniform_mat4f(zgl, shader.get_mvp_mat(), &m);
            map_text.mesh.draw(zgl, shader);
        }
        // unsafe {
        //     zgl.gl.Enable(gl::DEPTH_TEST);
        // }
        self.delete_old();
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
