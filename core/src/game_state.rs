use std::collections::{HashMap};
use unit::{Unit};
use db::{Db};
use map::{Map, Terrain};
use ::{CoreEvent, UnitId, ObjectId, Object, MapPos, Sector, SectorId, PlayerId, Score, objects_at};

pub trait GameState {
    fn map(&self) -> &Map<Terrain>;
    fn units(&self) -> &HashMap<UnitId, Unit>;
    fn objects(&self) -> &HashMap<ObjectId, Object>;
    fn sectors(&self) -> &HashMap<SectorId, Sector>;
    fn score(&self) -> &HashMap<PlayerId, Score>;
    fn reinforcement_points(&self) -> &HashMap<PlayerId, i32>;

    fn unit(&self, id: UnitId) -> &Unit {
        &self.units()[&id]
    }

    // TODO: Return iterator not vector
    fn units_at(&self, pos: MapPos) -> Vec<&Unit> {
        let mut units = Vec::new();
        for unit in self.units().values() {
            for map_pos in unit.pos.map_pos_iter() {
                if map_pos == pos {
                    units.push(unit);
                }
            }
        }
        units
    }

    fn objects_at(&self, pos: MapPos) -> Vec<&Object> {
        objects_at(self.objects(), pos)
    }
}

pub trait GameStateMut: GameState {
    fn apply_event(&mut self, db: &Db, event: &CoreEvent);
}
