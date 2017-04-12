use gfx;
use gfx::traits::{FactoryExt};
use gfx_gl;
use fs;
use context::{Context};
use texture::{Texture, load_texture};
use pipeline::{Vertex};

// TODO: Move to mesh_manager.rs
#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MeshId{pub id: i32}

#[derive(Clone, Copy, Debug)]
pub enum MeshType {
    Normal,
    Wire,
    NoDepth,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    slice: gfx::Slice<gfx_gl::Resources>,
    vertex_buffer: gfx::handle::Buffer<gfx_gl::Resources, Vertex>,
    texture: Texture,
    render_type: MeshType, // TODO: ?!?!?!?!?
}

impl Mesh {
    // TODO: typedef u16 -> VertexIndex
    pub fn new(context: &mut Context, vertices: &[Vertex], indices: &[u16], tex: Texture) -> Mesh {
        let (v, s) = context.factory_mut().create_vertex_buffer_with_slice(vertices, indices);
        Mesh {
            slice: s,
            vertex_buffer: v,
            texture: tex,
            render_type: MeshType::Normal,
        }
    }

    pub fn new_nodepth(context: &mut Context, vertices: &[Vertex], indices: &[u16], tex: Texture) -> Mesh {
        Mesh {
            render_type: MeshType::NoDepth,
            .. Mesh::new(context, vertices, indices, tex)
        }
    }

    pub fn new_wireframe(context: &mut Context, vertices: &[Vertex], indices: &[u16]) -> Mesh {
        let (v, s) = context.factory_mut().create_vertex_buffer_with_slice(vertices, indices);
        let texture_data = fs::load("white.png").into_inner();
        let texture = load_texture(context, &texture_data);
        Mesh {
            slice: s,
            vertex_buffer: v,
            texture: texture,
            render_type: MeshType::Wire,
        }
    }

    pub fn slice(&self) -> &gfx::Slice<gfx_gl::Resources> {
        &self.slice
    }

    pub fn vertex_buffer(&self) -> &gfx::handle::Buffer<gfx_gl::Resources, Vertex> {
        &self.vertex_buffer
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn render_type(&self) -> MeshType {
        self.render_type
    }
}
