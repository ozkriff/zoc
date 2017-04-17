use std::f32::consts::{PI};
use rand::{thread_rng, Rng};
use cgmath::{Vector3, Rad};
use core::unit::{Unit};
use types::{WorldPos};
use mesh::{MeshId};
use geom;
use gen;
use scene::{SceneNode, NodeId};
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
        pos: WorldPos, // TODO: этот аргумент тоже не пердавать, заменить на SetPos
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
        let rot = Rad(thread_rng().gen_range(0.0, PI * 2.0));
        let mut children = get_unit_scene_nodes(&self.unit, mesh_id);
        if self.unit.is_alive {
            children.push(SceneNode {
                pos: WorldPos{v: geom::vec3_z(geom::HEX_EX_RADIUS / 2.0)},
                mesh_id: Some(context.mesh_ids.marker_mesh_id),
                color: gen::get_player_color(self.unit.player_id),
                .. Default::default()
            });
        }
        context.scene.add_unit(self.node_id, self.unit.id, SceneNode {
            pos: self.pos,
            rot: rot,
            children: children,
            .. Default::default()
        });
    }
}

fn get_unit_scene_nodes(unit: &Unit, mesh_id: MeshId) -> Vec<SceneNode> {
    let color = if unit.is_alive {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        WRECKS_COLOR
    };
    let mut vec = Vec::new();
    if unit.count == 1 {
        vec![SceneNode {
            pos: WorldPos{v: Vector3{x: 0.0, y: 0.0, z: 0.0}},
            mesh_id: Some(mesh_id),
            color: color,
            .. Default::default()
        }]
    } else {
        for i in 0 .. unit.count {
            let pos = geom::index_to_circle_vertex(unit.count, i).v * 0.15;
            vec.push(SceneNode {
                pos: WorldPos{v: pos},
                mesh_id: Some(mesh_id),
                color: color,
                .. Default::default()
            });
        }
        vec
    }
}
