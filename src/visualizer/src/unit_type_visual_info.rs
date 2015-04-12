// See LICENSE file for copyright and license details.

use common::types::{ZInt, ZFloat};
use core::unit::{UnitTypeId};
use zgl::mesh::{MeshId};

pub struct UnitTypeVisualInfo {
    pub mesh_id: MeshId,
    pub move_speed: ZFloat, // TODO: MFloat -> Speed
}

pub struct UnitTypeVisualInfoManager {
    list: Vec<UnitTypeVisualInfo>,
}

impl UnitTypeVisualInfoManager {
    pub fn new(unit_types_count: ZInt) -> UnitTypeVisualInfoManager {
        UnitTypeVisualInfoManager {
            list: Vec::with_capacity(unit_types_count as usize),
        }
    }

    pub fn add_info(&mut self, unit_type_id: &UnitTypeId, info: UnitTypeVisualInfo) {
        let index = unit_type_id.id as usize;
        self.list.insert(index, info);
    }

    pub fn get<'a>(&'a self, type_id: &UnitTypeId) -> &'a UnitTypeVisualInfo {
        &self.list[type_id.id as usize]
    }
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
