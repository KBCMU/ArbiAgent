"use client";

import {
    MatchedEvent,
    OutcomeOdds,
    getEventOutcomes,
    findOutcome,
} from "@/lib/api";
import { cn } from "@/lib/utils";

interface EventRowProps {
    event: MatchedEvent;
    rowIndex: number;
    showSportCol?: boolean;
    showEventCol?: boolean;
}

const DATE_REGEX = /\((\d{4}-\d{2}-\d{2})\)/;
const EVENT_ID_DATE_REGEX = /(\d{4}-\d{2}-\d{2})$/;

function extractDate(event: MatchedEvent): string {
    if (event.game_start_time) {
        const d = new Date(event.game_start_time);
        const month = d.getMonth() + 1;
        const day = d.getDate();
        let hours = d.getHours();
        const minutes = d.getMinutes();
        const ampm = hours >= 12 ? "PM" : "AM";
        hours = hours % 12 || 12;
        const mm = minutes.toString().padStart(2, "0");
        return `${month}/${day} · ${hours}:${mm}${ampm}`;
    }

    const match = event.event_title.match(DATE_REGEX);
    if (match) {
        const d = new Date(match[1] + "T00:00:00");
        return `${d.getMonth() + 1}/${d.getDate()}`;
    }

    // Sports IDs are canonicalized as `sport-team-team-YYYY-MM-DD`.
    // Prefer this over created_at when no explicit start time is available.
    const idDateMatch = event.id.match(EVENT_ID_DATE_REGEX);
    if (idDateMatch) {
        const d = new Date(idDateMatch[1] + "T00:00:00");
        return `${d.getMonth() + 1}/${d.getDate()}`;
    }

    if (event.created_at) {
        const d = new Date(event.created_at);
        const month = d.getMonth() + 1;
        const day = d.getDate();
        return `${month}/${day}`;
    }

    return "";
}

const SPORT_META: Record<string, { icon: string; label: string; color: string }> = {
    cbb: { icon: "🏀", label: "CBB", color: "text-orange-600 dark:text-orange-400" },
    nba: { icon: "🏀", label: "NBA", color: "text-orange-600 dark:text-orange-400" },
    nfl: { icon: "🏈", label: "NFL", color: "text-green-600 dark:text-green-400" },
    cfb: { icon: "🏈", label: "CFB", color: "text-green-600 dark:text-green-400" },
    mlb: { icon: "⚾", label: "MLB", color: "text-red-600 dark:text-red-400" },
    nhl: { icon: "🏒", label: "NHL", color: "text-sky-600 dark:text-sky-400" },
    mls: { icon: "⚽", label: "MLS", color: "text-emerald-600 dark:text-emerald-400" },
    epl: { icon: "⚽", label: "EPL", color: "text-purple-600 dark:text-purple-400" },
    soccer: { icon: "⚽", label: "Soccer", color: "text-emerald-600 dark:text-emerald-400" },
    tennis: { icon: "🎾", label: "Tennis", color: "text-lime-600 dark:text-lime-400" },
    golf: { icon: "⛳", label: "Golf", color: "text-green-600 dark:text-green-400" },
    mma: { icon: "🥊", label: "MMA", color: "text-red-600 dark:text-red-400" },
    boxing: { icon: "🥊", label: "Boxing", color: "text-red-600 dark:text-red-400" },
    politics: { icon: "🏛️", label: "Politics", color: "text-blue-600 dark:text-blue-400" },
    economics: { icon: "📊", label: "Econ", color: "text-amber-600 dark:text-amber-400" },
    crypto: { icon: "₿", label: "Crypto", color: "text-yellow-600 dark:text-yellow-400" },
    science: { icon: "🔬", label: "Science", color: "text-indigo-600 dark:text-indigo-400" },
    culture: { icon: "🎭", label: "Culture", color: "text-pink-600 dark:text-pink-400" },
    world: { icon: "🌍", label: "World", color: "text-teal-600 dark:text-teal-400" },
};

const COL_BORDER = "border-r border-gray-200/50 dark:border-white/[0.06]";

type Platform = "kalshi" | "polymarket";

const PLATFORM_COLOR: Record<Platform, string> = {
    kalshi: "text-emerald-600 dark:text-emerald-400",
    polymarket: "text-blue-600 dark:text-blue-400",
};

function SportLabel({ sport }: { sport: string }) {
    const meta = SPORT_META[sport.toLowerCase()];
    const icon = meta?.icon ?? "📌";
    const label = meta?.label ?? sport.toUpperCase();
    const color = meta?.color ?? "text-gray-500 dark:text-white/50";

    return (
        <span className={cn("inline-flex items-center gap-1.5", color)}>
            <span className="text-sm leading-none">{icon}</span>
            <span
                className="text-[10px] font-extrabold uppercase tracking-widest"
                style={{ fontFamily: "var(--font-mono), monospace" }}
            >
                {label}
            </span>
        </span>
    );
}

function formatOdds(price: number): string {
    if (price >= 100) return "$1.00";
    if (price <= 0) return "—";
    return `${price}¢`;
}

function PriceCell({
    outcomes,
    outcomeName,
    available,
    platform,
}: {
    outcomes: OutcomeOdds[];
    outcomeName: string;
    available: boolean;
    platform: Platform;
}) {
    if (!available) {
        return (
            <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
                <span className="text-xs text-gray-300 dark:text-white/15">—</span>
            </td>
        );
    }

    const match = findOutcome(outcomes, outcomeName);

    if (!match) {
        return (
            <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
                <span className="text-xs text-gray-300 dark:text-white/15">—</span>
            </td>
        );
    }

    return (
        <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
            <span
                className={cn(
                    "text-sm font-bold tabular-nums",
                    PLATFORM_COLOR[platform],
                )}
                style={{ fontFamily: "var(--font-mono), monospace" }}
            >
                {formatOdds(match.yes_price)}
            </span>
        </td>
    );
}

function AvgOddsCell({
    kalshiOutcomes,
    polyOutcomes,
    outcomeName,
    hasKalshi,
    hasPoly,
}: {
    kalshiOutcomes: OutcomeOdds[];
    polyOutcomes: OutcomeOdds[];
    outcomeName: string;
    hasKalshi: boolean;
    hasPoly: boolean;
}) {
    const kalshiMatch = hasKalshi ? findOutcome(kalshiOutcomes, outcomeName) : undefined;
    const polyMatch = hasPoly ? findOutcome(polyOutcomes, outcomeName) : undefined;

    const prices: number[] = [];
    if (kalshiMatch && kalshiMatch.yes_price > 0) prices.push(kalshiMatch.yes_price);
    if (polyMatch && polyMatch.yes_price > 0) prices.push(polyMatch.yes_price);

    if (prices.length === 0) {
        return (
            <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
                <span className="text-xs text-gray-300 dark:text-white/15">—</span>
            </td>
        );
    }

    const avg = Math.round(prices.reduce((a, b) => a + b, 0) / prices.length);

    return (
        <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
            <span
                className="text-sm font-bold tabular-nums text-gray-900 dark:text-white"
                style={{ fontFamily: "var(--font-mono), monospace" }}
            >
                {formatOdds(avg)}
            </span>
        </td>
    );
}

function BestOddsCell({
    kalshiOutcomes,
    polyOutcomes,
    outcomeName,
    hasKalshi,
    hasPoly,
}: {
    kalshiOutcomes: OutcomeOdds[];
    polyOutcomes: OutcomeOdds[];
    outcomeName: string;
    hasKalshi: boolean;
    hasPoly: boolean;
}) {
    const kalshiMatch = hasKalshi ? findOutcome(kalshiOutcomes, outcomeName) : undefined;
    const polyMatch = hasPoly ? findOutcome(polyOutcomes, outcomeName) : undefined;

    const kalshiYes = kalshiMatch?.yes_price ?? -1;
    const polyYes = polyMatch?.yes_price ?? -1;

    if (kalshiYes <= 0 && polyYes <= 0) {
        return (
            <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
                <span className="text-xs text-gray-300 dark:text-white/15">—</span>
            </td>
        );
    }

    const bestYes = Math.max(kalshiYes, polyYes);
    const bestYesPlatform: Platform = kalshiYes >= polyYes ? "kalshi" : "polymarket";

    return (
        <td className={cn("px-3 py-2.5 text-center", COL_BORDER)}>
            <span
                className={cn(
                    "text-sm font-bold tabular-nums",
                    PLATFORM_COLOR[bestYesPlatform],
                )}
                style={{ fontFamily: "var(--font-mono), monospace" }}
                title={`Best — ${bestYesPlatform}`}
            >
                {formatOdds(bestYes)}
            </span>
        </td>
    );
}

export function EventRow({ event, rowIndex, showSportCol = true, showEventCol = false }: EventRowProps) {
    const outcomes = getEventOutcomes(event);
    const hasKalshi = event.kalshi_outcomes.length > 0;
    const hasPoly = event.polymarket_outcomes.length > 0;
    const dateStr = extractDate(event);
    const isEven = rowIndex % 2 === 0;

    return (
        <>
            {outcomes.map((outcome, idx) => (
                <tr
                    key={outcome}
                    className={cn(
                        "group transition-colors duration-150",
                        isEven
                            ? "bg-white dark:bg-transparent"
                            : "bg-gray-50/60 dark:bg-white/[0.015]",
                        "hover:bg-blue-50/50 dark:hover:bg-blue-500/[0.04]",
                        idx === outcomes.length - 1 && "border-b border-gray-200/70 dark:border-white/[0.07]",
                    )}
                >
                    {idx === 0 && (
                        <td
                            className={cn("py-2.5 pl-4 pr-2 align-middle", COL_BORDER)}
                            rowSpan={outcomes.length}
                        >
                            {dateStr ? (
                                <span
                                    className="whitespace-nowrap text-[11px] font-semibold tracking-tight text-gray-500 dark:text-white/40"
                                    style={{ fontFamily: "var(--font-mono), monospace" }}
                                >
                                    {dateStr}
                                </span>
                            ) : (
                                <span className="text-[11px] text-gray-300 dark:text-white/15">—</span>
                            )}
                        </td>
                    )}

                    {showSportCol && idx === 0 && (
                        <td
                            className={cn("px-2 py-2.5 align-middle", COL_BORDER)}
                            rowSpan={outcomes.length}
                        >
                            <SportLabel sport={event.sport} />
                        </td>
                    )}

                    {showEventCol && idx === 0 && (
                        <td
                            className={cn("px-3 py-2.5 align-middle", COL_BORDER)}
                            rowSpan={outcomes.length}
                        >
                            <span className="text-[12px] font-semibold leading-tight text-gray-700 dark:text-white/70 line-clamp-2">
                                {event.event_title}
                            </span>
                        </td>
                    )}

                    <td className={cn("px-3 py-2.5", COL_BORDER)}>
                        <span className="text-[13px] font-bold tracking-tight text-gray-800 dark:text-white/90">
                            {outcome}
                        </span>
                    </td>

                    <AvgOddsCell
                        kalshiOutcomes={event.kalshi_outcomes}
                        polyOutcomes={event.polymarket_outcomes}
                        outcomeName={outcome}
                        hasKalshi={hasKalshi}
                        hasPoly={hasPoly}
                    />

                    <BestOddsCell
                        kalshiOutcomes={event.kalshi_outcomes}
                        polyOutcomes={event.polymarket_outcomes}
                        outcomeName={outcome}
                        hasKalshi={hasKalshi}
                        hasPoly={hasPoly}
                    />

                    <PriceCell
                        outcomes={event.kalshi_outcomes}
                        outcomeName={outcome}
                        available={hasKalshi}
                        platform="kalshi"
                    />

                    <PriceCell
                        outcomes={event.polymarket_outcomes}
                        outcomeName={outcome}
                        available={hasPoly}
                        platform="polymarket"
                    />
                </tr>
            ))}
        </>
    );
}
