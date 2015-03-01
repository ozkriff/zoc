// See LICENSE file for copyright and license details.

use common::types::{ZInt};
use zgl::{Zgl, Vbo, MeshRenderMode};
use types::{VertexCoord, TextureCoord, Color3};
use shader::{Shader};
use texture::{Texture};

#[derive(Clone)]
pub struct MeshId{pub id: ZInt}

pub struct Mesh {
    vertex_coords_vbo: Vbo,
    colors_vbo: Option<Vbo>,
    texture_coords_vbo: Option<Vbo>,
    texture: Option<Texture>,
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
            texture_coords_vbo: None,
            texture: None,
            length: length,
            mode: MeshRenderMode::Triangles,
        }
    }

    pub fn add_colors(&mut self, zgl: &Zgl, colors: &[Color3]) {
        self.colors_vbo = Some(Vbo::from_data(zgl, colors));
    }

    pub fn add_texture(&mut self, zgl: &Zgl, texture: Texture, data: &[TextureCoord]) {
        assert_eq!(self.length, data.len() as ZInt);
        self.texture_coords_vbo = Some(Vbo::from_data(zgl, data));
        self.texture = Some(texture);
    }

    pub fn set_mode(&mut self, mode: MeshRenderMode) {
        self.mode = mode;
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
        if self.texture_coords_vbo.is_some() {
            let texture_coords_vbo = self.texture_coords_vbo.as_ref()
                .expect("Can`t get color vbo");
            texture_coords_vbo.bind(zgl);
            shader.prepare_texture_coords(zgl);
        }
        if let Some(ref texture) = self.texture {
            texture.enable(zgl, shader);
        }
        zgl.draw_arrays(&self.mode, self.length);
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
