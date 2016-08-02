use std::collections::{HashMap};
use core::unit::{UnitTypeId};
use mesh::{MeshId};

pub struct UnitTypeVisualInfo {
    pub mesh_id: MeshId,
    pub move_speed: f32, // TODO: f32 -> Speed
}

pub struct UnitTypeVisualInfoManager {
    map: HashMap<UnitTypeId, UnitTypeVisualInfo>,
}

impl UnitTypeVisualInfoManager {
    pub fn new() -> UnitTypeVisualInfoManager {
        UnitTypeVisualInfoManager {
            map: HashMap::new(),
        }
    }

    pub fn add_info(&mut self, unit_type_id: &UnitTypeId, info: UnitTypeVisualInfo) {
        self.map.insert(unit_type_id.clone(), info);
    }

    pub fn get(&self, type_id: &UnitTypeId) -> &UnitTypeVisualInfo {
        &self.map[type_id]
    }
}
