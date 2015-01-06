// See LICENSE file for copyright and license details.

use std::ptr;
use core_types::{ZInt};
use visualizer_types::{VertexCoord};
use gl;
use gl::types::{GLuint};
use zgl::{Zgl, Vbo};
use shader::{Shader};

pub enum MeshRenderMode {
    Triangles,
    // Lines,
}

impl MeshRenderMode {
    pub fn to_gl_type(&self) -> GLuint {
        match *self {
            MeshRenderMode::Triangles => gl::TRIANGLES,
            // MeshRenderMode::Lines => gl::LINES,
        }
    }
}

pub struct Mesh {
    vertex_coords_vbo: Vbo,
    length: ZInt,
    mode: MeshRenderMode,
}

impl Mesh {
    pub fn new(zgl: &Zgl, data: &[VertexCoord]) -> Mesh {
        let length = data.len() as ZInt;
        Mesh {
            vertex_coords_vbo: Vbo::from_data(zgl, data),
            length: length,
            mode: MeshRenderMode::Triangles,
        }
    }

    pub fn draw(&self, zgl: &Zgl, shader: &Shader) {
        self.vertex_coords_vbo.bind(zgl);
        unsafe {
            let attr_id = shader.get_attr_location(zgl, "position"); // TODO: Move to shader init step
            let components_count = 3;
            let is_normalized = gl::FALSE;
            let stride = 0;
            zgl.gl.VertexAttribPointer(
                attr_id,
                components_count,
                gl::FLOAT,
                is_normalized,
                stride,
                ptr::null_mut(),
            );
            zgl.check();
            zgl.gl.EnableVertexAttribArray(attr_id); // TODO: Move to shader init step
            zgl.check();
            let starting_index = 0;
            zgl.gl.DrawArrays(self.mode.to_gl_type(), starting_index, self.length);
            zgl.check();
        }
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
