use mesh::{MeshId};
use action::{Action, ActionContext};

#[derive(Debug)]
pub struct RemoveMesh {
    mesh_id: MeshId,
}

impl RemoveMesh {
    pub fn new(mesh_id: MeshId) -> Self {
        Self {
            mesh_id: mesh_id,
        }
    }
}

impl Action for RemoveMesh {
    fn begin(&mut self, context: &mut ActionContext) {
        context.meshes.remove(self.mesh_id);
    }
}
