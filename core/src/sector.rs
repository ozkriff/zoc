use std::collections::{HashSet};
use cgmath::{Vector2};
use db::{Db};
use game_state::{State};
use position::{MapPos};
use event::{CoreEvent};
use player::{PlayerId};

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct SectorId{pub id: i32}

#[derive(Clone, Debug)]
pub struct Sector {
    pub owner_id: Option<PlayerId>,
    pub positions: Vec<MapPos>,
}

impl Sector {
    pub fn center(&self) -> MapPos {
        let mut pos = Vector2{x: 0.0, y: 0.0};
        for sector_pos in &self.positions {
            pos.x += sector_pos.v.x as f32;
            pos.y += sector_pos.v.y as f32;
        }
        pos /= self.positions.len() as f32;
        let pos = MapPos{v: Vector2{
            x: (pos.x + 0.5) as i32,
            y: (pos.y + 0.5) as i32,
        }};
        assert!(self.positions.contains(&pos));
        pos
    }
}

pub fn check_sectors(db: &Db, state: &State) -> Vec<CoreEvent> {
    let mut events = Vec::new();
    for (&sector_id, sector) in state.sectors() {
        let mut claimers = HashSet::new();
        for &pos in &sector.positions {
            for unit in state.units_at(pos) {
                let unit_type = db.unit_type(unit.type_id);
                if !unit_type.is_air && unit.is_alive {
                    claimers.insert(unit.player_id);
                }
            }
        }
        let owner_id = if claimers.len() != 1 {
            None
        } else {
            Some(claimers.into_iter().next().unwrap())
        };
        if sector.owner_id != owner_id {
            events.push(CoreEvent::SectorOwnerChanged {
                sector_id: sector_id,
                new_owner_id: owner_id,
            });
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use cgmath::{Vector2};
    use sector::{Sector};
    use position::{MapPos};

    #[test]
    fn test_center_1() {
        let real = Sector {
            positions: vec![
                MapPos{v: Vector2{x: 5, y: 0}},
                MapPos{v: Vector2{x: 6, y: 0}},
                MapPos{v: Vector2{x: 5, y: 1}},
                MapPos{v: Vector2{x: 6, y: 1}},
                MapPos{v: Vector2{x: 7, y: 1}},
                MapPos{v: Vector2{x: 5, y: 2}},
                MapPos{v: Vector2{x: 6, y: 2}},
            ],
            owner_id: None,
        }.center();
        let expected = MapPos{v: Vector2{x: 6, y: 1}};
        assert_eq!(expected, real);
    }

    #[test]
    fn test_center_2() {
        let real = Sector {
            positions: vec![
                MapPos{v: Vector2{x: 6, y: 0}},
                MapPos{v: Vector2{x: 6, y: 1}},
                MapPos{v: Vector2{x: 6, y: 2}},
            ],
            owner_id: None,
        }.center();
        let expected = MapPos{v: Vector2{x: 6, y: 1}};
        assert_eq!(expected, real);
    }
}
