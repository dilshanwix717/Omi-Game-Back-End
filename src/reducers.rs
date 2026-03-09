// SpacetimeDB table definitions and server reducers for Omi card game
// Server-authoritative: all game logic runs here

use spacetimedb::{table, reducer, Identity, ReducerContext, Table};
use crate::types::*;
use crate::deck;
use crate::game_logic;
use crate::bot_ai;

// ─── Tables ────────────────────────────────────────────────────────────────────

#[table(name = "player", accessor = player, public)]
pub struct Player {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub username: String,
    pub identity: Identity,
    pub room_id: u64,
    pub team: String, // "A" or "B"
    pub seat_index: u8,
    pub is_bot: bool,
    pub is_connected: bool,
}

#[derive(Clone)]
#[table(name = "room", accessor = room, public)]
pub struct Room {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub room_code: String,
    pub status: GameStatus,
    pub mode: RoomMode,
    pub dealer_index: u8,
    pub trump_selector_index: u8,
    pub current_turn_index: u8,
    pub trump_suit: Option<Suit>,
    pub previous_round_tied: bool,
    pub hand_number: u8,
    pub deck: Vec<Card>,
}

#[table(name = "player_hand", accessor = player_hand, public)]
pub struct PlayerHand {
    #[primary_key]
    pub player_id: u64,
    pub cards: Vec<Card>,
}

#[table(name = "hand_record", accessor = hand_record, public)]
pub struct HandRecord {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub room_id: u64,
    pub hand_number: u8,
    pub plays: Vec<Play>,
    pub winner_id: Option<u64>,
    pub leading_suit: Option<Suit>,
}

#[derive(Clone)]
#[table(name = "score", accessor = score, public)]
pub struct Score {
    #[primary_key]
    pub room_id: u64,
    pub team_a_points: u32,
    pub team_b_points: u32,
    pub team_a_hands: u32,
    pub team_b_hands: u32,
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn generate_room_code(ctx: &ReducerContext) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    ctx.sender().hash(&mut hasher);
    ctx.timestamp.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:06}", hash % 1_000_000)
}

fn get_players_in_room(ctx: &ReducerContext, room_id: u64) -> Vec<Player> {
    ctx.db.player().iter().filter(|p| p.room_id == room_id).collect()
}

fn get_player_by_identity(ctx: &ReducerContext, room_id: u64, identity: &Identity) -> Option<Player> {
    ctx.db.player().iter().find(|p| p.room_id == room_id && p.identity == *identity)
}

fn get_player_by_seat(ctx: &ReducerContext, room_id: u64, seat_index: u8) -> Option<Player> {
    ctx.db.player().iter().find(|p| p.room_id == room_id && p.seat_index == seat_index)
}

fn get_current_hand_record(ctx: &ReducerContext, room_id: u64, hand_number: u8) -> Option<HandRecord> {
    // Use max_by_key so that if stale records from a previous round share the same
    // hand_number, we always return the most-recently inserted one (highest id).
    ctx.db.hand_record().iter()
        .filter(|h| h.room_id == room_id && h.hand_number == hand_number)
        .max_by_key(|h| h.id)
}

/// Assign team based on seat: seats 0,2 → "A", seats 1,3 → "B"
fn team_for_seat(seat: u8) -> String {
    if seat % 2 == 0 { "A".to_string() } else { "B".to_string() }
}

/// Get the teammate's player ID for a given player
fn get_teammate_id(ctx: &ReducerContext, player: &Player) -> Option<u64> {
    let teammate_seat = (player.seat_index + 2) % 4;
    get_player_by_seat(ctx, player.room_id, teammate_seat).map(|p| p.id)
}

fn make_rng(ctx: &ReducerContext) -> rand::rngs::StdRng {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use rand::SeedableRng;
    let mut hasher = DefaultHasher::new();
    ctx.sender().hash(&mut hasher);
    ctx.timestamp.hash(&mut hasher);
    rand::rngs::StdRng::seed_from_u64(hasher.finish())
}

// ─── Reducers ──────────────────────────────────────────────────────────────────

#[reducer]
pub fn create_room(ctx: &ReducerContext, username: String) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    let room_code = generate_room_code(ctx);

    let room = ctx.db.room().insert(Room {
        id: 0, // auto_inc
        room_code,
        status: GameStatus::WaitingForPlayers,
        mode: RoomMode::Multiplayer,
        dealer_index: 0,
        trump_selector_index: 1,
        current_turn_index: 0,
        trump_suit: None,
        previous_round_tied: false,
        hand_number: 0,
        deck: Vec::new(),
    });

    ctx.db.player().insert(Player {
        id: 0, // auto_inc
        username,
        identity: ctx.sender(),
        room_id: room.id,
        team: team_for_seat(0),
        seat_index: 0,
        is_bot: false,
        is_connected: true,
    });

    ctx.db.score().insert(Score {
        room_id: room.id,
        team_a_points: 0,
        team_b_points: 0,
        team_a_hands: 0,
        team_b_hands: 0,
    });

    log::info!("Room {} created with code {}", room.id, room.room_code);
    Ok(())
}

#[reducer]
pub fn join_room_by_code(ctx: &ReducerContext, room_code: String, username: String) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    let room = ctx.db.room().iter()
        .find(|r| r.room_code == room_code)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::WaitingForPlayers {
        return Err("Game has already started".to_string());
    }

    let players = get_players_in_room(ctx, room.id);
    if players.len() >= 4 {
        return Err("Room is full".to_string());
    }

    // Check if player already in room
    if players.iter().any(|p| p.identity == ctx.sender()) {
        return Err("You are already in this room".to_string());
    }

    let seat_index = players.len() as u8;

    ctx.db.player().insert(Player {
        id: 0,
        username,
        identity: ctx.sender(),
        room_id: room.id,
        team: team_for_seat(seat_index),
        seat_index,
        is_bot: false,
        is_connected: true,
    });

    // If 4 players joined, transition to WaitingForDeal
    if players.len() + 1 == 4 {
        let mut room = room;
        room.status = GameStatus::WaitingForDeal;
        ctx.db.room().id().update(room);
    }

    Ok(())
}

#[reducer]
pub fn join_room_by_link(ctx: &ReducerContext, room_code: String, username: String) -> Result<(), String> {
    // Same logic as join_room_by_code — the link contains the room code
    join_room_by_code(ctx, room_code, username)
}

#[reducer]
pub fn create_single_player_game(ctx: &ReducerContext, username: String) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    let room_code = generate_room_code(ctx);

    let room = ctx.db.room().insert(Room {
        id: 0,
        room_code,
        status: GameStatus::WaitingForDeal,
        mode: RoomMode::SinglePlayer,
        dealer_index: 0,
        trump_selector_index: 1,
        current_turn_index: 0,
        trump_suit: None,
        previous_round_tied: false,
        hand_number: 0,
        deck: Vec::new(),
    });

    // Human player at seat 0
    ctx.db.player().insert(Player {
        id: 0,
        username: username.clone(),
        identity: ctx.sender(),
        room_id: room.id,
        team: team_for_seat(0),
        seat_index: 0,
        is_bot: false,
        is_connected: true,
    });

    // 3 bot players at seats 1, 2, 3
    let bot_names = ["Bot 1", "Bot 2", "Bot 3"];
    for (i, name) in bot_names.iter().enumerate() {
        let seat = (i + 1) as u8;
        ctx.db.player().insert(Player {
            id: 0,
            username: name.to_string(),
            identity: ctx.sender(), // bots share creator's identity
            room_id: room.id,
            team: team_for_seat(seat),
            seat_index: seat,
            is_bot: true,
            is_connected: true,
        });
    }

    ctx.db.score().insert(Score {
        room_id: room.id,
        team_a_points: 0,
        team_b_points: 0,
        team_a_hands: 0,
        team_b_hands: 0,
    });

    log::info!("Single player game created: room {}", room.id);
    Ok(())
}

#[reducer]
pub fn dealer_click_deal(ctx: &ReducerContext, room_id: u64) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::WaitingForDeal {
        return Err("Not in dealing phase".to_string());
    }

    // Verify caller is the dealer (or any player in single-player mode)
    let caller = get_player_by_identity(ctx, room_id, &ctx.sender())
        .ok_or_else(|| "You are not in this room".to_string())?;

    if room.mode == RoomMode::Multiplayer && caller.seat_index != room.dealer_index {
        return Err("Only the dealer can deal".to_string());
    }

    // Generate and shuffle deck
    let mut new_deck = deck::generate_deck();
    let mut rng = make_rng(ctx);
    deck::shuffle_deck(&mut new_deck, &mut rng);

    // Deal first 4 cards to each player
    let players = get_players_in_room(ctx, room_id);
    let mut players_sorted: Vec<Player> = players;
    players_sorted.sort_by_key(|p| p.seat_index);

    for player in &players_sorted {
        let cards = deck::deal_cards(&mut new_deck, 4);
        // Remove existing hand if any
        if let Some(existing) = ctx.db.player_hand().player_id().find(player.id) {
            ctx.db.player_hand().player_id().delete(existing.player_id);
        }
        ctx.db.player_hand().insert(PlayerHand {
            player_id: player.id,
            cards,
        });
    }

    // Update room: store remaining deck, set status to trump selection
    let trump_selector_index = (room.dealer_index + 1) % 4;
    let mut room = room;
    room.deck = new_deck;
    room.status = GameStatus::WaitingForTrump;
    room.trump_selector_index = trump_selector_index;
    room.current_turn_index = trump_selector_index;
    room.hand_number = 1;
    room.trump_suit = None;
    ctx.db.room().id().update(room.clone());

    // If trump selector is a bot, auto-select trump
    if let Some(selector) = get_player_by_seat(ctx, room_id, trump_selector_index) {
        if selector.is_bot {
            if let Some(hand) = ctx.db.player_hand().player_id().find(selector.id) {
                let trump = bot_ai::select_trump(&hand.cards);
                do_select_trump(ctx, room_id, trump)?;
            }
        }
    }

    Ok(())
}

#[reducer]
pub fn select_trump(ctx: &ReducerContext, room_id: u64, suit: Suit) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::WaitingForTrump {
        return Err("Not in trump selection phase".to_string());
    }

    let caller = get_player_by_identity(ctx, room_id, &ctx.sender())
        .ok_or_else(|| "You are not in this room".to_string())?;

    if caller.seat_index != room.trump_selector_index {
        return Err("You are not the trump selector".to_string());
    }

    do_select_trump(ctx, room_id, suit)
}

/// Internal function to handle trump selection and dealing remaining cards
fn do_select_trump(ctx: &ReducerContext, room_id: u64, suit: Suit) -> Result<(), String> {
    let mut room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    let mut remaining_deck = room.deck.clone();

    // Deal remaining 4 cards to each player
    let players = get_players_in_room(ctx, room_id);
    let mut players_sorted: Vec<Player> = players;
    players_sorted.sort_by_key(|p| p.seat_index);

    for player in &players_sorted {
        let new_cards = deck::deal_cards(&mut remaining_deck, 4);
        if let Some(mut hand) = ctx.db.player_hand().player_id().find(player.id) {
            ctx.db.player_hand().player_id().delete(hand.player_id);
            hand.cards.extend(new_cards);
            ctx.db.player_hand().insert(hand);
        }
    }

    // Purge hand records from previous rounds so stale entries with the same
    // hand_number (1-8) cannot be confused with the current round's records.
    let stale_ids: Vec<u64> = ctx.db.hand_record().iter()
        .filter(|h| h.room_id == room_id)
        .map(|h| h.id)
        .collect();
    for stale_id in stale_ids {
        ctx.db.hand_record().id().delete(stale_id);
    }

    // Create the first hand record for this round
    ctx.db.hand_record().insert(HandRecord {
        id: 0,
        room_id,
        hand_number: 1,
        plays: Vec::new(),
        winner_id: None,
        leading_suit: None,
    });

    // Reset hand scores for this round
    if let Some(score) = ctx.db.score().room_id().find(room_id) {
        let mut score = score;
        score.team_a_hands = 0;
        score.team_b_hands = 0;
        ctx.db.score().room_id().update(score);
    }

    // Update room state
    room.trump_suit = Some(suit);
    room.status = GameStatus::Playing;
    room.current_turn_index = room.trump_selector_index;
    room.deck = remaining_deck;
    ctx.db.room().id().update(room.clone());

    // If current turn is a bot, trigger bot play
    maybe_run_bot_turn(ctx, room_id)?;

    Ok(())
}

#[reducer]
pub fn play_card(ctx: &ReducerContext, room_id: u64, card: Card) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::Playing {
        return Err("Game is not in playing phase".to_string());
    }

    let caller = get_player_by_identity(ctx, room_id, &ctx.sender())
        .ok_or_else(|| "You are not in this room".to_string())?;

    if caller.seat_index != room.current_turn_index {
        return Err("It's not your turn".to_string());
    }

    do_play_card(ctx, room_id, caller.id, card)
}

/// Internal card play logic shared by human and bot players
fn do_play_card(ctx: &ReducerContext, room_id: u64, player_id: u64, card: Card) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    let trump = room.trump_suit
        .ok_or_else(|| "Trump suit not set".to_string())?;

    // Get player's hand
    let hand = ctx.db.player_hand().player_id().find(player_id)
        .ok_or_else(|| "Player hand not found".to_string())?;

    // Get current hand record
    let hand_record = get_current_hand_record(ctx, room_id, room.hand_number)
        .ok_or_else(|| "Hand record not found".to_string())?;

    // Determine leading suit from first play
    let leading_suit = if hand_record.plays.is_empty() {
        None
    } else {
        Some(hand_record.plays[0].card.suit)
    };

    // Validate the play
    if !game_logic::is_valid_play(card, &hand.cards, leading_suit) {
        return Err("You must follow the leading suit".to_string());
    }

    // Remove card from player's hand
    let mut updated_cards = hand.cards.clone();
    let card_pos = updated_cards.iter().position(|c| *c == card)
        .ok_or_else(|| "Card not in hand".to_string())?;
    updated_cards.remove(card_pos);
    ctx.db.player_hand().player_id().delete(hand.player_id);
    ctx.db.player_hand().insert(PlayerHand {
        player_id,
        cards: updated_cards,
    });

    // Record the play
    let mut updated_record = hand_record;
    let new_leading_suit = if updated_record.plays.is_empty() {
        Some(card.suit)
    } else {
        updated_record.leading_suit
    };
    updated_record.plays.push(Play { player_id, card });
    updated_record.leading_suit = new_leading_suit;

    let plays_count = updated_record.plays.len();

    // Check if hand is complete (4 plays)
    if plays_count == 4 {
        // Resolve hand winner
        let winner_id = game_logic::resolve_hand_winner(&updated_record.plays, trump);
        updated_record.winner_id = winner_id;

        // Update hand record
        ctx.db.hand_record().id().delete(updated_record.id);
        ctx.db.hand_record().insert(updated_record);

        // Update score
        if let Some(winner_id) = winner_id {
            let winner = ctx.db.player().id().find(winner_id);
            if let Some(winner) = winner {
                if let Some(score) = ctx.db.score().room_id().find(room_id) {
                    let mut score = score;
                    if winner.team == "A" {
                        score.team_a_hands += 1;
                    } else {
                        score.team_b_hands += 1;
                    }
                    ctx.db.score().room_id().update(score);
                }

                // Check if round is over (8 hands played)
                if room.hand_number >= 8 {
                    do_end_round(ctx, room_id)?;
                } else {
                    // Next hand: winner leads
                    let next_hand = room.hand_number + 1;
                    let mut room = room;
                    room.hand_number = next_hand;
                    room.current_turn_index = winner.seat_index;
                    ctx.db.room().id().update(room);

                    // Create next hand record
                    ctx.db.hand_record().insert(HandRecord {
                        id: 0,
                        room_id,
                        hand_number: next_hand,
                        plays: Vec::new(),
                        winner_id: None,
                        leading_suit: None,
                    });

                    // If next player is bot, trigger bot
                    maybe_run_bot_turn(ctx, room_id)?;
                }
            }
        }
    } else {
        // Update hand record with new play
        ctx.db.hand_record().id().delete(updated_record.id);
        ctx.db.hand_record().insert(updated_record);

        // Advance turn clockwise
        let mut room = room;
        room.current_turn_index = (room.current_turn_index + 1) % 4;
        ctx.db.room().id().update(room);

        // If next player is bot, trigger bot
        maybe_run_bot_turn(ctx, room_id)?;
    }

    Ok(())
}

/// If the current turn belongs to a bot, automatically play for it
fn maybe_run_bot_turn(ctx: &ReducerContext, room_id: u64) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::Playing {
        return Ok(());
    }

    let current_player = get_player_by_seat(ctx, room_id, room.current_turn_index);

    if let Some(player) = current_player {
        if player.is_bot {
            let trump = room.trump_suit
                .ok_or_else(|| "Trump suit not set".to_string())?;

            let hand = ctx.db.player_hand().player_id().find(player.id)
                .ok_or_else(|| "Bot hand not found".to_string())?;

            let hand_record = get_current_hand_record(ctx, room_id, room.hand_number)
                .ok_or_else(|| "Hand record not found".to_string())?;

            let teammate_id = get_teammate_id(ctx, &player);

            let mut rng = make_rng(ctx);
            let card = bot_ai::decide_card(
                &hand.cards,
                &hand_record.plays,
                trump,
                teammate_id,
                1, // medium difficulty
                &mut rng,
            );

            do_play_card(ctx, room_id, player.id, card)?;
        }
    }

    Ok(())
}

#[reducer]
pub fn run_bot_decision(ctx: &ReducerContext, room_id: u64) -> Result<(), String> {
    maybe_run_bot_turn(ctx, room_id)
}

/// End the current round, calculate scores, check for game over
fn do_end_round(ctx: &ReducerContext, room_id: u64) -> Result<(), String> {
    let mut room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    let score = ctx.db.score().room_id().find(room_id)
        .ok_or_else(|| "Score not found".to_string())?;

    // Determine if trump selector's team is A
    let trump_selector = get_player_by_seat(ctx, room_id, room.trump_selector_index)
        .ok_or_else(|| "Trump selector not found".to_string())?;
    let trump_team_is_a = trump_selector.team == "A";

    let result = game_logic::calculate_round_score(
        score.team_a_hands,
        score.team_b_hands,
        trump_team_is_a,
        room.previous_round_tied,
    );

    // Update total points
    let mut score = score;
    score.team_a_points += result.team_a_round_points;
    score.team_b_points += result.team_b_round_points;
    ctx.db.score().room_id().update(score.clone());

    // Check game over (10 points)
    if score.team_a_points >= 10 || score.team_b_points >= 10 {
        room.status = GameStatus::GameOver;
        ctx.db.room().id().update(room);
        log::info!(
            "Game over in room {}! Team A: {}, Team B: {}",
            room_id, score.team_a_points, score.team_b_points
        );
        return Ok(());
    }

    // Rotate dealer clockwise, set up next round
    room.dealer_index = (room.dealer_index + 1) % 4;
    room.trump_selector_index = (room.dealer_index + 1) % 4;
    room.current_turn_index = room.dealer_index;
    room.status = GameStatus::WaitingForDeal;
    room.trump_suit = None;
    room.hand_number = 0;
    room.previous_round_tied = result.is_tie;
    room.deck = Vec::new();
    ctx.db.room().id().update(room.clone());

    // In single-player mode, if dealer is a bot, auto-deal
    if room.mode == RoomMode::SinglePlayer {
        if let Some(dealer) = get_player_by_seat(ctx, room_id, room.dealer_index) {
            if dealer.is_bot {
                do_auto_deal(ctx, room_id)?;
            }
        }
    }

    Ok(())
}

/// Auto-deal for bot dealers in single-player mode
fn do_auto_deal(ctx: &ReducerContext, room_id: u64) -> Result<(), String> {
    let room = ctx.db.room().id().find(room_id)
        .ok_or_else(|| "Room not found".to_string())?;

    if room.status != GameStatus::WaitingForDeal {
        return Ok(());
    }

    // Generate and shuffle deck
    let mut new_deck = deck::generate_deck();
    let mut rng = make_rng(ctx);
    deck::shuffle_deck(&mut new_deck, &mut rng);

    // Deal first 4 cards to each player
    let mut players = get_players_in_room(ctx, room_id);
    players.sort_by_key(|p| p.seat_index);

    for player in &players {
        let cards = deck::deal_cards(&mut new_deck, 4);
        if let Some(existing) = ctx.db.player_hand().player_id().find(player.id) {
            ctx.db.player_hand().player_id().delete(existing.player_id);
        }
        ctx.db.player_hand().insert(PlayerHand {
            player_id: player.id,
            cards,
        });
    }

    let trump_selector_index = (room.dealer_index + 1) % 4;
    let mut room = room;
    room.deck = new_deck;
    room.status = GameStatus::WaitingForTrump;
    room.trump_selector_index = trump_selector_index;
    room.current_turn_index = trump_selector_index;
    room.hand_number = 1;
    room.trump_suit = None;
    ctx.db.room().id().update(room);

    // If trump selector is a bot, auto-select
    if let Some(selector) = get_player_by_seat(ctx, room_id, trump_selector_index) {
        if selector.is_bot {
            if let Some(hand) = ctx.db.player_hand().player_id().find(selector.id) {
                let trump = bot_ai::select_trump(&hand.cards);
                do_select_trump(ctx, room_id, trump)?;
            }
        }
    }

    Ok(())
}

// ─── Connection lifecycle ──────────────────────────────────────────────────────

#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender());
}

#[reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    // Mark player as disconnected in all rooms
    let players: Vec<Player> = ctx.db.player().iter()
        .filter(|p| p.identity == ctx.sender() && !p.is_bot)
        .collect();

    for player in players {
        let mut player = player;
        player.is_connected = false;
        ctx.db.player().id().update(player);
    }

    log::info!("Client disconnected: {:?}", ctx.sender());
}
