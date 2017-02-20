use std::collections::{HashMap};
use cgmath::{Vector2, Vector3, Matrix4, ortho};
use context::{Context};
use texture::{load_texture_raw};
use types::{Size2, ScreenPos};
use text;
use mesh::{Mesh};
use pipeline::{Vertex};

static mut GLOBAL_GUI_ID: u32 = 0;

pub fn new_gui_id() -> GuiId {
    unsafe {
        GLOBAL_GUI_ID += 1;
        GuiId { id: GLOBAL_GUI_ID }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TextSize {
    Basic,
    Small,
}

pub fn point_in_rect(point: ScreenPos, loc: ScreenPos, dim: Size2) -> bool {
    let x = point.v.x;
    let y = point.v.y;
    x >= loc.v.x && x <= loc.v.x + dim.w
        && y >= loc.v.y && y <= loc.v.y + dim.h
}

/// Check if this was a tap or swipe
pub fn is_tap(context: &Context) -> bool {
    let mouse = context.mouse();
    let diff = mouse.pos.v - mouse.last_press_pos.v;
    let tolerance = 20; // TODO: read from config file
    diff.x.abs() < tolerance && diff.y.abs() < tolerance
}

pub fn basic_text_size(context: &Context) -> f32 {
    // TODO: use different value for android
    let lines_per_screen_h = 14.0;
    (context.win_size().h as f32) / lines_per_screen_h
}

pub fn small_text_size(context: &Context) -> f32 {
    basic_text_size(context) / 2.0
}

pub fn get_2d_screen_matrix(win_size: Size2) -> Matrix4<f32> {
    let left = 0.0;
    let right = win_size.w as f32;
    let bottom = 0.0;
    let top = win_size.h as f32;
    let near = -1.0;
    let far = 1.0;
    ortho(left, right, bottom, top, near, far)
}

pub fn text_to_mesh(context: &mut Context, size: TextSize, text: &str, color: &[u8; 4]) -> (Size2, Mesh) {
    let size = match size {
        TextSize::Basic => basic_text_size(context),
        TextSize::Small => small_text_size(context),
    };
    let (texture_size, texture_data) =
        text::text_to_texture(context.font(), size, text, &color);
    let texture = load_texture_raw(context.factory_mut(), texture_size, &texture_data);
    let h = texture_size.h as f32;
    let w = texture_size.w as f32;
    let vertices = &[
        Vertex{pos: [0.0, 0.0, 0.0], uv: [0.0, 1.0]},
        Vertex{pos: [0.0, h, 0.0], uv: [0.0, 0.0]},
        Vertex{pos: [w, 0.0, 0.0], uv: [1.0, 1.0]},
        Vertex{pos: [w, h, 0.0], uv: [1.0, 0.0]},
    ];
    let indices = &[0,  1,  2,  1,  2,  3];
    (texture_size, Mesh::new(context, vertices, indices, texture))
}

pub trait Widget {
    fn mouse_over(&self, context: &Context) -> bool;
    fn draw(&self, context: &mut Context);
    fn pos(&self) -> ScreenPos;
    fn set_pos<'a>(&'a mut self, pos: ScreenPos) -> &'a mut Self;
    fn size(&self) -> Size2;
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct GuiId {pub id: u32}

#[derive(Clone, Debug)]
pub struct Button {
    pos: ScreenPos,
    size: Size2,
    text: String,
    text_size: TextSize,
    default_mesh: Option<Mesh>,
    hover_mesh: Option<Mesh>,
}

impl Button {
    pub fn new(context: &mut Context, text: &str, pos: ScreenPos) -> Button {
        let mut b = Button::new2();
        b.text(text).set_pos(pos).build(context);
        b
    }

    pub fn new_small(context: &mut Context, text: &str, pos: ScreenPos) -> Button {
        let mut b = Button::new2();
        b.text(text)
            .text_size(TextSize::Small)
            .set_pos(pos)
            .build(context);
        b
    }

    pub fn new2() -> Button {
        Button {
            pos: ScreenPos{v: Vector2{ x: 0, y: 0}},
            size: Size2{w: 0, h: 0},
            text: String::new(),
            text_size: TextSize::Basic,
            default_mesh: None,
            hover_mesh: None
        }
    }

    pub fn text<'a, S: Into<String>>(&'a mut self, text: S) -> &'a mut Button {
        self.text = text.into();
        self
    }

    pub fn text_size<'a>(&'a mut self, size: TextSize) -> &'a mut Button {
        self.text_size = size;
        self
    }

    pub fn build(&mut self, context: &mut Context) {
        let (size, def_mesh) = text_to_mesh(context, self.text_size, self.text.as_str(), &[255, 0, 0, 255]);
        let (_, hov_mesh) = text_to_mesh(context, self.text_size, self.text.as_str(), &[0, 255, 255, 255]);
        self.default_mesh = Some(def_mesh);
        self.hover_mesh = Some(hov_mesh);
        self.size = size;
    }
}

impl Widget for Button {
    fn mouse_over(&self, context: &Context) -> bool {
        let x = context.mouse().pos.v.x;
        let y = context.win_size().h - context.mouse().pos.v.y;
        let mouse = ScreenPos { v: Vector2 { x: x, y: y} };
        point_in_rect(mouse, self.pos(), self.size())
    }

    fn set_pos<'a>(&'a mut self, pos: ScreenPos) -> &'a mut Button {
        self.pos = pos;
        self
    }

    fn pos(&self) -> ScreenPos {
        self.pos
    }

    fn size(&self) -> Size2 {
        self.size
    }

    fn draw(&self, context: &mut Context) {
        let mouse_over = self.mouse_over(context);
        let proj_mat = get_2d_screen_matrix(context.win_size());
        let tr_mat = Matrix4::from_translation(Vector3 {
            x: self.pos.v.x as f32,
            y: self.pos.v.y as f32,
            z: 0.0,
        });
        context.set_mvp(proj_mat * tr_mat);

        if mouse_over && self.hover_mesh.is_some() {
            context.draw_mesh(&self.hover_mesh.as_ref().unwrap());
        } else if self.default_mesh.is_some() {
            context.draw_mesh(self.default_mesh.as_ref().unwrap());
        }
    }
}

#[derive(Clone, Debug)]
pub struct ButtonManager {
    buttons: HashMap<GuiId, Button>,
    last_id: GuiId,
}

impl ButtonManager {
    pub fn new() -> ButtonManager {
        ButtonManager {
            buttons: HashMap::new(),
            last_id: GuiId{id: 0},
        }
    }

    pub fn buttons(&self) -> &HashMap<GuiId, Button> {
        &self.buttons
    }

    pub fn buttons_mut(&mut self) -> &mut HashMap<GuiId, Button> {
        &mut self.buttons
    }

    pub fn add_button(&mut self, button: Button) -> GuiId {
        let id = new_gui_id();
        self.buttons.insert(id, button);
        id
    }

    pub fn remove_button(&mut self, id: GuiId) {
        self.buttons.remove(&id).unwrap();
    }

    pub fn get_clicked_button_id(&self, context: &Context) -> Option<GuiId> {
        for (&id, button) in self.buttons() {
            if button.mouse_over(context) {
                return Some(id);
            }
        }
        None
    }

    pub fn draw(&self, context: &mut Context) {
        for button in self.buttons().values() {
            button.draw(context);
        }
    }
}
