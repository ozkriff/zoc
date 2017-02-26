// TODO: subturns?
#[derive(Clone, Debug, PartialEq)]
pub struct Turns(i32);

// TODO: Timed? Как назвать вообще?
#[derive(Clone, Debug, PartialEq)]
pub struct TimedEffect {
    time: Turns,
    effect: Effect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Immobilized,
    WeaponBroken,
    ReducedMovement,
    // ReducedAttackPoints,
    // Destroyed(u8), // TODO: ?
    // Pinned, // пехотинцы "прижаты", должно бы заменить поле remove_move_points
}
