// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use core::unit::{UnitTypeId};
use types::{ZFloat};
use mesh::{MeshId};

pub struct UnitTypeVisualInfo {
    pub mesh_id: MeshId,
    pub move_speed: ZFloat, // TODO: MFloat -> Speed
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

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
