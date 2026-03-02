// ─── Frontend Types ──────────────────────────────────────────────────

/** Price data for a single outcome on a single platform (cents, 0-100). */
export interface OutcomeOdds {
    outcome: string;
    yes_price: number;
    no_price: number;
}

/** Summary arb info embedded on an event (from /api/v2/events). */
export interface ArbOpportunity {
    outcome: string;
    buy_platform: string;
    buy_price: number;
    sell_platform: string;
    sell_price: number;
    margin_pct: number;
    score: number;
}

/** A matched event with per-platform outcomes (from /api/v2/events). */
export interface MatchedEvent {
    id: string;
    event_title: string;
    sport: string;
    game_start_time: string | null;
    created_at: string;
    status: string;
    kalshi_outcomes: OutcomeOdds[];
    polymarket_outcomes: OutcomeOdds[];
    arb_opportunity: ArbOpportunity | null;
}

/** Full arb opportunity from the /api/v2/arbitrage endpoint. */
export interface ActiveArb {
    canonical_event_id: string;
    event_title: string;
    sport: string;
    outcome: string;
    buy_platform: string;
    buy_price: number;
    sell_platform: string;
    sell_price: number;
    margin_pct: number;
    max_executable_size: number | null;
    estimated_fees: number | null;
    estimated_net_profit: number | null;
    score: number;
    detected_at: string;
}

// ─── Raw types from the Rust backend (private) ──────────────────────

interface RustOutcomePrice {
    yes_price: number;
    no_price: number;
}

interface RustPlatformOdds {
    outcomes: Record<string, RustOutcomePrice>;
    updated_at?: string;
}

interface RustArbOpportunity {
    outcome: string;
    buy_platform: string;
    buy_price: number;
    sell_platform: string;
    sell_price: number;
    margin_pct: number;
    score: number;
}

interface RustEventResponse {
    id: string;
    sport: string;
    event_title: string;
    game_start_time?: string | null;
    created_at?: string;
    status: string;
    kalshi?: RustPlatformOdds | null;
    polymarket?: RustPlatformOdds | null;
    arb_opportunity?: RustArbOpportunity | null;
}

interface RustEventsListResponse {
    events: RustEventResponse[];
    total: number;
}

interface RustArbListResponse {
    opportunities: ActiveArb[];
    total: number;
}

// ─── Config ─────────────────────────────────────────────────────────

const API_BASE_URL =
    process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

// ─── Transforms ─────────────────────────────────────────────────────

/** Convert a Rust platform odds block into OutcomeOdds[] in cents. */
function transformPlatformOdds(
    platform: RustPlatformOdds | null | undefined,
): OutcomeOdds[] {
    if (!platform) return [];

    return Object.entries(platform.outcomes).map(([outcome, prices]) => ({
        outcome,
        yes_price: Math.round(prices.yes_price * 100),
        no_price: Math.round(prices.no_price * 100),
    }));
}

/** Transform a raw Rust event response into a frontend MatchedEvent. */
function transformEvent(rust: RustEventResponse): MatchedEvent {
    return {
        id: rust.id,
        event_title: rust.event_title,
        sport: rust.sport,
        game_start_time: rust.game_start_time ?? null,
        created_at: rust.created_at ?? new Date().toISOString(),
        status: rust.status,
        kalshi_outcomes: transformPlatformOdds(rust.kalshi),
        polymarket_outcomes: transformPlatformOdds(rust.polymarket),
        arb_opportunity: rust.arb_opportunity ?? null,
    };
}

// ─── API Functions ──────────────────────────────────────────────────

export async function fetchMatchedEvents(params?: {
    limit?: number;
    status?: string;
    sport?: string;
}): Promise<{ events: MatchedEvent[]; total: number }> {
    const queryParams = new URLSearchParams();
    if (params?.limit) queryParams.append("limit", params.limit.toString());
    if (params?.status) queryParams.append("status", params.status);
    if (params?.sport) queryParams.append("sport", params.sport);

    const response = await fetch(
        `${API_BASE_URL}/api/v2/events?${queryParams}`,
    );

    if (!response.ok) {
        throw new Error("Failed to fetch events");
    }

    const data: RustEventsListResponse = await response.json();

    return {
        events: data.events.map(transformEvent),
        total: data.total,
    };
}

export async function fetchActiveArbs(): Promise<ActiveArb[]> {
    const response = await fetch(`${API_BASE_URL}/api/v2/arbitrage`);
    if (!response.ok) return [];

    const data: RustArbListResponse = await response.json();
    return data.opportunities ?? [];
}

export async function fetchEventDetail(
    eventId: string,
): Promise<MatchedEvent | null> {
    const response = await fetch(
        `${API_BASE_URL}/api/v2/events/${encodeURIComponent(eventId)}`,
    );

    if (!response.ok) return null;

    const data: RustEventResponse = await response.json();
    if ("error" in data) return null;

    return transformEvent(data);
}

export async function fetchArbHistory(params?: {
    event_id?: string;
    limit?: number;
}): Promise<ActiveArb[]> {
    const queryParams = new URLSearchParams();
    if (params?.event_id) queryParams.append("event_id", params.event_id);
    if (params?.limit) queryParams.append("limit", params.limit.toString());

    const response = await fetch(
        `${API_BASE_URL}/api/v2/arbitrage/history?${queryParams}`,
    );

    if (!response.ok) return [];

    const data = await response.json();
    return data.history ?? [];
}

export async function fetchArbStats(): Promise<Record<string, number>> {
    const response = await fetch(`${API_BASE_URL}/api/v2/arbitrage/stats`);
    if (!response.ok) return {};
    return response.json();
}

export async function fetchOddsHistory(
    eventId: string,
    hours = 24,
): Promise<Record<string, unknown>[]> {
    const response = await fetch(
        `${API_BASE_URL}/api/v2/events/${encodeURIComponent(eventId)}/odds-history?hours=${hours}`,
    );

    if (!response.ok) return [];

    const data = await response.json();
    return data.snapshots ?? [];
}

export async function fetchHealthCheck(): Promise<{
    status: string;
    tracked_events: number;
    dual_platform_events: number;
    active_arbs: number;
}> {
    const response = await fetch(`${API_BASE_URL}/api/v2/health`);
    if (!response.ok) throw new Error("Health check failed");
    return response.json();
}

// ─── Helpers ────────────────────────────────────────────────────────

const GENERIC_OUTCOME_RE = /^OUTCOME_\d+$/;

/** Get all unique outcome names across both platforms for an event.
 *  Filters out generic "OUTCOME_N" entries when real named outcomes exist. */
export function getEventOutcomes(event: MatchedEvent): string[] {
    const outcomes = new Set<string>();
    for (const o of event.kalshi_outcomes) outcomes.add(o.outcome);
    for (const o of event.polymarket_outcomes) outcomes.add(o.outcome);

    const all = Array.from(outcomes);
    const named = all.filter((n) => !GENERIC_OUTCOME_RE.test(n));
    return named.length > 0 ? named : all;
}

/** Find an outcome's odds within a platform's outcome list. */
export function findOutcome(
    outcomes: OutcomeOdds[],
    name: string,
): OutcomeOdds | undefined {
    return outcomes.find((o) => o.outcome === name);
}
