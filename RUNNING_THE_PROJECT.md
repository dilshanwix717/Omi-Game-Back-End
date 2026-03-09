# Running the Omi Card Game Project

## Project Overview

A real-time, browser-based Omi card game with multiplayer and singleplayer modes. Built with Next.js (TypeScript) frontend and SpacetimeDB (Rust) backend communicating via WebSockets. Features server-authoritative game logic, bot AI with difficulty levels, Framer Motion + GSAP animations, and a modern glassmorphism UI.

## Prerequisites

- **Node.js** LTS (v18+ recommended)
- **pnpm** (`npm install -g pnpm`)
- **Rust** (via [rustup](https://rustup.rs/)) — only needed for SpacetimeDB module compilation
- **SpacetimeDB CLI** ([installation guide](https://spacetimedb.com/docs/getting-started))
- **Git**

## Project Structure

```
omi_game/
├── client/                # Next.js frontend (TypeScript + TailwindCSS)
│   ├── app/               # App Router pages (/, /game, /rules)
│   │   ├── page.tsx       # Landing page with mode selection
│   │   ├── game/page.tsx  # Main game table
│   │   └── rules/page.tsx # Rules, scoring, tutorial
│   ├── components/        # React components
│   │   ├── GameTable.tsx   # Main 4-player game table layout
│   │   ├── Card.tsx        # Card with animations & validation
│   │   ├── PlayerHand.tsx  # Player's sorted hand
│   │   ├── Lobby.tsx       # Room lobby with code sharing
│   │   ├── ModeSelector.tsx # Game mode picker
│   │   ├── TrumpSelector.tsx # Trump suit picker
│   │   ├── ScoreBoard.tsx   # Team scores display
│   │   ├── HandHistory.tsx  # Previous hands panel
│   │   ├── AnimationOverlay.tsx # Shuffle/deal/collect animations
│   │   ├── RoundSummary.tsx    # Round end overlay
│   │   ├── ConnectionStatus.tsx # WebSocket status indicator
│   │   ├── Toast.tsx        # Notification toasts
│   │   └── InfoButton.tsx   # Link to rules
│   ├── hooks/             # Custom React hooks
│   │   ├── useSpacetimeDB.ts  # WebSocket connection + reconnect
│   │   ├── useGameState.ts    # Game state management
│   │   └── useAnimations.ts   # GSAP animation orchestration
│   ├── lib/               # Utilities, types, constants
│   │   ├── types.ts        # TypeScript game types
│   │   ├── constants.ts    # Suits, ranks, scoring rules
│   │   ├── spacetimedb.ts  # SpacetimeDB config
│   │   ├── sounds.ts       # Web Audio API sound effects
│   │   ├── accessibility.tsx # Colorblind mode, reduced motion
│   │   └── GameContext.tsx  # React context provider
│   └── public/            # Static assets
├── server/                # SpacetimeDB Rust module
│   ├── src/
│   │   ├── lib.rs          # Module entry
│   │   ├── types.rs        # Suit, Rank, Card, enums
│   │   ├── deck.rs         # 32-card deck, Fisher-Yates shuffle
│   │   ├── game_logic.rs   # Hand resolution, move validation, scoring
│   │   ├── bot_ai.rs       # Bot trump selection & card play AI
│   │   └── reducers.rs     # SpacetimeDB tables & reducers
│   └── Cargo.toml
├── RUNNING_THE_PROJECT.md
└── README.md
```

## SpacetimeDB Setup

### 1. Install SpacetimeDB CLI

```bash
# macOS / Linux
curl -sSf https://install.spacetimedb.com | sh

# Verify installation
spacetime version
```

### 2. Start SpacetimeDB Local Server

```bash
spacetime start
```

### 3. Publish the Server Module

```bash
cd server
spacetime publish omi-card-game --project-path .
```

### 4. Generate TypeScript Client Bindings

```bash
spacetime generate --lang typescript --out-dir ../client/lib/generated --bin-path target/wasm32-unknown-unknown/release/omi_card_game.wasm
```

## Running the Backend

```bash
# Start SpacetimeDB (if not already running)
spacetime start

# Publish/update the module
cd server
spacetime publish omi-card-game --project-path .
```

## Running the Frontend

```bash
cd client

# Install dependencies
pnpm install

# Start development server
pnpm dev
```

The frontend will be available at **http://localhost:3000**.

## Environment Variables

Create `client/.env.local` (already created with defaults):

```env
NEXT_PUBLIC_SPACETIMEDB_URL=ws://localhost:3000
NEXT_PUBLIC_GAME_NAME=omi-card-game
```

## Development Workflow

1. **Backend changes**: Edit Rust files in `server/src/`, then run `spacetime publish omi-card-game --project-path .`
2. **Frontend changes**: Edit files in `client/`, hot-reload is automatic with `pnpm dev`
3. **After schema changes**: Regenerate TypeScript bindings with `spacetime generate --lang typescript --out-dir ../client/lib/generated --bin-path target/wasm32-unknown-unknown/release/omi_card_game.wasm`

## Connection Flow

1. Frontend initializes SpacetimeDB client with WebSocket URL
2. Client connects and subscribes to room state tables
3. Client listens for real-time state updates (players, hands, scores)
4. Player actions call server reducers (create_room, play_card, etc.)
5. Server validates and updates state; changes broadcast to all subscribers

## Troubleshooting

| Issue | Solution |
|---|---|
| `spacetime: command not found` | Reinstall SpacetimeDB CLI, ensure it's in your PATH |
| WebSocket connection fails | Verify SpacetimeDB is running (`spacetime start`) |
| Module publish fails | Check Rust compilation errors with `cargo build` in `server/` |
| Frontend build errors | Run `pnpm install` to ensure dependencies are installed |
| Stale TypeScript types | Regenerate bindings: `spacetime generate --lang typescript --out-dir ../client/lib/generated --bin-path target/wasm32-unknown-unknown/release/omi_card_game.wasm` |
| Port 3000 in use | Change port in `.env.local` or kill existing process |

## Deployment Notes

- **Frontend**: Deploy to Vercel (`vercel deploy` from `client/`)
- **Backend**: Deploy to SpacetimeDB Cloud (`spacetime publish --server cloud`)
- Update `NEXT_PUBLIC_SPACETIMEDB_URL` to point to the production SpacetimeDB instance

## Singleplayer / Demo Mode

The frontend includes a built-in singleplayer demo mode that works **without** SpacetimeDB. When you select "Play vs Computer" from the landing page, the game runs entirely client-side with simulated bot opponents using `setTimeout` delays. This is useful for:

- Testing the UI without setting up the backend
- Playing offline
- Demonstrating the game

## Game Features

| Feature | Status |
|---|---|
| 4-player Omi card game | ✅ |
| Multiplayer via WebSocket | ✅ (requires SpacetimeDB) |
| Singleplayer vs 3 bots | ✅ (client-side demo) |
| Bot AI (easy/medium/hard) | ✅ |
| Private rooms (code + link) | ✅ |
| Trump selection | ✅ |
| Move validation (server-authoritative) | ✅ |
| Scoring (kapothi, ties, etc.) | ✅ |
| Framer Motion animations | ✅ |
| GSAP animation sequences | ✅ |
| Sound effects (Web Audio API) | ✅ |
| Colorblind mode | ✅ |
| Responsive design | ✅ |
| WebSocket auto-reconnect | ✅ |
| Spectator mode | ✅ |
| Rules page with tutorial | ✅ |
