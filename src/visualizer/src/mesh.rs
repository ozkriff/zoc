// See LICENSE file for copyright and license details.

use gfx;
use gfx::traits::{FactoryExt};
use gfx_gl;
use types::{ZInt};
use fs;
use context::{Context};
use texture::{Texture, load_texture};
use pipeline::{Vertex};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct MeshId{pub id: ZInt}

// TODO: TODO: make fields private
pub struct Mesh {
    pub slice: gfx::Slice<gfx_gl::Resources>,
    pub vertex_buffer: gfx::handle::Buffer<gfx_gl::Resources, Vertex>,
    pub texture: Texture,
    is_wire: bool,
}

impl Mesh {
    pub fn new(context: &mut Context, vertices: &[Vertex], indices: &[u16], tex: Texture) -> Mesh {
        let (v, s) = context.factory.create_vertex_buffer_with_slice(vertices, indices);
        Mesh {
            slice: s,
            vertex_buffer: v,
            texture: tex,
            is_wire: false,
        }
    }

    pub fn new_wireframe(context: &mut Context, vertices: &[Vertex], indices: &[u16]) -> Mesh {
        let (v, s) = context.factory.create_vertex_buffer_with_slice(vertices, indices);
        let texture_data = fs::load("white.png").into_inner();
        let texture = load_texture(&mut context.factory, &texture_data);
        Mesh {
            slice: s,
            vertex_buffer: v,
            texture: texture,
            is_wire: true,
        }
    }

    pub fn is_wire(&self) -> bool {
        self.is_wire
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
