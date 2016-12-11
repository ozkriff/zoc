use std::collections::{HashMap, HashSet, BTreeMap};
use std::cmp::{Ord, Ordering};
use cgmath::{Rad};
use core::{UnitId, SectorId, ObjectId};
use types::{WorldPos};
use mesh::{MeshId};

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NodeId{pub id: i32}

// TODO: Builder constructor
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub pos: WorldPos,
    pub rot: Rad<f32>,
    pub mesh_id: Option<MeshId>,
    pub color: [f32; 4],
    pub children: Vec<SceneNode>,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
pub struct Z(f32);

impl Eq for Z {}

impl Ord for Z {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    unit_id_to_node_id_map: HashMap<UnitId, NodeId>,
    sector_id_to_node_id_map: HashMap<SectorId, NodeId>,
    object_id_to_node_id_map: HashMap<ObjectId, HashSet<NodeId>>,
    nodes: HashMap<NodeId, SceneNode>,
    transparent_node_ids: BTreeMap<Z, HashSet<NodeId>>,
    next_id: NodeId,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            unit_id_to_node_id_map: HashMap::new(),
            sector_id_to_node_id_map: HashMap::new(),
            object_id_to_node_id_map: HashMap::new(),
            nodes: HashMap::new(),
            transparent_node_ids: BTreeMap::new(),
            next_id: NodeId{id: 0},
        }
    }

    pub fn unit_id_to_node_id_opt(&self, unit_id: UnitId) -> Option<NodeId> {
        self.unit_id_to_node_id_map.get(&unit_id).map(|v| v.clone())
    }

    pub fn unit_id_to_node_id(&self, unit_id: UnitId) -> NodeId {
        self.unit_id_to_node_id_map[&unit_id]
    }

    pub fn sector_id_to_node_id(&self, sector_id: SectorId) -> NodeId {
        self.sector_id_to_node_id_map[&sector_id]
    }

    pub fn object_id_to_node_id(&self, object_id: ObjectId) -> &HashSet<NodeId> {
        &self.object_id_to_node_id_map[&object_id]
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        self.nodes.remove(&node_id).unwrap();
        for layer in self.transparent_node_ids.values_mut() {
            layer.remove(&node_id);
        }
    }

    pub fn add_node(&mut self, node: SceneNode) -> NodeId {
        let node_id = self.next_id;
        self.next_id.id += 1;
        assert!(!self.nodes.contains_key(&node_id));
        if node.color[3] < 1.0 {
            let z = Z(node.pos.v.z);
            self.transparent_node_ids.entry(z).or_insert_with(HashSet::new);
            let layer = self.transparent_node_ids.get_mut(&z).unwrap();
            layer.insert(node_id);
        }
        self.nodes.insert(node_id, node);
        node_id
    }

    pub fn remove_unit(&mut self, unit_id: UnitId) {
        assert!(self.unit_id_to_node_id_map.contains_key(&unit_id));
        let node_id = self.unit_id_to_node_id(unit_id);
        self.remove_node(node_id);
        self.unit_id_to_node_id_map.remove(&unit_id).unwrap();
    }

    pub fn remove_object(&mut self, object_id: ObjectId) {
        assert!(self.object_id_to_node_id_map.contains_key(&object_id));
        let node_ids = self.object_id_to_node_id(object_id).clone();
        for node_id in node_ids {
            self.remove_node(node_id);
        }
        self.object_id_to_node_id_map.remove(&object_id).unwrap();
    }

    pub fn add_unit(&mut self, unit_id: UnitId, node: SceneNode) -> NodeId {
        let node_id = self.add_node(node);
        assert!(!self.unit_id_to_node_id_map.contains_key(&unit_id));
        self.unit_id_to_node_id_map.insert(unit_id, node_id);
        node_id
    }

    pub fn add_sector(&mut self, sector_id: SectorId, node: SceneNode) -> NodeId {
        let node_id = self.add_node(node);
        assert!(!self.sector_id_to_node_id_map.contains_key(&sector_id));
        self.sector_id_to_node_id_map.insert(sector_id, node_id);
        node_id
    }

    pub fn add_object(&mut self, object_id: ObjectId, node: SceneNode) -> NodeId {
        let node_id = self.add_node(node);
        self.object_id_to_node_id_map.entry(object_id).or_insert_with(HashSet::new);
        let node_ids = self.object_id_to_node_id_map.get_mut(&object_id).unwrap();
        node_ids.insert(node_id);
        node_id
    }

    pub fn nodes(&self) -> &HashMap<NodeId, SceneNode> {
        &self.nodes
    }

    pub fn transparent_node_ids(&self) -> &BTreeMap<Z, HashSet<NodeId>> {
        &self.transparent_node_ids
    }

    pub fn node(&self, node_id: NodeId) -> &SceneNode {
        &self.nodes[&node_id]
    }

    pub fn node_mut(&mut self, node_id: NodeId) -> &mut SceneNode {
        self.nodes.get_mut(&node_id).expect("Bad node id")
    }
}
