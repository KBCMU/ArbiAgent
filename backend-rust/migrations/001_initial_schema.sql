-- ArbiAgent Rust Backend - Initial Schema
-- Run against the Supabase project: zfvehrpqwqcdasoaocdx

-- Canonical events (discovered via DomeAPI matching)
CREATE TABLE IF NOT EXISTS canonical_events (
    id TEXT PRIMARY KEY,                    -- e.g. "nfl-ari-den-2025-08-16"
    sport TEXT NOT NULL,
    event_title TEXT NOT NULL,
    game_start_time TIMESTAMPTZ,
    status TEXT DEFAULT 'open',             -- open, closed, resolved
    kalshi_event_ticker TEXT,
    kalshi_market_tickers TEXT[],
    polymarket_market_slug TEXT,
    polymarket_token_ids TEXT[],
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Real-time odds snapshots (written periodically for charting)
CREATE TABLE IF NOT EXISTS odds_snapshots (
    id BIGSERIAL PRIMARY KEY,
    canonical_event_id TEXT REFERENCES canonical_events(id),
    platform TEXT NOT NULL,                 -- 'kalshi' or 'polymarket'
    outcome_name TEXT NOT NULL,             -- 'ARI', 'DEN', etc.
    yes_price NUMERIC(5,4),                 -- 0.0000 to 1.0000
    no_price NUMERIC(5,4),
    best_bid NUMERIC(5,4),
    best_ask NUMERIC(5,4),
    bid_size NUMERIC(12,2),
    ask_size NUMERIC(12,2),
    recorded_at TIMESTAMPTZ DEFAULT NOW()
);

-- Detected arbitrage opportunities
CREATE TABLE IF NOT EXISTS arbitrage_opportunities (
    id BIGSERIAL PRIMARY KEY,
    canonical_event_id TEXT REFERENCES canonical_events(id),
    outcome_name TEXT NOT NULL,
    buy_platform TEXT NOT NULL,
    buy_price NUMERIC(5,4) NOT NULL,
    sell_platform TEXT NOT NULL,
    sell_price NUMERIC(5,4) NOT NULL,
    margin_pct NUMERIC(6,3) NOT NULL,
    max_executable_size NUMERIC(12,2),
    estimated_fees NUMERIC(8,2),
    estimated_net_profit NUMERIC(8,2),
    score INTEGER,
    detected_at TIMESTAMPTZ DEFAULT NOW(),
    closed_at TIMESTAMPTZ,                  -- when the arb window closed
    duration_ms INTEGER                     -- how long the window lasted
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_odds_event_time ON odds_snapshots(canonical_event_id, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_arb_detected ON arbitrage_opportunities(detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_arb_event ON arbitrage_opportunities(canonical_event_id);
CREATE INDEX IF NOT EXISTS idx_events_sport ON canonical_events(sport, status);
CREATE INDEX IF NOT EXISTS idx_events_start ON canonical_events(game_start_time);

-- Enable RLS (public reads, service-role writes)
ALTER TABLE canonical_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE odds_snapshots ENABLE ROW LEVEL SECURITY;
ALTER TABLE arbitrage_opportunities ENABLE ROW LEVEL SECURITY;

-- Public read policies
CREATE POLICY "Allow public read on canonical_events"
    ON canonical_events FOR SELECT
    USING (true);

CREATE POLICY "Allow public read on odds_snapshots"
    ON odds_snapshots FOR SELECT
    USING (true);

CREATE POLICY "Allow public read on arbitrage_opportunities"
    ON arbitrage_opportunities FOR SELECT
    USING (true);

-- Service role write policies
CREATE POLICY "Allow service role insert on canonical_events"
    ON canonical_events FOR INSERT
    WITH CHECK (true);

CREATE POLICY "Allow service role update on canonical_events"
    ON canonical_events FOR UPDATE
    USING (true);

CREATE POLICY "Allow service role insert on odds_snapshots"
    ON odds_snapshots FOR INSERT
    WITH CHECK (true);

CREATE POLICY "Allow service role insert on arbitrage_opportunities"
    ON arbitrage_opportunities FOR INSERT
    WITH CHECK (true);
