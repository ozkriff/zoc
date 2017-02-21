#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GameType {
    Hotseat,
    SingleVsAi,
}

impl Default for GameType {
    fn default() -> GameType {
        GameType::Hotseat
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub game_type: GameType,
    pub map_name: String,
    pub players_count: i32, // TODO: must it be defined by map/scenario?
}
