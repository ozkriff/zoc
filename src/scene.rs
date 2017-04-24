use std::default::{Default};
use std::collections::{HashMap, HashSet};
use std::cmp::{Ordering};
use cgmath::{Rad, Vector3, Array, InnerSpace};
use core::object::{ObjectId};
use core::unit::{UnitId};
use core::sector::{SectorId};
use types::{WorldPos};
use mesh::{MeshId};

// TODO: rename and move to some better place
// TODO: functuin -> trait method, implemented for Vec
fn vec_rem_opt<T: PartialEq>(vec: &mut Vec<T>, value: T) -> Option<T> {
    // TODO: separate two steps: 1-find, 2-remove
    vec.iter().position(|n| *n == value).map(|pos| vec.swap_remove(pos))
}

fn vec_rem<T: PartialEq>(vec: &mut Vec<T>, value: T) {
    vec_rem_opt(vec, value).unwrap();
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct NodeId{pub id: i32}

// TODO: Rename to NodeType
#[derive(Clone, Copy, Debug)]
pub enum SceneNodeType {
    Normal,
    Transparent,
}

// TODO: Rename to Node
// TODO: Builder constructor
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub pos: WorldPos,
    pub rot: Rad<f32>, // TODO: Store Matrix3 here?
    pub mesh_id: Option<MeshId>,
    pub color: [f32; 4],
    pub children: Vec<NodeId>,

    // TODO: прямо при создании спрашивать это дело
    // а не только на основе color.
    //
    // Только проверять что is_transparent == false для всех у кого color.3 == 1.0
    //
    pub node_type: SceneNodeType,
}

impl Default for SceneNode {
    fn default() -> Self {
        SceneNode {
            rot: Rad(0.0),
            mesh_id: None,
            color: [1.0, 1.0, 1.0, 1.0],
            children: vec![],

            // TODO: there's not much sence in making this fields optional..
            pos: WorldPos{v: Vector3::from_value(0.0)},
            node_type: SceneNodeType::Normal,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    unit_id_to_node_id_map: HashMap<UnitId, NodeId>,
    sector_id_to_node_id_map: HashMap<SectorId, NodeId>,
    object_id_to_node_id_map: HashMap<ObjectId, HashSet<NodeId>>,
    nodes: HashMap<NodeId, SceneNode>,
    normal_node_ids: Vec<NodeId>,
    transparent_node_ids: Vec<NodeId>,
    static_plane_node_ids: Vec<NodeId>,
    next_id: NodeId,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            unit_id_to_node_id_map: HashMap::new(),
            sector_id_to_node_id_map: HashMap::new(),
            object_id_to_node_id_map: HashMap::new(),
            nodes: HashMap::new(),
            normal_node_ids: Vec::new(),
            transparent_node_ids: Vec::new(),
            static_plane_node_ids: Vec::new(),
            next_id: NodeId{id: 0},
        }
    }

    pub fn unit_id_to_node_id_opt(&self, unit_id: UnitId) -> Option<NodeId> {
        self.unit_id_to_node_id_map.get(&unit_id).cloned()
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

    // TODO: Move all Actions to this module and remove all other ways
    // to directly mutate the scene (methods taking `&mut self`)!
    pub fn remove_node(&mut self, node_id: NodeId) {
        // TODO: remove children?
        let node_type = self.node(node_id).node_type;
        self.nodes.remove(&node_id).unwrap();
        match node_type {
            SceneNodeType::Normal => {
                vec_rem(&mut self.normal_node_ids, node_id);
            },
            SceneNodeType::Transparent => {
                vec_rem(&mut self.transparent_node_ids, node_id);
            },
        }
    }

    pub fn allocate_node_id(&mut self) -> NodeId {
        let node_id = self.next_id;
        self.next_id.id += 1;
        node_id
    }

    pub fn sort_transparent_nodes(&mut self, pos: WorldPos) {
        let nodes = &self.nodes;
        self.transparent_node_ids.sort_by(|a, b| {
            let dist_a = (pos.v - nodes[a].pos.v).magnitude();
            let dist_b = (pos.v - nodes[b].pos.v).magnitude();
            dist_a.partial_cmp(&dist_b).unwrap_or(Ordering::Equal)
        });
    }

    fn set_node_internal(&mut self, node_id: NodeId, node: SceneNode) {
        assert!(!self.nodes.contains_key(&node_id));
        // TODO: node.color[3] < 1.0;
        self.nodes.insert(node_id, node);
    }

    pub fn attach_node(&mut self, parent_id: NodeId, child_id: NodeId) {
        self.node_mut(parent_id).children.push(child_id);
        let _ = vec_rem_opt(&mut self.normal_node_ids, child_id);
    }

    pub fn detach_node(&mut self, parent_id: NodeId, node_id: NodeId) {
        vec_rem(&mut self.node_mut(parent_id).children, node_id);
        self.normal_node_ids.push(node_id);
    }

    pub fn set_child_node(
        &mut self,
        parent_id: NodeId,
        child_id: NodeId,
        node: SceneNode,
    ) {
        self.set_node_internal(child_id, node);
        self.attach_node(parent_id, child_id);
    }

    pub fn set_node(&mut self, node_id: NodeId, node: SceneNode) {
        let node_type = node.node_type;
        self.set_node_internal(node_id, node);
        match node_type {
            SceneNodeType::Normal => {
                self.normal_node_ids.push(node_id);
            },
            SceneNodeType::Transparent => {
                self.transparent_node_ids.push(node_id);
            },
        }
    }

    // TODO: deprecate?
    pub fn add_node(&mut self, node: SceneNode) -> NodeId {
        let node_id = self.allocate_node_id();
        self.set_node(node_id, node);
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

    pub fn add_unit(
        &mut self,
        node_id: NodeId,
        unit_id: UnitId,
        node: SceneNode,
    ) {
        self.set_node(node_id, node);
        assert!(!self.unit_id_to_node_id_map.contains_key(&unit_id));
        self.unit_id_to_node_id_map.insert(unit_id, node_id);
    }

    pub fn add_sector(&mut self, sector_id: SectorId, node: SceneNode) -> NodeId {
        let node_id = self.add_node(node);
        assert!(!self.sector_id_to_node_id_map.contains_key(&sector_id));
        self.sector_id_to_node_id_map.insert(sector_id, node_id);
        node_id
    }

    pub fn add_object(
        &mut self,
        node_id: NodeId,
        object_id: ObjectId,
        node: SceneNode
    ) {
        self.set_node(node_id, node);
        self.object_id_to_node_id_map.entry(object_id).or_insert_with(HashSet::new);
        let node_ids = self.object_id_to_node_id_map.get_mut(&object_id).unwrap();
        node_ids.insert(node_id);
    }

    pub fn nodes(&self) -> &HashMap<NodeId, SceneNode> {
        &self.nodes
    }

    pub fn normal_node_ids(&self) -> &[NodeId] {
        &self.normal_node_ids
    }

    pub fn static_plane_node_ids(&self) -> &[NodeId] {
        &self.static_plane_node_ids
    }

    pub fn transparent_node_ids(&self) -> &[NodeId] {
        &self.transparent_node_ids
    }

    pub fn node(&self, node_id: NodeId) -> &SceneNode {
        &self.nodes[&node_id]
    }

    pub fn node_mut(&mut self, node_id: NodeId) -> &mut SceneNode {
        self.nodes.get_mut(&node_id).expect("Bad node id")
    }
}
