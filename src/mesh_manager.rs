use std::collections::{HashMap};
use context::{Context};
use core::game_state::{State};
use core::sector::{SectorId};
use texture::{load_texture};
use mesh::{Mesh, MeshId};
use selection::{get_selection_mesh};
use fs;
use obj;
use gen;

#[derive(Clone, Debug)]
pub struct MeshIdManager {
    pub big_building_mesh_id: MeshId,
    pub building_mesh_id: MeshId,
    pub big_building_mesh_w_id: MeshId,
    pub building_mesh_w_id: MeshId,
    pub road_mesh_id: MeshId,
    pub trees_mesh_id: MeshId,
    pub shell_mesh_id: MeshId,
    pub marker_mesh_id: MeshId,
    pub walkable_mesh_id: MeshId,
    pub targets_mesh_id: MeshId,
    pub map_mesh_id: MeshId,
    pub water_mesh_id: MeshId,
    pub selection_marker_mesh_id: MeshId,
    pub smoke_mesh_id: MeshId,
    pub fow_tile_mesh_id: MeshId,
    pub reinforcement_sector_tile_mesh_id: MeshId,
    pub sector_mesh_ids: HashMap<SectorId, MeshId>,
}

impl MeshIdManager {
    pub fn new(
        context: &mut Context,
        meshes: &mut MeshManager,
        state: &State,
    ) -> MeshIdManager {
        let smoke_tex = load_texture(
            context, &fs::load("smoke.png").into_inner());
        let floor_tex = load_texture(
            context, &fs::load("hex.png").into_inner());
        let reinforcement_sector_tex = load_texture(
            context, &fs::load("reinforcement_sector.png").into_inner());
        let chess_grid_tex = load_texture(
            context, &fs::load("chess_grid.png").into_inner());
        MeshIdManager {
            big_building_mesh_id: meshes.add(
                load_object_mesh(context, "big_building")),
            building_mesh_id: meshes.add(
                load_object_mesh(context, "building")),
            big_building_mesh_w_id: meshes.add(
                load_object_mesh(context, "big_building_wire")),
            building_mesh_w_id: meshes.add(
                load_object_mesh(context, "building_wire")),
            trees_mesh_id: meshes.add(load_object_mesh(context, "trees")),
            road_mesh_id: meshes.add(gen::get_road_mesh(context)),
            shell_mesh_id: meshes.add(gen::get_shell_mesh(context)),
            marker_mesh_id: meshes.add(gen::get_marker(context, "white.png")),
            walkable_mesh_id: meshes.add(gen::empty_mesh(context)),
            targets_mesh_id: meshes.add(gen::empty_mesh(context)),
            map_mesh_id: meshes.add(gen::generate_map_mesh(
                context, state, floor_tex.clone())),
            water_mesh_id: meshes.add(gen::generate_water_mesh(
                context, state, floor_tex.clone())),
            selection_marker_mesh_id: meshes.add(get_selection_mesh(context)),
            smoke_mesh_id: meshes.add(
                gen::get_one_tile_mesh_transparent(context, smoke_tex)),
            fow_tile_mesh_id: meshes.add(
                gen::get_one_tile_mesh_transparent(context, floor_tex)),
            reinforcement_sector_tile_mesh_id: meshes.add(
                gen::get_one_tile_mesh_transparent(context, reinforcement_sector_tex)),
            sector_mesh_ids: {
                let mut sector_mesh_ids = HashMap::new();
                for (&id, sector) in state.sectors() {
                    let mesh_id = meshes.add(gen::generate_sector_mesh(
                        context, sector, chess_grid_tex.clone()));
                    sector_mesh_ids.insert(id, mesh_id);
                }
                sector_mesh_ids
            },
        }
    }
}

pub fn load_object_mesh(context: &mut Context, name: &str) -> Mesh {
    let model = obj::Model::new(&format!("{}.obj", name));
    let (vertices, indices) = obj::build(&model);
    if model.is_wire() {
        Mesh::new_wireframe(context, &vertices, &indices)
    } else {
        let texture_data = fs::load(format!("{}.png", name)).into_inner();
        let texture = load_texture(context, &texture_data);
        Mesh::new(context, &vertices, &indices, texture)
    }
}

#[derive(Clone, Debug)]
pub struct MeshManager {
    meshes: HashMap<MeshId, Mesh>,
    next_id: MeshId,
}

impl MeshManager {
    pub fn new() -> MeshManager {
        MeshManager {
            meshes: HashMap::new(),
            next_id: MeshId{id: 0},
        }
    }

    pub fn allocate_id(&mut self) -> MeshId {
        let id = self.next_id;
        self.next_id.id += 1;
        id
    }

    pub fn add(&mut self, mesh: Mesh) -> MeshId {
        let id = self.allocate_id();
        self.set(id, mesh);
        id
    }

    pub fn set(&mut self, id: MeshId, mesh: Mesh) {
        self.meshes.insert(id, mesh);
    }

    pub fn remove(&mut self, id: MeshId) {
        self.meshes.remove(&id).unwrap();
    }

    pub fn get(&self, id: MeshId) -> &Mesh {
        &self.meshes[&id]
    }
}
