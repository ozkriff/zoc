// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Deg};
use common::types::{ZInt, ZFloat, WorldPos};
use zgl::mesh::{MeshId};

// TODO: why scene knows about other systems?
pub const MAX_UNIT_NODE_ID: NodeId = NodeId{id: 1000};
pub const MIN_MARKER_NODE_ID: NodeId = NodeId{id: MAX_UNIT_NODE_ID.id + 1};
pub const MAX_MARKER_NODE_ID: NodeId = NodeId{id: MAX_UNIT_NODE_ID.id * 2};
pub const SHELL_NODE_ID: NodeId = NodeId{id: MAX_MARKER_NODE_ID.id + 1};
pub const SELECTION_NODE_ID: NodeId = NodeId{id: SHELL_NODE_ID.id + 1};
pub const MIN_MAP_OBJECT_NODE_ID: NodeId = NodeId{id: SELECTION_NODE_ID.id + 1};
// pub const MAX_MAP_OBJECT_NODE_ID: NodeId = NodeId{id: MIN_MAP_OBJECT_NODE_ID.id + 100}; // TODO: 100?

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct NodeId{pub id: ZInt}

pub struct SceneNode {
    pub pos: WorldPos,
    pub rot: Deg<ZFloat>,
    pub mesh_id: Option<MeshId>,
    pub children: Vec<SceneNode>,
}

pub struct Scene {
    pub nodes: HashMap<NodeId, SceneNode>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            nodes: HashMap::new(),
        }
    }

    // TODO: node -> node_mut
    pub fn node(&mut self, node_id: &NodeId) -> &mut SceneNode {
        self.nodes.get_mut(node_id)
            .expect("Bad node id")
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
