// See LICENSE file for copyright and license details.

use core_types::{ZInt};
use visualizer_types::{VertexCoord, Color3};
use zgl::{Zgl, Vbo, MeshRenderMode};
use shader::{Shader};

pub struct Mesh {
    vertex_coords_vbo: Vbo,
    colors_vbo: Option<Vbo>,
    length: ZInt,
    mode: MeshRenderMode,
}

impl Mesh {
    pub fn new(zgl: &Zgl, data: &[VertexCoord]) -> Mesh {
        let length = data.len() as ZInt;
        let vertex_coords_vbo = Vbo::from_data(zgl, data);
        Mesh {
            vertex_coords_vbo: vertex_coords_vbo,
            colors_vbo: None,
            length: length,
            mode: MeshRenderMode::Triangles,
        }
    }

    pub fn add_colors(&mut self, zgl: &Zgl, colors: &[Color3]) {
        self.colors_vbo = Some(Vbo::from_data(zgl, colors));
    }

    pub fn draw(&self, zgl: &Zgl, shader: &Shader) {
        self.vertex_coords_vbo.bind(zgl);
        shader.prepare_pos(zgl);
        if self.colors_vbo.is_some() {
            let colors_vbo = self.colors_vbo.as_ref()
                .expect("Can`t get color vbo");
            colors_vbo.bind(zgl);
            shader.prepare_color(zgl);
        }
        zgl.draw_arrays(&self.mode, self.length);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
