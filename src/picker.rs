// See LICENSE file for copyright and license details.

use cgmath::{Vector2};
use core_map::{MapPosIter};
use core_types::{ZInt, Size2, MapPos, /*UnitId*/};
// use core::fs::FileSystem;
// use game_state::GameState;
use zgl::{Zgl};
use camera::{Camera};
use geom;
use mesh::{Mesh};
use visualizer_types::{Color3, ZFloat, VertexCoord, ScreenPos};
use shader::{Shader};
use dir::{DirIter};

static VS_SRC: &'static str = "\
    #version 100\n\
    uniform mat4 mvp_mat;\n\
    attribute vec3 position;\n\
    attribute vec3 a_color;\n\
    varying vec3 pass_color;\n\
    void main() {\n\
        gl_Position = mvp_mat * vec4(position, 1.0);\n\
        pass_color = a_color;\n\
    }\n\
";

static FS_SRC: &'static str = "\
    #version 100\n\
    precision mediump float;\n\
    varying vec3 pass_color;\n\
    void main() {\n\
        gl_FragColor = vec4(pass_color, 1);\n\
    }\n\
";

const PICK_CODE_NOTHING: ZInt = 0;
const PICK_CODE_MAP_POS: ZInt = 1;
// const PICK_CODE_UNIT: ZInt = 2;

fn i_to_f(n: ZInt) -> f32 {
    n as ZFloat / 255.0
}

pub enum PickResult {
    MapPos(MapPos),
    // UnitId(UnitId),
    Nothing
}

pub struct TilePicker {
    shader: Shader,
    mesh: Mesh,
    // map_size: Size2<ZInt>,
}

/*
fn tile_color(state: &GameState, pos: &MapPos) -> Color3 {
    let mut unit = None;
    for (_, unit2) in state.units.iter() {
        if unit2.pos == pos {
            unit = Some(unit2);
        }
    }
    match unit {
        Some(unit) => {
            Color3{r: i_to_f(unit.id.id), g: 0.0, b: i_to_f(PICK_CODE_UNIT)}
        },
        None => {
            let col_x = i_to_f(pos.v.x);
            let col_y = i_to_f(pos.v.y);
            Color3{r: col_x, g: col_y, b: i_to_f(PICK_CODE_MAP_POS)}
        },
    }
}
*/

fn tile_color(/*state: &GameState,*/ pos: &MapPos) -> Color3 {
    let col_x = i_to_f(pos.v.x);
    let col_y = i_to_f(pos.v.y);
    Color3{r: col_x, g: col_y, b: i_to_f(PICK_CODE_MAP_POS)}
}

fn get_mesh(zgl: &Zgl, /*state: &GameState,*/ map_size: &Size2<ZInt>) -> Mesh {
    let mut c_data = Vec::new();
    let mut v_data = Vec::new();
    for tile_pos in MapPosIter::new(map_size) {
        let pos = geom::map_pos_to_world_pos(&tile_pos);
        for dir in DirIter::new() {
            let num = dir.to_int();
            let vertex = geom::index_to_hex_vertex(num);
            let next_vertex = geom::index_to_hex_vertex(num + 1);
            let color = tile_color(/*state,*/ &tile_pos);
            v_data.push(VertexCoord{v: pos.v + vertex.v});
            c_data.push(color.clone());
            v_data.push(VertexCoord{v: pos.v + next_vertex.v});
            c_data.push(color.clone());
            v_data.push(VertexCoord{v: pos.v});
            c_data.push(color.clone());
        }
    }
    let mut mesh = Mesh::new(zgl, v_data.as_slice());
    mesh.add_colors(zgl, c_data.as_slice());
    mesh
}

impl TilePicker {
    // fs: &FileSystem, state: &GameState
    pub fn new(zgl: &Zgl, map_size: &Size2<ZInt>) -> TilePicker {
        let mut shader = Shader::new(zgl, VS_SRC, FS_SRC);
        shader.enable_color(zgl);
        shader.activate(zgl);
        let mesh = get_mesh(zgl, /*state,*/ map_size);
        let tile_picker = TilePicker {
            mesh: mesh,
            shader: shader,
            // map_size: map_size.clone(),
        };
        tile_picker
    }

    /*
    pub fn update_units(&mut self, state: &GameState) {
        self.mesh = get_mesh(state, self.map_size, &self.shader);
    }
    */

    pub fn pick_tile(
        &mut self,
        zgl: &mut Zgl,
        camera: &Camera,
        win_size: &Size2<ZInt>,
        mouse_pos: &ScreenPos,
    ) -> PickResult {
        self.shader.activate(zgl);
        let bg_color = Color3{r: 0.0, g: 0.0, b: 0.0};
        zgl.set_clear_color(&bg_color);
        zgl.clear_screen();
        self.shader.set_uniform_mat4f(
            zgl, self.shader.get_mvp_mat(), &camera.mat(zgl));
        self.mesh.draw(zgl, &self.shader);
        let (r, g, b, _) = zgl.read_pixel_bytes(win_size, mouse_pos);
        match b {
            PICK_CODE_NOTHING => PickResult::Nothing,
            PICK_CODE_MAP_POS => PickResult::MapPos(MapPos{v: Vector2{x: r, y: g}}),
            // PICK_CODE_UNIT => PickResult::UnitId(UnitId{id: r}),
            bad_tag => {
                println!("Picker: bad color tag: {}", bad_tag);
                PickResult::Nothing
            },
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
