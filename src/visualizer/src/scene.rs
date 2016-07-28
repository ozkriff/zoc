// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use cgmath::{Rad};
use core::{UnitId};
use types::{ZInt, ZFloat, WorldPos};
use mesh::{MeshId};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct NodeId{pub id: ZInt}

pub struct SceneNode {
    pub pos: WorldPos,
    pub rot: Rad<ZFloat>,
    pub mesh_id: Option<MeshId>,
    pub children: Vec<SceneNode>,
}

pub struct Scene {
    unit_id_to_node_id_map: HashMap<UnitId, NodeId>,
    nodes: HashMap<NodeId, SceneNode>,
    next_id: NodeId,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            unit_id_to_node_id_map: HashMap::new(),
            nodes: HashMap::new(),
            next_id: NodeId{id: 0},
        }
    }

    pub fn unit_id_to_node_id(&self, unit_id: &UnitId) -> NodeId {
        self.unit_id_to_node_id_map[unit_id].clone()
    }

    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.nodes.remove(node_id).unwrap();
    }

    pub fn add_node(&mut self, node: SceneNode) -> NodeId {
        let node_id = self.next_id.clone();
        self.next_id.id += 1;
        assert!(!self.nodes.contains_key(&node_id));
        self.nodes.insert(node_id.clone(), node);
        node_id
    }

    pub fn remove_unit(&mut self, unit_id: &UnitId) {
        assert!(self.unit_id_to_node_id_map.contains_key(unit_id));
        let node_id = self.unit_id_to_node_id(unit_id);
        self.remove_node(&node_id);
        self.unit_id_to_node_id_map.remove(unit_id).unwrap();
    }

    pub fn add_unit(&mut self, unit_id: &UnitId, node: SceneNode) -> NodeId {
        let node_id = self.add_node(node);
        assert!(!self.unit_id_to_node_id_map.contains_key(unit_id));
        self.unit_id_to_node_id_map.insert(unit_id.clone(), node_id.clone());
        node_id
    }

    pub fn nodes(&self) -> &HashMap<NodeId, SceneNode> {
        &self.nodes
    }

    pub fn node(&self, node_id: &NodeId) -> &SceneNode {
        &self.nodes[node_id]
    }

    pub fn node_mut(&mut self, node_id: &NodeId) -> &mut SceneNode {
        self.nodes.get_mut(node_id)
            .expect("Bad node id")
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
