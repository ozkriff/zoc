use mesh::{MeshId, Mesh};
use text;
use texture::{load_texture_raw};
use action::{Action, ActionContext};
use pipeline::{Vertex};

#[derive(Debug)]
pub struct CreateTextMesh {
    text: String,
    mesh_id: MeshId,
}

impl CreateTextMesh {
    pub fn new(text: String, mesh_id: MeshId) -> Self {
        Self {
            text: text,
            mesh_id: mesh_id,
        }
    }
}

impl Action for CreateTextMesh {
    fn begin(&mut self, context: &mut ActionContext) {
        let text_size = 80.0; // TODO: ???
        let (size, texture_data) = text::text_to_texture(
            context.context.font(), text_size, &self.text);
        let texture = load_texture_raw(
            context.context.factory_mut(), size, &texture_data);
        let scale_factor = 200.0; // TODO: take camera zoom into account
        let h_2 = (size.h as f32 / scale_factor) / 2.0;
        let w_2 = (size.w as f32 / scale_factor) / 2.0;
        let vertices = &[
            Vertex{pos: [-w_2, -h_2, 0.0], uv: [0.0, 1.0]},
            Vertex{pos: [-w_2, h_2, 0.0], uv: [0.0, 0.0]},
            Vertex{pos: [w_2, -h_2, 0.0], uv: [1.0, 1.0]},
            Vertex{pos: [w_2, h_2, 0.0], uv: [1.0, 0.0]},
        ];
        let indices = &[0,  1,  2,  1,  2,  3];
        let mesh = Mesh::new_nodepth(context.context, vertices, indices, texture);
        context.meshes.set(self.mesh_id, mesh);
    }
}
