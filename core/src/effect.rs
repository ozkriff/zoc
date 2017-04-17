// use unit::{Unit, UnitId};
use movement::{MovePoints};
use attack::{AttackPoints};
// use game_state::{State};
// use event::{Event};

// #[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
// pub struct EffectId{pub id: i32}

// TODO: subturns? EffectTime?
#[derive(Clone, Debug, PartialEq)]
pub enum Time {
    Forever,
    Turns(i32),
    Instant,
}

// TODO: Timed? Как назвать вообще?
#[derive(Clone, Debug, PartialEq)]
pub struct TimedEffect {
    pub time: Time,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Immobilized,
    WeaponBroken, // TODO: WeaponId?
    ReducedMovementPoints(MovePoints),
    ReducedAttackPoints(AttackPoints),

    // TODO: это точно должен быть эффект вообще?
    // Destroyed(u8),

    // пехотинцы "прижаты", должно бы заменить поле remove_move_points
    // применимо только к пехоте
    Pinned,

    // Smoke(MapPos), // TODO ???

    ReducedAccuracy(i32), // TODO: ReducedAccuracy(i32, WeaponId)

    Suppressed(i32), // TODO: i32 -> Morale?

    // Убийство солдат,
    // применимо только к пехоте
    SoldierKilled(i32),

    // уничтожение машины
    // применимо только к технике
    VehicleDestroyed,

    Attacked(Attacked),
}

// TODO: это временное событие
// потом надо будет разбить его на части
#[derive(Clone, Debug, PartialEq)]
pub struct Attacked {
    // TODO в эффект "убито людей"
    pub killed: i32,

    // TODO в эффект "подавление"
    pub suppression: i32,

    // это точно нужно хранить?
    // TODO: в эффект "юнит уничтожен"?
    pub leave_wrecks: bool,

    // TODO: заменить на Effect::Pinned
    pub remove_move_points: bool,
}

// TODO: если тут не будет методов, то затолкать содержимое effect.rs м event.rs

/*
impl Effect {
    // есть чувство, что мне тут может понадобиться не просто юнит, а все состояние
    pub fn apply(&self, unit: &mut Unit) {
        match *self {
            Effect::Immobilized => {
                unit.move_points = Some(MovePoints{n: 0});
            },
            Effect::Pinned => {
                // TODO: какие еще последствия?
                // минус к точности стрельбы?
                unit.move_points = Some(MovePoints{n: 0});
            },
            _ => unimplemented!(),
        }
    }
}
*/
