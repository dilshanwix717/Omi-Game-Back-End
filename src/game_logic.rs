use crate::types::{Card, Play, Suit};

/// Determine the winner of a hand given the plays and trump suit.
pub fn resolve_hand_winner(plays: &[Play], trump: Suit) -> Option<u64> {
    if plays.is_empty() {
        return None;
    }

    let leading_suit = plays[0].card.suit;
    let mut best_play = &plays[0];

    for play in &plays[1..] {
        if beats(play.card, best_play.card, leading_suit, trump) {
            best_play = play;
        }
    }

    Some(best_play.player_id)
}

/// Check if `challenger` beats `current` given leading suit and trump.
fn beats(challenger: Card, current: Card, leading_suit: Suit, trump: Suit) -> bool {
    let challenger_is_trump = challenger.suit == trump;
    let current_is_trump = current.suit == trump;
    let challenger_follows_lead = challenger.suit == leading_suit;
    let current_follows_lead = current.suit == leading_suit;

    match (challenger_is_trump, current_is_trump) {
        // Both trump: higher rank wins
        (true, true) => challenger.rank.value() > current.rank.value(),
        // Challenger is trump, current is not: challenger wins
        (true, false) => true,
        // Challenger is not trump, current is: current stays
        (false, true) => false,
        // Neither is trump
        (false, false) => {
            if challenger_follows_lead && current_follows_lead {
                challenger.rank.value() > current.rank.value()
            } else if challenger_follows_lead {
                true
            } else {
                false
            }
        }
    }
}

/// Check if a player can legally play a card.
pub fn is_valid_play(
    card: Card,
    player_hand: &[Card],
    leading_suit: Option<Suit>,
) -> bool {
    // Card must exist in player's hand
    if !player_hand.contains(&card) {
        return false;
    }

    // If there's a leading suit, player must follow if they have that suit
    if let Some(lead) = leading_suit {
        let has_leading_suit = player_hand.iter().any(|c| c.suit == lead);
        if has_leading_suit && card.suit != lead {
            return false;
        }
    }

    true
}

/// Calculate round points based on Omi scoring rules.
pub struct RoundResult {
    pub team_a_round_points: u32,
    pub team_b_round_points: u32,
    pub is_tie: bool,
}

pub fn calculate_round_score(
    team_a_hands: u32,
    team_b_hands: u32,
    trump_selector_team_is_a: bool,
    previous_round_tied: bool,
) -> RoundResult {
    if team_a_hands == 4 && team_b_hands == 4 {
        return RoundResult {
            team_a_round_points: 0,
            team_b_round_points: 0,
            is_tie: true,
        };
    }

    let team_a_wins = team_a_hands > team_b_hands;
    let is_kapothi = team_a_hands == 8 || team_b_hands == 8;

    let (winner_points, loser_points) = if is_kapothi {
        if (team_a_wins && trump_selector_team_is_a) || (!team_a_wins && !trump_selector_team_is_a)
        {
            // Kapothi by trump-selecting team
            (2, 0)
        } else {
            // Kapothi by non-trump-selecting team
            (3, 0)
        }
    } else if (team_a_wins && trump_selector_team_is_a)
        || (!team_a_wins && !trump_selector_team_is_a)
    {
        // Trump-selecting team wins
        if previous_round_tied {
            (2, 0)
        } else {
            (1, 0)
        }
    } else {
        // Non-trump-selecting team wins
        (2, 0)
    };

    if team_a_wins {
        RoundResult {
            team_a_round_points: winner_points,
            team_b_round_points: loser_points,
            is_tie: false,
        }
    } else {
        RoundResult {
            team_a_round_points: loser_points,
            team_b_round_points: winner_points,
            is_tie: false,
        }
    }
}
