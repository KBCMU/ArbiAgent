# ArbiAgent

Prediction market arbitrage detection platform.

## Features

- Real-time market data from Polymarket and Kalshi via DomeAPI
- Clean, data-dense dashboard interface
- Arbitrage opportunity detection with fee-aware margin calculation
- Rust backend with async ingestion, in-memory cache, and Supabase persistence
- Next.js frontend with Supabase Auth
- WebSocket push for live price and arb updates

## Tech Stack

### Backend (Rust)
- **Axum** - HTTP + WebSocket framework
- **Tokio** - Async runtime
- **sqlx** - Postgres (Supabase) driver
- **DashMap** - Lock-free concurrent cache
- **DomeAPI** - Unified prediction market data source
- **reqwest** - Async HTTP client
- **tokio-tungstenite** - WebSocket client (Kalshi/Polymarket ingesters)

### Frontend
- **Next.js 16** - React framework with App Router
- **React 19** - UI library
- **TypeScript** - Type-safe development
- **Tailwind CSS v4** - Utility-first styling
- **Supabase Auth** - Authentication (email/password)
- **Framer Motion** - Animations
- **Lucide React** - Icon library

### Infrastructure
- **Supabase** - Postgres database + Auth + Row Level Security

## Getting Started

### Prerequisites
- Rust 1.70+ (with cargo)
- Node.js 18+
- DomeAPI key (get from [domeapi.io](https://domeapi.io))
- Supabase project (for database and auth)

### Backend Setup

1. Configure environment:
```bash
cd backend-rust
cp .env.example .env
# Edit .env with your DOME_API_KEY, Supabase credentials, etc.
```

2. Run the database migration against your Supabase project:
```sql
-- Apply backend-rust/migrations/001_initial_schema.sql
```

3. Build and run:
```bash
cd backend-rust
cargo run
```

Backend will be available at `http://localhost:8080`

### Frontend Setup

1. Configure environment:
```bash
cd frontend
cp .env.local.example .env.local
# Edit with your NEXT_PUBLIC_SUPABASE_URL and NEXT_PUBLIC_SUPABASE_ANON_KEY
```

2. Install dependencies:
```bash
cd frontend
npm install
```

3. Run the dev server:
```bash
npm run dev
```

Frontend will be available at `http://localhost:3000`

## Project Structure

```
ArbiAgent/
├── backend-rust/
│   ├── src/
│   │   ├── main.rs              # App bootstrap, spawns all background loops
│   │   ├── config.rs            # Environment configuration
│   │   ├── api/                 # HTTP routes + WebSocket server
│   │   ├── ingestion/           # DomeAPI poller, Kalshi WS, Polymarket WS
│   │   ├── models/              # Data types (events, arbs, platform, WS messages)
│   │   ├── processing/          # Arbitrage detection engine
│   │   └── storage/             # In-memory cache + Supabase persistence
│   ├── migrations/              # SQL schema
│   ├── Cargo.toml               # Rust dependencies
│   └── .env                     # Backend config (not committed)
├── frontend/
│   ├── app/                     # Next.js pages (landing, markets, auth)
│   ├── components/              # React components (Sidebar, Header, EventTable, etc.)
│   ├── lib/                     # API client, utilities, Supabase clients
│   └── .env.local               # Frontend config (not committed)
├── AGENTS.md                    # AI agent operating instructions
└── .gitignore
```

## API Endpoints (Rust Backend)

- `GET /` - Root status
- `GET /api/v2/health` - Health check with cache stats
- `GET /api/v2/events` - List matched events (filters: `sport`, `status`, `limit`)
- `GET /api/v2/events/{id}` - Get specific event with odds
- `GET /api/v2/events/{id}/odds-history` - Historical odds (query: `hours`)
- `GET /api/v2/arbitrage` - List active arbitrage opportunities
- `GET /api/v2/arbitrage/history` - Historical arb records
- `GET /api/v2/arbitrage/stats` - Aggregate arb statistics
- `GET /api/v2/sports/{sport}/today` - Today's events for a sport
- `WS /ws/arb-feed` - Real-time price + arb push

## How It Works

1. **Event Discovery** - DomeAPI identifies matching events across Kalshi and Polymarket
2. **Price Ingestion** - REST polling (Phase 1) or WebSocket streams (Phase 2) feed live prices into an in-memory cache
3. **Arb Detection** - Every 2 seconds, the engine compares cross-platform prices, accounting for platform-specific fees (Kalshi ~3%, Polymarket 0%)
4. **Alerting** - Detected opportunities are broadcast to frontend clients via WebSocket and persisted to Supabase
5. **Snapshot Writing** - Odds snapshots are periodically written to Supabase for historical charting

## License

MIT
