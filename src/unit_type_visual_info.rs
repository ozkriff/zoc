use std::collections::{HashMap};
use core::unit::{UnitTypeId};
use core::db::{Db};
use context::{Context};
use mesh::{MeshId};
use types::{Speed};
use mesh_manager::{MeshManager, load_object_mesh};

#[derive(Clone, Debug)]
pub struct UnitTypeVisualInfo {
    pub mesh_id: MeshId,
    pub move_speed: Speed,
}

#[derive(Clone, Debug)]
pub struct UnitTypeVisualInfoManager {
    map: HashMap<UnitTypeId, UnitTypeVisualInfo>,
}

impl UnitTypeVisualInfoManager {
    pub fn new() -> UnitTypeVisualInfoManager {
        UnitTypeVisualInfoManager {
            map: HashMap::new(),
        }
    }

    pub fn add_info(&mut self, unit_type_id: UnitTypeId, info: UnitTypeVisualInfo) {
        self.map.insert(unit_type_id, info);
    }

    pub fn get(&self, type_id: UnitTypeId) -> &UnitTypeVisualInfo {
        &self.map[&type_id]
    }
}

pub fn get_unit_type_visual_info(
    db: &Db,
    context: &mut Context,
    meshes: &mut MeshManager,
) -> UnitTypeVisualInfoManager {
    let mut manager = UnitTypeVisualInfoManager::new();
    for &(unit_name, model_name, move_speed) in &[
        ("soldier", "soldier", 2.0),
        ("smg", "submachine", 2.0),
        ("scout", "scout", 2.5),
        ("mortar", "mortar", 1.5),
        ("field_gun", "field_gun", 1.5),
        ("light_spg", "light_spg", 3.0),
        ("light_tank", "light_tank", 3.0),
        ("medium_tank", "medium_tank", 2.5),
        ("heavy_tank", "tank", 2.0),
        ("mammoth_tank", "mammoth", 1.5),
        ("truck", "truck", 3.0),
        ("jeep", "jeep", 3.5),
        ("helicopter", "helicopter", 3.0),
    ] {
        manager.add_info(db.unit_type_id(unit_name), UnitTypeVisualInfo {
            mesh_id: meshes.add(load_object_mesh(context, model_name)),
            move_speed: Speed{n: move_speed},
        });
    }
    manager
}
