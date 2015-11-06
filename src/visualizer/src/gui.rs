// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Vector3};
use common::types::{ZInt, Size2, ZFloat};
use zgl::types::{ScreenPos};
use zgl::shader::{Shader};
use zgl::mesh::{Mesh};
use zgl::{Zgl};
use context::{Context};

/// Check if this was a tap or swipe
pub fn is_tap(context: &Context) -> bool {
    let mouse = context.mouse();
    let pos = &mouse.pos;
    let x = pos.v.x - mouse.last_press_pos.v.x;
    let y = pos.v.y - mouse.last_press_pos.v.y;
    let tolerance = 20;
    x.abs() < tolerance && y.abs() < tolerance
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ButtonId {pub id: ZInt}

pub struct Button {
    pos: ScreenPos,
    size: Size2,
    mesh: Mesh,
}

impl Button {
    pub fn new(context: &mut Context, label: &str, pos: &ScreenPos) -> Button {
        let text_size = (context.win_size.h as ZFloat) / 400.0; // TODO: 400?
        let (_, size) = context.font_stash.get_text_size(&context.zgl, label);
        Button {
            pos: pos.clone(),
            size: Size2 {
                w: (size.w as ZFloat * text_size) as ZInt,
                h: (size.h as ZFloat * text_size) as ZInt,
            },
            mesh: context.font_stash.get_mesh(&context.zgl, label, text_size, false),
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

    pub fn get_clicked_button_id(&self, context: &Context) -> Option<ButtonId> {
        let x = context.mouse().pos.v.x;
        let y = context.win_size.h - context.mouse().pos.v.y;
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

    pub fn draw(&self, context: &Context) {
        let m = context.zgl.get_2d_screen_matrix(&context.win_size);
        for (_, button) in self.buttons() {
            let text_offset = Vector3 {
                x: button.pos().v.x as ZFloat,
                y: button.pos().v.y as ZFloat,
                z: 0.0,
            };
            context.shader.set_uniform_mat4f(
                &context.zgl,
                context.shader.get_mvp_mat(),
                &context.zgl.tr(m, &text_offset));
            button.draw(&context.zgl, &context.shader);
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
