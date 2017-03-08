use unit::{Unit};
use movement::{MovePoints};

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
    WeaponBroken,
    ReducedMovement,
    ReducedAttackPoints,

    // TODO: это точно должен быть эффект вообще?
    // Destroyed(u8),

    // пехотинцы "прижаты", должно бы заменить поле remove_move_points
    // применимо только к пехоте
    Pinned,

    Suppressed{value: i32},

    // Убийство солдат,
    // применимо только к пехоте
    SoldierKilled,

    // уничтожение машины
    // применимо только к технике
    VehicleDestroyed,
}

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
