use rand::{thread_rng, Rng};
use db::{Db};
use game_state::{State};
use unit::{Unit};
use misc::{clamp};
use map::{Terrain};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttackPoints{pub n: i32}

#[derive(PartialOrd, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct HitChance{pub n: i32}

pub fn hit_chance(
    db: &Db,
    state: &State,
    attacker: &Unit,
    defender: &Unit,
) -> HitChance {
    let attacker_type = db.unit_type(attacker.type_id);
    let defender_type = db.unit_type(defender.type_id);
    let weapon_type = db.weapon_type(attacker_type.weapon_type_id);
    let cover_bonus = cover_bonus(db, state, defender);
    let hit_test_v = -7 - cover_bonus + defender_type.size
        + weapon_type.accuracy + attacker_type.weapon_skill;
    let pierce_test_v = 10 + -defender_type.armor + weapon_type.ap;
    let wound_test_v = 5 -defender_type.toughness + weapon_type.damage;
    let hit_test_v = clamp(hit_test_v, 0, 10);
    let pierce_test_v = clamp(pierce_test_v, 0, 10);
    let wound_test_v = clamp(wound_test_v, 0, 10);
    let k = (hit_test_v * pierce_test_v * wound_test_v) / 10;
    HitChance{n: clamp(k, 0, 100)}
}

fn cover_bonus(db: &Db, state: &State, defender: &Unit) -> i32 {
    let defender_type = db.unit_type(defender.type_id);
    if defender_type.is_infantry {
        match *state.map().tile(defender.pos) {
            Terrain::Plain | Terrain::Water => 0,
            Terrain::Trees => 2,
            Terrain::City => 3,
        }
    } else {
        0
    }
}

pub fn get_killed_count(db: &Db, state: &State, attacker: &Unit, defender: &Unit) -> i32 {
    let hit = attack_test(db, state, attacker, defender);
    if !hit {
        return 0;
    }
    let defender_type = db.unit_type(defender.type_id);
    if defender_type.is_infantry {
        clamp(thread_rng().gen_range(1, 5), 1, defender.count)
    } else {
        1
    }
}

fn attack_test(db: &Db, state: &State, attacker: &Unit, defender: &Unit) -> bool {
    let k = hit_chance(db, state, attacker, defender).n;
    let r = thread_rng().gen_range(0, 100);
    r < k
}
