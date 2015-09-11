// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector3};
use common::types::{ZInt, Size2, ZFloat};
use zgl::types::{ScreenPos, MatId};
use zgl::shader::{Shader};
use zgl::font_stash::{FontStash};
use zgl::mesh::{Mesh};
use zgl::zgl::{Zgl};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ButtonId {pub id: ZInt}

pub struct Button {
    pos: ScreenPos,
    size: Size2,
    mesh: Mesh,
}

impl Button {
    pub fn new(
        zgl: &Zgl,
        win_size: &Size2,
        label: &str,
        font_stash: &mut FontStash,
        pos: ScreenPos,
    ) -> Button {
        let text_size = (win_size.h as ZFloat) / 400.0; // TODO: 400?
        let (_, size) = font_stash.get_text_size(zgl, label);
        Button {
            pos: pos,
            size: Size2 {
                w: (size.w as ZFloat * text_size) as ZInt,
                h: (size.h as ZFloat * text_size) as ZInt,
            },
            mesh: font_stash.get_mesh(zgl, label, text_size, false),
        }
    }

    pub fn draw(&self, zgl: &Zgl, shader: &Shader) {
        self.mesh.draw(zgl, shader);
    }

    pub fn pos(&self) -> &ScreenPos {
        &self.pos
    }

    pub fn size(&self) -> &Size2 {
        &self.size
    }
}

pub struct ButtonManager {
    buttons: HashMap<ButtonId, Button>,
    last_id: ButtonId,
}

impl ButtonManager {
    pub fn new() -> ButtonManager {
        ButtonManager {
            buttons: HashMap::new(),
            last_id: ButtonId{id: 0},
        }
    }

    pub fn buttons(&self) -> &HashMap<ButtonId, Button> {
        &self.buttons
    }

    pub fn add_button(&mut self, button: Button) -> ButtonId {
        let id = self.last_id.clone();
        self.buttons.insert(id.clone(), button);
        self.last_id.id += 1;
        id
    }

    // TODO: context: &Context
    pub fn get_clicked_button_id(
        &self,
        mouse_pos: &ScreenPos,
        win_size: &Size2,
    ) -> Option<ButtonId> {
        let x = mouse_pos.v.x;
        let y = win_size.h - mouse_pos.v.y;
        for (id, button) in self.buttons() {
            if x >= button.pos().v.x
                && x <= button.pos().v.x + button.size().w
                && y >= button.pos().v.y
                && y <= button.pos().v.y + button.size().h
            {
                return Some(id.clone());
            }
        }
        None
    }

    // TODO: context: &Context
    pub fn draw(
        &self,
        zgl: &Zgl,
        win_size: &Size2,
        shader: &Shader,
        mvp_mat_id: &MatId,
    ) {
        let m = zgl.get_2d_screen_matrix(win_size);
        for (_, button) in self.buttons() {
            let text_offset = Vector3 {
                x: button.pos().v.x as ZFloat,
                y: button.pos().v.y as ZFloat,
                z: 0.0,
            };
            shader.set_uniform_mat4f(
                zgl, mvp_mat_id, &zgl.tr(m, &text_offset));
            button.draw(zgl, shader);
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
