use db::{Db};
use unit::{Unit};
use game_state::{State};
use map::{Terrain};
use position::{MapPos};

pub fn print_unit_info(db: &Db, unit: &Unit) {
    let unit_type = db.unit_type(unit.type_id);
    let weapon_type = db.weapon_type(unit_type.weapon_type_id);
    println!("unit:");
    println!("  player_id: {}", unit.player_id.id);
    if let Some(move_points) = unit.move_points {
        println!("  move_points: {}", move_points.n);
    } else {
        println!("  move_points: ?");
    }
    if let Some(attack_points) = unit.attack_points {
        println!("  attack_points: {}", attack_points.n);
    } else {
        println!("  attack_points: ?");
    }
    if let Some(reactive_attack_points) = unit.reactive_attack_points {
        println!("  reactive_attack_points: {}", reactive_attack_points.n);
    } else {
        println!("  reactive_attack_points: ?");
    }
    println!("  count: {}", unit.count);
    println!("  morale: {}", unit.morale);
    println!("  passenger_id: {:?}", unit.passenger_id);
    println!("  attached_unit_id: {:?}", unit.attached_unit_id);
    println!("  is_alive: {:?}", unit.is_alive);
    println!("type:");
    println!("  name: {}", unit_type.name);
    println!("  is_infantry: {}", unit_type.is_infantry);
    println!("  count: {}", unit_type.count);
    println!("  size: {}", unit_type.size);
    println!("  armor: {}", unit_type.armor);
    println!("  toughness: {}", unit_type.toughness);
    println!("  weapon_skill: {}", unit_type.weapon_skill);
    println!("  mp: {}", unit_type.move_points.n);
    println!("  ap: {}", unit_type.attack_points.n);
    println!("  reactive_ap: {}", unit_type.reactive_attack_points.n);
    println!("  los_range: {}", unit_type.los_range.n);
    println!("  cover_los_range: {}", unit_type.cover_los_range.n);
    println!("weapon:");
    println!("  name: {}", weapon_type.name);
    println!("  damage: {}", weapon_type.damage);
    println!("  ap: {}", weapon_type.ap);
    println!("  accuracy: {}", weapon_type.accuracy);
    println!("  min_distance: {}", weapon_type.min_distance.n);
    println!("  max_distance: {}", weapon_type.max_distance.n);
    println!("  smoke: {:?}", weapon_type.smoke);
}

pub fn print_terrain_info(state: &State, pos: MapPos) {
    match *state.map().tile(pos) {
        Terrain::City => println!("City"),
        Terrain::Trees => println!("Trees"),
        Terrain::Plain => println!("Plain"),
        Terrain::Water => println!("Water"),
    }
}

pub fn print_pos_info(db: &Db, state: &State, pos: MapPos) {
    print_terrain_info(state, pos);
    println!("");
    for unit in state.units_at(pos) {
        print_unit_info(db, unit);
        println!("");
    }
}
