// See LICENSE file for copyright and license details.

use std::collections::{HashMap};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use ::{CoreEvent, UnitId, ObjectId, Object, MapPos};

pub trait GameState {
    fn map(&self) -> &Map<Terrain>;
    fn units(&self) -> &HashMap<UnitId, Unit>;
    fn objects(&self) -> &HashMap<ObjectId, Object>;

    fn unit(&self, id: &UnitId) -> &Unit {
        &self.units()[id]
    }

    fn units_at(&self, pos: &MapPos) -> Vec<&Unit> {
        let mut units = Vec::new();
        for (_, unit) in self.units() {
            if unit.pos.map_pos == *pos {
                units.push(unit);
            }
        }
        units
    }

    fn objects_at(&self, pos: &MapPos) -> Vec<&Object> {
        let mut objects = Vec::new();
        for (_, object) in self.objects() {
            if object.pos.map_pos == *pos {
                objects.push(object);
            }
        }
        objects
    }
}

pub trait GameStateMut: GameState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent);
}

// vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
