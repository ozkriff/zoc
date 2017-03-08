use std::default::{Default};
use std::rc::{Rc};
use game_state::{State};
use map::{Map, Terrain, distance};
use fov::{fov, simple_fov};
use db::{Db};
use unit::{Unit, UnitType};
use position::{MapPos, ExactPos, SlotId};
use event::{CoreEvent, Event};
use player::{PlayerId};
use object::{ObjectClass};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub enum TileVisibility {
    No,
    // Bad,
    Normal,
    Excellent,
}

impl Default for TileVisibility {
    fn default() -> Self { TileVisibility::No }
}

fn calc_visibility(
    state: &State,
    unit_type: &UnitType,
    origin: MapPos,
    pos: MapPos,
) -> TileVisibility {
    let distance = distance(origin, pos);
    if distance > unit_type.los_range {
        return TileVisibility::No;
    }
    if !unit_type.is_air && distance <= unit_type.cover_los_range {
        return TileVisibility::Excellent;
    }
    let mut vis = match *state.map().tile(pos) {
        Terrain::City | Terrain::Trees => TileVisibility::Normal,
        Terrain::Plain | Terrain::Water => TileVisibility::Excellent,
    };
    for object in state.objects_at(pos) {
        match object.class {
            // TODO: Remove Terrain::City and Terrain::Trees, use Smoke-like objects in logic
            ObjectClass::Building | ObjectClass::Smoke => {
                vis = TileVisibility::Normal;
            }
            ObjectClass::Road |
            ObjectClass::ReinforcementSector => {},
        }
    }
    vis
}

/// Fog of War
#[derive(Clone, Debug)]
pub struct Fow {
    map: Map<TileVisibility>,
    air_map: Map<TileVisibility>,
    player_id: PlayerId,
    db: Rc<Db>,
}

impl Fow {
    pub fn new(state: &State, player_id: PlayerId) -> Fow {
        let db = state.db().clone();
        let map_size = state.map().size();
        let mut fow = Fow {
            map: Map::new(map_size),
            air_map: Map::new(map_size),
            player_id: player_id,
            db: db,
        };
        fow.reset(state);
        fow
    }

    pub fn is_ground_tile_visible(&self, pos: MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent |
            TileVisibility::Normal => true,
            TileVisibility::No => false,
        }
    }

    pub fn is_visible(&self, unit: &Unit) -> bool {
        self.is_visible_at(unit, unit.pos)
    }

    pub fn is_visible_at(&self, unit: &Unit, pos: ExactPos) -> bool {
        if pos.slot_id == SlotId::Air {
            *self.air_map.tile(pos.map_pos) != TileVisibility::No
        } else {
            let unit_type = self.db.unit_type(unit.type_id);
            match *self.map.tile(pos.map_pos) {
                TileVisibility::Excellent => true,
                TileVisibility::Normal => !unit_type.is_infantry,
                TileVisibility::No => false,
            }
        }
    }

    fn fov_unit(&mut self, state: &State, unit: &Unit) {
        assert!(unit.is_alive);
        let origin = unit.pos.map_pos;
        let unit_type = self.db.unit_type(unit.type_id);
        let range = unit_type.los_range;
        let ground_fow = &mut self.map;
        let ground_cb = &mut |pos| {
            let vis = calc_visibility(state, unit_type, origin, pos);
            if vis > *ground_fow.tile_mut(pos) {
                *ground_fow.tile_mut(pos) = vis;
            }
        };
        if unit.pos.slot_id == SlotId::Air {
            simple_fov(state, origin, range, ground_cb);
        } else {
            fov(state, origin, range, ground_cb);
        }
        let air_fow = &mut self.air_map;
        simple_fov(state, origin, range, &mut |pos| {
            *air_fow.tile_mut(pos) = TileVisibility::Excellent;
        });
    }

    fn clear(&mut self) {
        for pos in self.map.get_iter() {
            *self.map.tile_mut(pos) = TileVisibility::No;
            *self.air_map.tile_mut(pos) = TileVisibility::No;
        }
    }

    fn reset(&mut self, state: &State) {
        self.clear();
        for (_, unit) in state.units() {
            if unit.player_id == self.player_id && unit.is_alive {
                self.fov_unit(state, unit);
            }
        }
        for object in state.objects().values() {
            if object.class != ObjectClass::ReinforcementSector
                || object.owner_id != Some(self.player_id)
            {
                continue;
            }
            *self.map.tile_mut(object.pos) = TileVisibility::Excellent;
            *self.air_map.tile_mut(object.pos) = TileVisibility::Excellent;
        }
    }

    pub fn apply_event(
        &mut self,
        state: &State,
        event: &CoreEvent,
    ) {
        // match *event {
        match event.event {
            Event::Move{unit_id, ..} => {
                let unit = state.unit(unit_id);
                if unit.player_id == self.player_id {
                    self.fov_unit(state, unit);
                }
            },
            Event::EndTurn{new_id, ..} => {
                if self.player_id == new_id {
                    self.reset(state);
                }
            },
            Event::CreateUnit{ref unit_info} => {
                let unit = state.unit(unit_info.id);
                if self.player_id == unit_info.player_id {
                    self.fov_unit(state, unit);
                }
            },
            Event::AttackUnit{ref attack_info} => {
                if let Some(attacker_id) = attack_info.attacker_id {
                    if !attack_info.is_ambush {
                        let pos = state.unit(attacker_id).pos;
                        // TODO: do not give away all units in this tile!
                        *self.map.tile_mut(pos) = TileVisibility::Excellent;
                    }
                }
            },
            Event::UnloadUnit{ref unit_info, ..} => {
                if self.player_id == unit_info.player_id {
                    let unit = state.unit(unit_info.id);
                    self.fov_unit(state, unit);
                }
            },
            Event::Detach{transporter_id, ..} => {
                let transporter = state.unit(transporter_id);
                if self.player_id == transporter.player_id {
                    self.fov_unit(state, transporter);
                }
            },
            // Event::Effect{..} |
            Event::Reveal{..} |
            Event::ShowUnit{..} |
            Event::HideUnit{..} |
            Event::LoadUnit{..} |
            Event::Attach{..} |
            Event::SetReactionFireMode{..} |
            Event::SectorOwnerChanged{..} |
            Event::Smoke{..} |
            Event::RemoveSmoke{..} |
            Event::VictoryPoint{..} => {},
        }
    }
}
