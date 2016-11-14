use rand::{thread_rng, Rng};
use game_state::{GameState, GameStateMut};
use partial_state::{PartialState};
use map::{distance};
use pathfinder::{self, Pathfinder, path_cost, truncate_path};
use dir::{Dir, dirs};
use unit::{Unit, UnitTypeId};
use db::{Db};
use misc::{get_shuffled_indices};
use check::{check_command};
use ::{
    CoreEvent,
    Command,
    MoveMode,
    PlayerId,
    ExactPos,
    ObjectClass,
    Object,
    MovePoints,
    MapPos,
    Options,
    get_free_exact_pos,
};

#[derive(Clone, Debug)]
pub struct Ai {
    id: PlayerId,
    state: PartialState,
    pathfinder: Pathfinder,
}

impl Ai {
    pub fn new(options: &Options, id: PlayerId) -> Ai {
        let state = PartialState::new(options, id);
        let map_size = state.map().size();
        Ai {
            id: id,
            state: state,
            pathfinder: Pathfinder::new(map_size),
        }
    }

    pub fn apply_event(&mut self, db: &Db, event: &CoreEvent) {
        self.state.apply_event(db, event);
    }

    fn get_best_pos(&self, db: &Db, unit: &Unit) -> Option<ExactPos> {
        let mut best_pos = None;
        let mut best_cost = pathfinder::max_cost();
        for enemy in self.state.units().values() {
            if enemy.player_id == self.id || !enemy.is_alive {
                continue;
            }
            for dir in dirs() {
                let pos = Dir::get_neighbour_pos(enemy.pos.map_pos, dir);
                if !self.state.map().is_inboard(pos) {
                    continue;
                }
                if let Some((cost, pos)) = self.estimate_path(db, unit, pos) {
                    if best_cost.n > cost.n {
                        best_cost = cost;
                        best_pos = Some(pos);
                    }
                }
            }
        }
        for sector in self.state.sectors().values() {
            if sector.owner_id == Some(self.id) {
                continue;
            }
            for &pos in &sector.positions {
                if unit.pos.map_pos == pos {
                    return None;
                }
                if let Some((cost, pos)) = self.estimate_path(db, unit, pos) {
                    if best_cost.n > cost.n {
                        best_cost = cost;
                        best_pos = Some(pos);
                    }
                }
            }
        }
        best_pos
    }

    fn estimate_path(
        &self,
        db: &Db,
        unit: &Unit,
        destination: MapPos,
    ) -> Option<(MovePoints, ExactPos)> {
        let exact_destination = match get_free_exact_pos(
            db, &self.state, unit.type_id, destination
        ) {
            Some(pos) => pos,
            None => return None,
        };
        let path = match self.pathfinder.get_path(exact_destination) {
            Some(path) => path,
            None => return None,
        };
        let cost = path_cost(db, &self.state, unit, &path);
        Some((cost, exact_destination))
    }

    fn is_close_to_enemies(&self, db: &Db, unit: &Unit) -> bool {
        for target in self.state.units().values() {
            if target.player_id == self.id {
                continue;
            }
            let target_type = db.unit_type(target.type_id);
            let attacker_type = db.unit_type(unit.type_id);
            let weapon_type = db.weapon_type(attacker_type.weapon_type_id);
            let distance = distance(unit.pos.map_pos, target.pos.map_pos);
            let max_distance = if target_type.is_air {
                match weapon_type.max_air_distance {
                    Some(max_air_distance) => max_air_distance,
                    None => continue, // can not attack air unit, skipping.
                }
            } else {
                weapon_type.max_distance
            };
            if distance <= max_distance {
                return true;
            }
        }
        false
    }

    pub fn try_get_attack_command(&self, db: &Db) -> Option<Command> {
        for unit in self.state.units().values() {
            if unit.player_id != self.id {
                continue;
            }
            if unit.attack_points.unwrap().n <= 0 {
                continue;
            }
            for target in self.state.units().values() {
                if target.player_id == self.id {
                    continue;
                }
                let command = Command::AttackUnit {
                    attacker_id: unit.id,
                    defender_id: target.id,
                };
                if check_command(db, self.id, &self.state, &command).is_ok() {
                    return Some(command);
                }
            }
        }
        None
    }

    pub fn try_get_move_command(&mut self, db: &Db) -> Option<Command> {
        for unit in self.state.units().values() {
            if unit.player_id != self.id {
                continue;
            }
            if self.is_close_to_enemies(db, unit) {
                continue;
            }
            self.pathfinder.fill_map(db, &self.state, unit);
            let destination = match self.get_best_pos(db, unit) {
                Some(destination) => destination,
                None => continue,
            };
            let path = match self.pathfinder.get_path(destination) {
                Some(path) => path,
                None => continue,
            };
            let path = match truncate_path(db, &self.state, &path, unit) {
                Some(path) => path,
                None => continue,
            };
            let cost = path_cost(db, &self.state, unit, &path);
            let move_points = unit.move_points.unwrap();
            if move_points.n < cost.n {
                continue;
            }
            let command = Command::Move {
                unit_id: unit.id,
                path: path,
                mode: MoveMode::Fast,
            };
            if check_command(db, self.id, &self.state, &command).is_err() {
                continue;
            }
            return Some(command);
        }
        None
    }

    fn get_shuffled_reinforcement_sectors(&self, player_id: PlayerId) -> Vec<&Object> {
        let mut reinforcement_sectors = Vec::new();
        for object in self.state.objects().values() {
            let owner_id = match object.owner_id {
                Some(id) => id,
                None => continue,
            };
            if owner_id != player_id {
                continue;
            }
            if object.class != ObjectClass::ReinforcementSector {
                continue;
            }
            reinforcement_sectors.push(object);
        }
        thread_rng().shuffle(&mut reinforcement_sectors);
        reinforcement_sectors
    }

    pub fn try_get_create_unit_command(&self, db: &Db) -> Option<Command> {
        let reinforcement_sectors = self.get_shuffled_reinforcement_sectors(self.id);
        let reinforcement_points = self.state.reinforcement_points()[&self.id];
        for type_index in get_shuffled_indices(db.unit_types()) {
            let unit_type_id = UnitTypeId{id: type_index as i32};
            let unit_type = db.unit_type(unit_type_id);
            if unit_type.cost > reinforcement_points {
                continue;
            }
            for sector in &reinforcement_sectors {
                let exact_pos = match get_free_exact_pos(
                    db,
                    &self.state,
                    unit_type_id,
                    sector.pos.map_pos,
                ) {
                    Some(pos) => pos,
                    None => continue,
                };
                let command = Command::CreateUnit {
                    type_id: unit_type_id,
                    pos: exact_pos,
                };
                if check_command(db, self.id, &self.state, &command).is_err() {
                    continue;
                }
                return Some(command);
            }
        }
        None
    }

    pub fn get_command(&mut self, db: &Db) -> Command {
        if let Some(cmd) = self.try_get_attack_command(db) {
            cmd
        } else if let Some(cmd) = self.try_get_move_command(db) {
            cmd
        } else if let Some(cmd) = self.try_get_create_unit_command(db) {
            cmd
        } else {
            Command::EndTurn
        }
    }
}
