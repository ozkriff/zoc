use std::collections::{HashMap};
use context::{Context};
use core::{self};
use core::game_state::{State};
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
    pub sector_mesh_ids: HashMap<core::SectorId, MeshId>,
}

impl MeshIdManager {
    pub fn new(
        context: &mut Context,
        meshes: &mut MeshManager,
        state: &State,
    ) -> MeshIdManager {
        let smoke_tex = load_texture(context, &fs::load("smoke.png").into_inner());
        let floor_tex = load_texture(context, &fs::load("hex.png").into_inner());
        let reinforcement_sector_tex = load_texture(
            context, &fs::load("reinforcement_sector.png").into_inner());
        let chess_grid_tex = load_texture(context, &fs::load("chess_grid.png").into_inner());
        let map_mesh_id = meshes.add(gen::generate_map_mesh(
            context, state, floor_tex.clone()));
        let water_mesh_id = meshes.add(gen::generate_water_mesh(
            context, state, floor_tex.clone()));
        let mut sector_mesh_ids = HashMap::new();
        for (&id, sector) in state.sectors() {
            let mesh_id = meshes.add(gen::generate_sector_mesh(
                context, sector, chess_grid_tex.clone()));
            sector_mesh_ids.insert(id, mesh_id);
        }
        let selection_marker_mesh_id = meshes.add(get_selection_mesh(context));
        let smoke_mesh_id = meshes.add(gen::get_one_tile_mesh(context, smoke_tex));
        let fow_tile_mesh_id = meshes.add(gen::get_one_tile_mesh(context, floor_tex));
        let reinforcement_sector_tile_mesh_id = meshes.add(
            gen::get_one_tile_mesh(context, reinforcement_sector_tex));
        let big_building_mesh_id = meshes.add(
            load_object_mesh(context, "big_building"));
        let building_mesh_id = meshes.add(
            load_object_mesh(context, "building"));
        let big_building_mesh_w_id = meshes.add(
            load_object_mesh(context, "big_building_wire"));
        let building_mesh_w_id = meshes.add(
            load_object_mesh(context, "building_wire"));
        let trees_mesh_id = meshes.add(load_object_mesh(context, "trees"));
        let shell_mesh_id = meshes.add(gen::get_shell_mesh(context));
        let road_mesh_id = meshes.add(gen::get_road_mesh(context));
        let marker_mesh_id = meshes.add(gen::get_marker(context, "white.png"));
        let walkable_mesh_id = meshes.add(gen::empty_mesh(context));
        let targets_mesh_id = meshes.add(gen::empty_mesh(context));
        MeshIdManager {
            big_building_mesh_id: big_building_mesh_id,
            building_mesh_id: building_mesh_id,
            big_building_mesh_w_id: big_building_mesh_w_id,
            building_mesh_w_id: building_mesh_w_id,
            trees_mesh_id: trees_mesh_id,
            road_mesh_id: road_mesh_id,
            shell_mesh_id: shell_mesh_id,
            marker_mesh_id: marker_mesh_id,
            walkable_mesh_id: walkable_mesh_id,
            targets_mesh_id: targets_mesh_id,
            map_mesh_id: map_mesh_id,
            water_mesh_id: water_mesh_id,
            selection_marker_mesh_id: selection_marker_mesh_id,
            smoke_mesh_id: smoke_mesh_id,
            fow_tile_mesh_id: fow_tile_mesh_id,
            reinforcement_sector_tile_mesh_id: reinforcement_sector_tile_mesh_id,
            sector_mesh_ids: sector_mesh_ids,
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
    meshes: Vec<Mesh>,
}

impl MeshManager {
    pub fn new() -> MeshManager {
        MeshManager {
            meshes: Vec::new(),
        }
    }

    pub fn add(&mut self, mesh: Mesh) -> MeshId {
        self.meshes.push(mesh);
        MeshId{id: (self.meshes.len() as i32) - 1}
    }

    pub fn set(&mut self, id: MeshId, mesh: Mesh) {
        let index = id.id as usize;
        self.meshes[index] = mesh;
    }

    pub fn get(&self, id: MeshId) -> &Mesh {
        let index = id.id as usize;
        &self.meshes[index]
    }
}
