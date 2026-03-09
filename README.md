# Omi Card Game - Backend

A SpacetimeDB backend for the Omi card game, a popular Sri Lankan trick-taking card game.

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [SpacetimeDB CLI](https://spacetimedb.com/install)

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/YOUR_USERNAME/Omi-Game-Back-End.git
cd Omi-Game-Back-End
```

### 2. Build the Project

```bash
cargo build --target wasm32-unknown-unknown --release
```

> **Note:** The `rust-toolchain.toml` file automatically installs the required WebAssembly target when you run any cargo command.

### 3. Run with SpacetimeDB

```bash
# Start local SpacetimeDB server
spacetime start

# Publish the module
spacetime publish omi-card-game
```

## Project Structure

```
src/
├── lib.rs          # Module entry point and exports
├── types.rs        # Core types (Card, Suit, Rank, Player, Game)
├── deck.rs         # Deck creation and shuffling
├── game_logic.rs   # Game rules and scoring
├── reducers.rs     # SpacetimeDB reducers (API endpoints)
└── bot_ai.rs       # AI bot logic
```

## Game Rules

Omi is a 4-player trick-taking card game played in teams:

- **Teams:** 2 teams of 2 players (partners sit opposite)
- **Cards:** Standard 32-card deck (7-Ace in each suit)
- **Trump:** One player selects the trump suit each round
- **Objective:** Win more tricks than the opposing team
- **Scoring:**
  - Win by taking 5+ tricks: 1 point (2 if previous round tied)
  - Kapothi (win all 8 tricks): 2 points (3 if by non-trump-selecting team)
  - First team to 10 points wins

## Development

```bash
# Check for errors
cargo check

# Run tests
cargo test

# Build in debug mode
cargo build --target wasm32-unknown-unknown
```

## License

MIT
