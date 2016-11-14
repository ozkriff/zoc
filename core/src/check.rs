use std::{fmt, error};
use game_state::{GameState};
use map::{distance};
use pathfinder::{path_cost, tile_cost};
use unit::{Unit, UnitClass};
use db::{Db};
use fov::{fov, simple_fov};
use ::{
    Command,
    FireMode,
    PlayerId,
    ObjectClass,
    is_exact_pos_free,
    move_cost_modifier,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandError {
    TileIsOccupied,
    CanNotCommandEnemyUnits,
    NotInReinforcementSector,
    NotEnoughMovePoints,
    NotEnoughAttackPoints,
    NotEnoughReactiveAttackPoints,
    NotEnoughReinforcementPoints,
    BadMorale,
    OutOfRange,
    TooClose,
    NoLos,
    BadTransporterClass,
    BadPassengerClass,
    TransporterIsNotEmpty,
    TransporterIsEmpty,
    TransporterIsTooFarAway,
    PassengerHasNotEnoughMovePoints,
    UnloadDistanceIsTooBig,
    DestinationTileIsNotEmpty,
    BadUnitId,
    BadTransporterId,
    BadPassengerId,
    BadAttackerId,
    BadDefenderId,
    BadPath,
    BadUnitType,
    UnitIsDead,
}

impl CommandError {
    fn to_str(&self) -> &str {
        match *self {
            CommandError::TileIsOccupied => "Tile is occupied",
            CommandError::CanNotCommandEnemyUnits => "Can not command enemy units",
            CommandError::NotInReinforcementSector => "Not in reinforcement sector",
            CommandError::NotEnoughMovePoints => "Not enough move points",
            CommandError::NotEnoughAttackPoints => "No attack points",
            CommandError::NotEnoughReactiveAttackPoints => "No reactive attack points",
            CommandError::NotEnoughReinforcementPoints => "No reinforcement points",
            CommandError::BadMorale => "Can`t attack when suppresset",
            CommandError::OutOfRange => "Out of range",
            CommandError::TooClose => "Too close",
            CommandError::NoLos => "No Line of Sight",
            CommandError::BadTransporterClass => "Bad transporter class",
            CommandError::BadPassengerClass => "Bad passenger class",
            CommandError::TransporterIsNotEmpty => "Transporter is not empty",
            CommandError::TransporterIsEmpty => "Transporter is empty",
            CommandError::TransporterIsTooFarAway => "Transporter is too far away",
            CommandError::PassengerHasNotEnoughMovePoints => "Passenger has not enough move points",
            CommandError::UnloadDistanceIsTooBig => "Unload pos it too far away",
            CommandError::DestinationTileIsNotEmpty => "Destination tile is not empty",
            CommandError::BadUnitId => "Bad unit id",
            CommandError::BadTransporterId => "Bad transporter id",
            CommandError::BadPassengerId => "Bad passenger id",
            CommandError::BadAttackerId => "Bad attacker id",
            CommandError::BadDefenderId => "Bad defender id",
            CommandError::BadPath => "Bad path",
            CommandError::BadUnitType => "Bad unit type",
            CommandError::UnitIsDead => "Unit is dead",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl error::Error for CommandError {
    fn description(&self) -> &str {
        self.to_str()
    }
}

pub fn check_command<S: GameState>(
    db: &Db,
    player_id: PlayerId,
    state: &S,
    command: &Command,
) -> Result<(), CommandError> {
    match *command {
        Command::EndTurn => Ok(()),
        Command::CreateUnit{pos, type_id} => {
            let mut is_sector = false;
            for object in state.objects_at(pos.map_pos) {
                if object.class == ObjectClass::ReinforcementSector {
                    is_sector = true;
                    break;
                }
            }
            if !is_sector {
                return Err(CommandError::NotInReinforcementSector);
            }
            let unit_type = db.unit_type(type_id);
            let reinforcement_points = state.reinforcement_points()[&player_id];
            if unit_type.cost > reinforcement_points {
                return Err(CommandError::NotEnoughReinforcementPoints);
            }
            if !is_exact_pos_free(db, state, type_id, pos) {
                return Err(CommandError::TileIsOccupied);
            }
            Ok(())
        },
        Command::Move{unit_id, ref path, mode} => {
            let unit = state.unit(unit_id);
            if !unit.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if unit.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            if path.len() < 2 {
                return Err(CommandError::BadPath);
            }
            if state.units().get(&unit_id).is_none() {
                return Err(CommandError::BadUnitId);
            }
            let unit = state.unit(unit_id);
            for window in path.windows(2) {
                let pos = window[1];
                if !is_exact_pos_free(db, state, unit.type_id, pos) {
                    return Err(CommandError::BadPath);
                }
            }
            let cost = path_cost(db, state, unit, path).n
                * move_cost_modifier(mode);
            let move_points = unit.move_points.unwrap();
            if cost > move_points.n {
                return Err(CommandError::NotEnoughMovePoints);
            }
            Ok(())
        },
        Command::AttackUnit{attacker_id, defender_id} => {
            if state.units().get(&attacker_id).is_none() {
                return Err(CommandError::BadAttackerId);
            }
            if state.units().get(&defender_id).is_none() {
                return Err(CommandError::BadDefenderId);
            }
            let attacker = state.unit(attacker_id);
            if !attacker.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if attacker.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            let defender = state.unit(defender_id);
            if !defender.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            check_attack(db, state, attacker, defender, FireMode::Active)
        },
        Command::LoadUnit{transporter_id, passenger_id} => {
            if state.units().get(&transporter_id).is_none() {
                return Err(CommandError::BadTransporterId);
            }
            if state.units().get(&passenger_id).is_none() {
                return Err(CommandError::BadPassengerId);
            }
            let passenger = state.unit(passenger_id);
            if !passenger.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            let transporter = state.unit(transporter_id);
            if !transporter.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if passenger.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            if transporter.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            if !db.unit_type(transporter.type_id).is_transporter {
                return Err(CommandError::BadTransporterClass);
            }
            match db.unit_type(passenger.type_id).class {
                UnitClass::Infantry => {},
                _ => {
                    return Err(CommandError::BadPassengerClass);
                }
            }
            if transporter.passenger_id.is_some() {
                return Err(CommandError::TransporterIsNotEmpty);
            }
            if distance(transporter.pos.map_pos, passenger.pos.map_pos) > 1 {
                return Err(CommandError::TransporterIsTooFarAway);
            }
            // TODO: 0 -> real move cost of transport tile for passenger
            let passenger_move_points = passenger.move_points.unwrap();
            if passenger_move_points.n == 0 {
                return Err(CommandError::PassengerHasNotEnoughMovePoints);
            }
            Ok(())
        },
        Command::UnloadUnit{transporter_id, passenger_id, pos} => {
            if state.units().get(&transporter_id).is_none() {
                return Err(CommandError::BadTransporterId);
            }
            let passenger = match state.units().get(&passenger_id) {
                Some(passenger) => passenger,
                None => return Err(CommandError::BadPassengerId),
            };
            if !passenger.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            let transporter = state.unit(transporter_id);
            if !transporter.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if passenger.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            if transporter.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            if !db.unit_type(transporter.type_id).is_transporter {
                return Err(CommandError::BadTransporterClass);
            }
            if distance(transporter.pos.map_pos, pos.map_pos) > 1 {
                return Err(CommandError::UnloadDistanceIsTooBig);
            }
            if transporter.passenger_id.is_none() {
                return Err(CommandError::TransporterIsEmpty);
            }
            if !is_exact_pos_free(db, state, passenger.type_id, pos) {
                return Err(CommandError::DestinationTileIsNotEmpty);
            }
            let passenger_type = db.unit_type(passenger.type_id);
            let cost = tile_cost(db, state, passenger, transporter.pos, pos);
            if cost.n > passenger_type.move_points.n {
                return Err(CommandError::NotEnoughMovePoints);
            }
            Ok(())
        },
        Command::SetReactionFireMode{unit_id, ..} => {
            let unit = match state.units().get(&unit_id) {
                Some(unit) => unit,
                None => return Err(CommandError::BadUnitId),
            };
            if !unit.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if unit.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            Ok(())
        },
        Command::Smoke{unit_id, pos} => {
            let unit = match state.units().get(&unit_id) {
                Some(unit) => unit,
                None => return Err(CommandError::BadUnitId),
            };
            if !unit.is_alive {
                return Err(CommandError::UnitIsDead);
            }
            if unit.player_id != player_id {
                return Err(CommandError::CanNotCommandEnemyUnits);
            }
            let unit_type = db.unit_type(unit.type_id);
            let weapon_type = db.weapon_type(unit_type.weapon_type_id);
            if !weapon_type.smoke.is_some() {
                return Err(CommandError::BadUnitType);
            }
            if distance(unit.pos.map_pos, pos) > weapon_type.max_distance {
                return Err(CommandError::OutOfRange);
            }
            let attack_points = unit.attack_points.unwrap();
            if attack_points.n != unit_type.attack_points.n {
                return Err(CommandError::NotEnoughAttackPoints);
            }
            Ok(())
        },
    }
}

pub fn check_attack<S: GameState>(
    db: &Db,
    state: &S,
    attacker: &Unit,
    defender: &Unit,
    fire_mode: FireMode,
) -> Result<(), CommandError> {
    if !attacker.is_alive {
        return Err(CommandError::UnitIsDead);
    }
    if !defender.is_alive {
        return Err(CommandError::UnitIsDead);
    }
    let attack_points = attacker.attack_points.unwrap();
    let reactive_attack_points = attacker.reactive_attack_points.unwrap();
    match fire_mode {
        FireMode::Active => if attack_points.n <= 0 {
            return Err(CommandError::NotEnoughAttackPoints);
        },
        FireMode::Reactive => if reactive_attack_points.n <= 0 {
            return Err(CommandError::NotEnoughReactiveAttackPoints);
        },
    }
    let minimal_ok_morale = 50;
    if attacker.morale < minimal_ok_morale {
        return Err(CommandError::BadMorale);
    }
    let attacker_type = db.unit_type(attacker.type_id);
    let defender_type = db.unit_type(defender.type_id);
    let weapon_type = db.weapon_type(attacker_type.weapon_type_id);
    let distance =  distance(attacker.pos.map_pos, defender.pos.map_pos);
    if defender_type.is_air {
        if let Some(max_air_distance) = weapon_type.max_air_distance {
            if distance > max_air_distance {
                return Err(CommandError::OutOfRange);
            }
        } else {
            return Err(CommandError::OutOfRange);
        }
    } else {
        if distance > weapon_type.max_distance {
            return Err(CommandError::OutOfRange);
        }
        if distance < weapon_type.min_distance {
            return Err(CommandError::TooClose);
        }
    }
    let is_los_ok = los(db, state, attacker, defender);
    if !weapon_type.is_inderect && !is_los_ok {
        return Err(CommandError::NoLos);
    }
    Ok(())
}

// TODO: profile and optimize!
fn los<S: GameState>(
    db: &Db,
    state: &S,
    attacker: &Unit,
    defender: &Unit,
) -> bool {
    let attacker_type = db.unit_type(attacker.type_id);
    let defender_type = db.unit_type(defender.type_id);
    let from = attacker.pos.map_pos;
    let to = defender.pos.map_pos;
    let range = attacker_type.los_range;
    let mut v = false;
    let f = if attacker_type.is_air || defender_type.is_air {
        simple_fov
    } else {
        fov
    };
    f(state, from, range, &mut |p| if p == to { v = true });
    v
}
