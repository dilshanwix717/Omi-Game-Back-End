use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rank {
    Ace,
    King,
    Queen,
    Jack,
    Ten,
    Nine,
    Eight,
    Seven,
}

impl Rank {
    pub fn value(self) -> u8 {
        match self {
            Rank::Ace => 7,
            Rank::King => 6,
            Rank::Queen => 5,
            Rank::Jack => 4,
            Rank::Ten => 3,
            Rank::Nine => 2,
            Rank::Eight => 1,
            Rank::Seven => 0,
        }
    }
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    WaitingForPlayers,
    WaitingForDeal,
    WaitingForTrump,
    Playing,
    RoundEnd,
    GameOver,
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomMode {
    Multiplayer,
    SinglePlayer,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq)]
pub struct Play {
    pub player_id: u64,
    pub card: Card,
}
