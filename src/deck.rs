use crate::types::{Card, Rank, Suit};
use rand::seq::SliceRandom;
use rand::Rng;

const SUITS: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
const RANKS: [Rank; 8] = [
    Rank::Ace,
    Rank::King,
    Rank::Queen,
    Rank::Jack,
    Rank::Ten,
    Rank::Nine,
    Rank::Eight,
    Rank::Seven,
];

pub fn generate_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(32);
    for &suit in &SUITS {
        for &rank in &RANKS {
            deck.push(Card { suit, rank });
        }
    }
    // Sanity check: exactly 32 unique cards (4 suits × 8 ranks).
    debug_assert_eq!(deck.len(), 32, "Deck must contain exactly 32 cards");
    debug_assert!(
        {
            let mut seen = std::collections::HashSet::new();
            deck.iter().all(|c| seen.insert((c.suit as u8, c.rank as u8)))
        },
        "Deck contains duplicate cards"
    );
    deck
}

pub fn shuffle_deck(deck: &mut Vec<Card>, rng: &mut impl Rng) {
    deck.shuffle(rng);
}

pub fn deal_cards(deck: &mut Vec<Card>, count: usize) -> Vec<Card> {
    deck.drain(..count.min(deck.len())).collect()
}
