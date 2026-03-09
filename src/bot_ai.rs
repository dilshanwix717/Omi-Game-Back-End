use crate::types::{Card, Play, Rank, Suit};
use rand::Rng;

/// Select trump suit from first 4 cards using bot AI logic.
pub fn select_trump(cards: &[Card]) -> Suit {
    let mut suit_counts: [(Suit, u8, u8); 4] = [
        (Suit::Hearts, 0, 0),
        (Suit::Diamonds, 0, 0),
        (Suit::Clubs, 0, 0),
        (Suit::Spades, 0, 0),
    ];

    for card in cards {
        for entry in suit_counts.iter_mut() {
            if entry.0 == card.suit {
                entry.1 += 1;
                if card.rank.value() > entry.2 {
                    entry.2 = card.rank.value();
                }
            }
        }
    }

    suit_counts.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    suit_counts[0].0
}

/// Decide which card to play given the bot's hand, current plays, trump suit, and teammate info.
pub fn decide_card(
    hand: &[Card],
    plays: &[Play],
    trump: Suit,
    teammate_id: Option<u64>,
    difficulty: u8, // 0=easy, 1=medium, 2=hard
    rng: &mut impl Rng,
) -> Card {
    if hand.is_empty() {
        panic!("Bot has no cards to play");
    }

    // Easy: 40% chance of random play; Medium: 10%; Hard: 0%
    let random_chance = match difficulty {
        0 => 40,
        1 => 10,
        _ => 0,
    };
    if rng.gen_range(0u8..100) < random_chance {
        return random_valid_card(hand, plays, rng);
    }

    if plays.is_empty() {
        first_player_strategy(hand, trump)
    } else {
        following_player_strategy(hand, plays, trump, teammate_id)
    }
}

// ── Random (for easy/medium difficulty) ────────────────────────────────

fn random_valid_card(hand: &[Card], plays: &[Play], rng: &mut impl Rng) -> Card {
    if plays.is_empty() {
        return hand[rng.gen_range(0..hand.len())];
    }
    let leading_suit = plays[0].card.suit;
    let has_lead = hand.iter().any(|c| c.suit == leading_suit);
    let valid: Vec<&Card> = hand
        .iter()
        .filter(|c| !has_lead || c.suit == leading_suit)
        .collect();
    *valid[rng.gen_range(0..valid.len())]
}

// ── First Player Strategy ──────────────────────────────────────────────

fn first_player_strategy(hand: &[Card], trump: Suit) -> Card {
    // 1. Has Non-Trump Ace? → Play it
    if let Some(card) = hand
        .iter()
        .find(|c| c.suit != trump && c.rank == Rank::Ace)
    {
        return *card;
    }

    // 2. Has a singleton from a Non-Trump Suit? → Play it
    if let Some(card) = find_singleton_non_trump(hand, trump) {
        return *card;
    }

    // 3. Has Non-Trump Cards? → Play lowest non-trump
    let non_trump: Vec<&Card> = hand.iter().filter(|c| c.suit != trump).collect();
    if !non_trump.is_empty() {
        return *lowest(&non_trump);
    }

    // 4. Only Trump Cards → Has Trump Ace? Play it; else lowest trump
    if let Some(card) = hand.iter().find(|c| c.rank == Rank::Ace) {
        return *card;
    }
    *lowest_from_slice(hand)
}

// ── Following Player Strategy ──────────────────────────────────────────

fn following_player_strategy(
    hand: &[Card],
    plays: &[Play],
    trump: Suit,
    teammate_id: Option<u64>,
) -> Card {
    let leading_suit = plays[0].card.suit;
    let has_leading_suit = hand.iter().any(|c| c.suit == leading_suit);

    if has_leading_suit {
        follow_with_leading_suit(hand, plays, leading_suit, trump, teammate_id)
    } else {
        follow_without_leading_suit(hand, plays, trump)
    }
}

// ── Has Leading Suit ───────────────────────────────────────────────────

fn follow_with_leading_suit(
    hand: &[Card],
    plays: &[Play],
    leading_suit: Suit,
    trump: Suit,
    teammate_id: Option<u64>,
) -> Card {
    let suit_cards: Vec<&Card> = hand.iter().filter(|c| c.suit == leading_suit).collect();
    let is_last_player = plays.len() == 3;

    if is_last_player {
        let current_winner = find_current_winner(plays, leading_suit, trump);
        let partner_winning = teammate_id
            .and_then(|tid| current_winner.map(|p| p.player_id == tid))
            .unwrap_or(false);

        if partner_winning {
            return *lowest(&suit_cards);
        }

        // Trump played by other players? (only meaningful when leading suit != trump)
        let trump_played = leading_suit != trump
            && plays.iter().any(|p| p.card.suit == trump);

        if trump_played {
            // Can't beat a trump with a leading-suit card → dump lowest
            return *lowest(&suit_cards);
        }

        // Try to beat the highest leading-suit card played so far
        let highest_lead = plays
            .iter()
            .filter(|p| p.card.suit == leading_suit)
            .map(|p| p.card.rank.value())
            .max()
            .unwrap_or(0);

        if let Some(card) = smallest_above(&suit_cards, highest_lead) {
            return *card;
        }
        return *lowest(&suit_cards);
    }

    // Not last player → play Ace of leading suit if we have it, else lowest
    if let Some(card) = suit_cards.iter().find(|c| c.rank == Rank::Ace) {
        return **card;
    }
    *lowest(&suit_cards)
}

// ── No Leading Suit ────────────────────────────────────────────────────

fn follow_without_leading_suit(
    hand: &[Card],
    plays: &[Play],
    trump: Suit,
) -> Card {
    let trump_cards: Vec<&Card> = hand.iter().filter(|c| c.suit == trump).collect();
    let other_cards: Vec<&Card> = hand.iter().filter(|c| c.suit != trump).collect();

    if trump_cards.is_empty() {
        // No trump cards — play singleton non-trump suit card if any, else lowest overall
        if let Some(card) = find_singleton_non_trump(hand, trump) {
            return *card;
        }
        return *lowest_from_slice(hand);
    }

    // Has trump cards
    let leading_suit = plays[0].card.suit;
    let trump_previously_played =
        leading_suit != trump && plays.iter().any(|p| p.card.suit == trump);

    if trump_previously_played {
        // A previous player already trumped — can we beat it?
        let highest_trump = plays
            .iter()
            .filter(|p| p.card.suit == trump)
            .map(|p| p.card.rank.value())
            .max()
            .unwrap_or(0);

        if let Some(card) = smallest_above(&trump_cards, highest_trump) {
            return *card;
        }
        // Can't beat existing trump → play lowest from other suits if possible
        if !other_cards.is_empty() {
            return *lowest(&other_cards);
        }
        return *lowest(&trump_cards);
    }

    // No trump played yet → cut with lowest trump
    *lowest(&trump_cards)
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Find the first card that is the only card of its non-trump suit in hand.
fn find_singleton_non_trump(hand: &[Card], trump: Suit) -> Option<&Card> {
    hand.iter()
        .filter(|c| c.suit != trump)
        .find(|c| hand.iter().filter(|h| h.suit == c.suit).count() == 1)
}

/// Return the lowest-ranked card from a slice of card references.
fn lowest<'a>(cards: &[&'a Card]) -> &'a Card {
    cards.iter().min_by_key(|c| c.rank.value()).unwrap()
}

/// Return the lowest-ranked card from a slice of cards.
fn lowest_from_slice(cards: &[Card]) -> &Card {
    cards.iter().min_by_key(|c| c.rank.value()).unwrap()
}

/// Return the card with the smallest rank strictly above `threshold`.
fn smallest_above<'a>(cards: &[&'a Card], threshold: u8) -> Option<&'a Card> {
    cards
        .iter()
        .filter(|c| c.rank.value() > threshold)
        .min_by_key(|c| c.rank.value())
        .copied()
}

/// Find the play that is currently winning the trick.
fn find_current_winner<'a>(plays: &'a [Play], leading_suit: Suit, trump: Suit) -> Option<&'a Play> {
    if plays.is_empty() {
        return None;
    }
    let mut best = &plays[0];
    for play in &plays[1..] {
        let dominated = match (play.card.suit == trump, best.card.suit == trump) {
            (true, true) => play.card.rank.value() > best.card.rank.value(),
            (true, false) => true,
            (false, true) => false,
            (false, false) => {
                play.card.suit == leading_suit
                    && (best.card.suit != leading_suit
                        || play.card.rank.value() > best.card.rank.value())
            }
        };
        if dominated {
            best = play;
        }
    }
    Some(best)
}
