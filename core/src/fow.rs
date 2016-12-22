use std::default::{Default};
use std::rc::{Rc};
use types::{Size2};
use game_state::{GameState};
use map::{Map, Terrain, distance};
use fov::{fov, simple_fov};
use db::{Db};
use unit::{Unit, UnitType};
use ::{CoreEvent, PlayerId, MapPos, ExactPos, ObjectClass, SlotId};

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

fn calc_visibility<S: GameState>(
    state: &S,
    unit_type: &UnitType,
    origin: MapPos,
    pos: MapPos,
) -> TileVisibility {
    let distance = distance(origin, pos);
    if distance > unit_type.los_range {
        return TileVisibility::No;
    }
    if distance <= unit_type.cover_los_range {
        return TileVisibility::Excellent;
    }
    let mut vis = match *state.map().tile(pos) {
        Terrain::City | Terrain::Trees => TileVisibility::Normal,
        Terrain::Plain | Terrain::Water => TileVisibility::Excellent,
    };
    for object in state.objects_at(pos) {
        match object.class {
            // TODO: Removed Terrain::City and Terrain::Trees, use Smoke-like objects in logic
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
    pub fn new(db: Rc<Db>, map_size: Size2, player_id: PlayerId) -> Fow {
        Fow {
            map: Map::new(map_size),
            air_map: Map::new(map_size),
            player_id: player_id,
            db: db,
        }
    }

    pub fn is_ground_tile_visible(&self, pos: MapPos) -> bool {
        match *self.map.tile(pos) {
            TileVisibility::Excellent |
            TileVisibility::Normal => true,
            TileVisibility::No => false,
        }
    }

    pub fn is_visible(&self, unit: &Unit, pos: ExactPos) -> bool {
        if pos.slot_id == SlotId::Air {
            *self.air_map.tile(pos.map_pos) != TileVisibility::No
        } else if unit.is_loaded {
            false
        } else {
            let unit_type = self.db.unit_type(unit.type_id);
            match *self.map.tile(pos.map_pos) {
                TileVisibility::Excellent => true,
                TileVisibility::Normal => !unit_type.is_infantry,
                TileVisibility::No => false,
            }
        }
    }

    fn fov_unit<S: GameState>(&mut self, state: &S, unit: &Unit) {
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

    fn reset<S: GameState>(&mut self, state: &S) {
        self.clear();
        for (_, unit) in state.units() {
            if unit.player_id == self.player_id && unit.is_alive {
                self.fov_unit(state, unit);
            }
        }
    }

    pub fn apply_event<S: GameState>(
        &mut self,
        state: &S,
        event: &CoreEvent,
    ) {
        match *event {
            CoreEvent::Move{unit_id, ..} => {
                let unit = state.unit(unit_id);
                if unit.player_id == self.player_id {
                    self.fov_unit(state, unit);
                }
            },
            CoreEvent::EndTurn{new_id, ..} => {
                if self.player_id == new_id {
                    self.reset(state);
                }
            },
            CoreEvent::CreateUnit{ref unit_info} => {
                let unit = state.unit(unit_info.unit_id);
                if self.player_id == unit_info.player_id {
                    self.fov_unit(state, unit);
                }
            },
            CoreEvent::AttackUnit{ref attack_info} => {
                if let Some(attacker_id) = attack_info.attacker_id {
                    if !attack_info.is_ambush {
                        let pos = state.unit(attacker_id).pos;
                        // TODO: do not give away all units in this tile!
                        *self.map.tile_mut(pos) = TileVisibility::Excellent;
                    }
                }
            },
            CoreEvent::UnloadUnit{ref unit_info, ..} => {
                if self.player_id == unit_info.player_id {
                    let unit = state.unit(unit_info.unit_id);
                    self.fov_unit(state, unit);
                }
            },
            CoreEvent::Detach{transporter_id, ..} => {
                let transporter = state.unit(transporter_id);
                if self.player_id == transporter.player_id {
                    self.fov_unit(state, transporter);
                }
            },
            CoreEvent::Reveal{..} |
            CoreEvent::ShowUnit{..} |
            CoreEvent::HideUnit{..} |
            CoreEvent::LoadUnit{..} |
            CoreEvent::Attach{..} |
            CoreEvent::SetReactionFireMode{..} |
            CoreEvent::SectorOwnerChanged{..} |
            CoreEvent::Smoke{..} |
            CoreEvent::RemoveSmoke{..} |
            CoreEvent::VictoryPoint{..} => {},
        }
    }
}

#[derive(Clone, Debug)]
pub struct FakeFow;

pub fn fake_fow() -> &'static FakeFow {
    static FAKE_FOW: FakeFow = FakeFow;
    &FAKE_FOW
}

pub trait FogOfWar: Clone {
    fn is_visible(&self, unit: &Unit, pos: ExactPos) -> bool;
}

impl FogOfWar for FakeFow {
    fn is_visible(&self, _: &Unit, _: ExactPos) -> bool {
        true
    }
}

impl FogOfWar for Fow {
    fn is_visible(&self, unit: &Unit, pos: ExactPos) -> bool {
        self.is_visible(unit, pos)
    }
}
