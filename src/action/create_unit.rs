use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, Rad};
use core::unit::{Unit};
use types::{WorldPos};
use mesh::{MeshId};
use geom;
use gen;
use scene::{self, SceneNode, NodeId};
use action::{Action, ActionContext, WRECKS_COLOR};

// TODO: Action::CreateSceneNode?
#[derive(Debug)]
pub struct CreateUnit {
    unit: Unit,
    pos: WorldPos,
    node_id: NodeId,
}

impl CreateUnit {
    pub fn new(
        unit: Unit,
        pos: WorldPos, // TODO: этот аргумент не пердавать, заменить на action::SetPos
        node_id: NodeId,
    ) -> Box<Action> {
        Box::new(Self {
            unit: unit,
            pos: pos,
            node_id: node_id,
        })
    }
}

impl Action for CreateUnit {
    fn begin(&mut self, context: &mut ActionContext) {
        let mesh_id = context.visual_info.get(self.unit.type_id).mesh_id;
        let size = context.visual_info.get(self.unit.type_id).size;
        context.scene.add_unit(self.node_id, self.unit.id, SceneNode {
            pos: self.pos,
            rot: Rad(thread_rng().gen_range(0.0, PI * 2.0)),
            .. Default::default()
        });
        set_children(context, self.node_id, &self.unit, mesh_id);
        if self.unit.is_alive {
            let id = context.scene.allocate_node_id();
            context.scene.set_child_node(self.node_id, id, SceneNode {
                pos: WorldPos{v: geom::vec3_z(geom::HEX_EX_RADIUS / 2.0)},
                mesh_id: Some(context.mesh_ids.marker_mesh_id),
                color: gen::get_player_color(self.unit.player_id),
                .. Default::default()
            });
            let shadow_node_id = context.scene.allocate_node_id();
            context.scene.set_child_node(self.node_id, shadow_node_id, SceneNode {
                pos: WorldPos{v: geom::vec3_z(0.01)},
                scale: 0.5 * size, // TODO: magic?
                mesh_id: Some(context.mesh_ids.shadow_mesh_id),
                color: [1.0, 0.0, 0.0, 0.8],
                node_type: scene::SceneNodeType::Transparent,
                .. Default::default()
            });
        }
    }
}

// TODO: rename
// TODO: BLOB SHADOWS
fn set_children(
    context: &mut ActionContext,
    parent_id: NodeId,
    unit: &Unit,
    mesh_id: MeshId,
) {
    let color = if unit.is_alive {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        WRECKS_COLOR
    };
    let base_node = SceneNode {
        mesh_id: Some(mesh_id),
        color: color,
        .. Default::default()
    };
    if unit.count == 1 {
        let id = context.scene.allocate_node_id();
        let node = SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}},
            ..base_node.clone()
        };
        context.scene.set_child_node(parent_id, id, node);
    } else {
        for i in 0 .. unit.count {
            let id = context.scene.allocate_node_id();
            let pos = geom::index_to_circle_vertex(unit.count, i).v * 0.15;
            let world_pos = WorldPos{v: pos};
            let node = SceneNode {
                pos: world_pos,
                ..base_node.clone()
            };
            context.scene.set_child_node(parent_id, id, node);
        }
    }
}
