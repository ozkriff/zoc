use position::{ExactPos};
use player::{PlayerId};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjectClass {
    Building,
    Road,
    Smoke,
    ReinforcementSector,
}

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ObjectId {
    pub id: i32,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub pos: ExactPos,
    pub class: ObjectClass,

    // TODO: можно таймер эффектов объединить с таймером объектов
    // и превратить в перечисление `enum Timer { Вечный, Ходов(i32) }`
    pub timer: Option<i32>,

    pub owner_id: Option<PlayerId>,
}
