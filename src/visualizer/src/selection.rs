// See LICENSE file for copyright and license details.

use cgmath::{Vector2, rad};
use core::{UnitId};
use core::partial_state::{PartialState};
use core::game_state::{GameState};
use zgl::misc::{add_quad_to_vec};
use zgl::mesh::{Mesh, MeshId};
use zgl::texture::Texture;
use zgl::types::{TextureCoord, WorldPos};
use zgl::{Zgl};
use geom;
use scene::{Scene, SceneNode, NodeId};

pub struct SelectionManager {
    unit_id: Option<UnitId>,
    mesh_id: MeshId,
    selection_marker_node_id: Option<NodeId>,
}

impl SelectionManager {
    pub fn new(mesh_id: MeshId) -> SelectionManager {
        SelectionManager {
            unit_id: None,
            mesh_id: mesh_id,
            selection_marker_node_id: None,
        }
    }

    fn get_pos(&self, state: &PartialState) -> WorldPos {
        let unit_id = self.unit_id.clone()
            .expect("Can`t get pos if no unit is selected");
        let map_pos = state.units()[&unit_id].pos.clone();
        WorldPos{v: geom::lift(geom::exact_pos_to_world_pos(&map_pos).v)}
    }

    pub fn create_selection_marker(
        &mut self,
        state: &PartialState,
        scene: &mut Scene,
        unit_id: &UnitId,
    ) {
        self.unit_id = Some(unit_id.clone());
        if let Some(ref node_id) = self.selection_marker_node_id {
            if scene.nodes().get(node_id).is_some() {
                scene.remove_node(node_id);
            }
        }
        let node = SceneNode {
            pos: self.get_pos(state),
            rot: rad(0.0),
            mesh_id: Some(self.mesh_id.clone()),
            children: Vec::new(),
        };
        self.selection_marker_node_id = Some(scene.add_node(node));
    }

    pub fn deselect(&mut self, scene: &mut Scene) {
        self.unit_id = None;
        if let Some(ref node_id) = self.selection_marker_node_id {
            scene.remove_node(node_id);
        }
        self.selection_marker_node_id = None;
    }
}

pub fn get_selection_mesh(zgl: &Zgl) -> Mesh {
    let tex = Texture::new(zgl, "shell.png");
    let mut vertex_data = Vec::new();
    let mut tex_data = Vec::new();
    let scale_1 = 0.6;
    let scale_2 = scale_1 + 0.05;
    for num in 0 .. 6 {
        let vertex_1_1 = geom::index_to_hex_vertex_s(scale_1, num);
        let vertex_1_2 = geom::index_to_hex_vertex_s(scale_2, num);
        let vertex_2_1 = geom::index_to_hex_vertex_s(scale_1, num + 1);
        let vertex_2_2 = geom::index_to_hex_vertex_s(scale_2, num + 1);
        add_quad_to_vec(
            &mut vertex_data,
            vertex_2_1,
            vertex_2_2,
            vertex_1_2,
            vertex_1_1,
        );
        add_quad_to_vec(
            &mut tex_data,
            TextureCoord{v: Vector2{x: 0.0, y: 0.0}},
            TextureCoord{v: Vector2{x: 0.0, y: 1.0}},
            TextureCoord{v: Vector2{x: 1.0, y: 1.0}},
            TextureCoord{v: Vector2{x: 1.0, y: 0.0}},
        );
    }
    let mut mesh = Mesh::new(zgl, &vertex_data);
    mesh.add_texture(zgl, tex, &tex_data);
    mesh
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
