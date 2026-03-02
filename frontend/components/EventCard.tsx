"use client";

import {
    MatchedEvent,
    OutcomeOdds,
    getEventOutcomes,
    findOutcome,
} from "@/lib/api";
import Image from "next/image";

const COLORS = {
    purple: "rgb(37, 99, 235)",
    emerald: "#367c53",
    cyan: "#0c9aa7",
} as const;

interface EventCardProps {
    event: MatchedEvent;
}

function PriceDisplay({
    yesPrice,
    noPrice,
    accentColor,
}: {
    yesPrice: string;
    noPrice: string;
    accentColor: "purple" | "cyan";
}) {
    const dotColor = accentColor === "purple" ? COLORS.purple : COLORS.cyan;

    return (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <div className="flex items-center gap-1.5">
                <span
                    className="h-1 w-1 shrink-0 rounded-full"
                    style={{ backgroundColor: dotColor }}
                />
                <span className="text-[10px] font-semibold uppercase tracking-wide text-gray-500 dark:text-white/50">
                    YES
                </span>
                <span className="tabular-nums font-bold text-gray-900 dark:text-white">
                    {yesPrice}
                </span>
            </div>
            <div className="flex items-center gap-1.5">
                <span className="text-[10px] font-semibold uppercase tracking-wide text-gray-500 dark:text-white/50">
                    NO
                </span>
                <span className="tabular-nums font-bold text-gray-500 dark:text-white/60">
                    {noPrice}
                </span>
            </div>
        </div>
    );
}

function PlatformSection({
    platform,
    outcomes,
    outcomeName,
    variant,
}: {
    platform: "kalshi" | "polymarket";
    outcomes: OutcomeOdds[];
    outcomeName: string;
    variant: "purple" | "cyan";
}) {
    const match = findOutcome(outcomes, outcomeName);
    const logo = platform === "kalshi" ? "/kalshi-logo-v2.png" : "/polymarket-logo.png";
    const label = platform === "kalshi" ? "KALSHI" : "POLYMARKET";
    const accentColor = variant === "purple" ? COLORS.purple : COLORS.cyan;

    return (
        <div className="relative flex-1">
            <div className="mb-2.5 flex items-center gap-2">
                <Image
                    src={logo}
                    alt={label}
                    width={22}
                    height={22}
                    className={platform === "polymarket" ? "rounded-full" : "rounded"}
                />
                <span className="text-xs font-bold uppercase tracking-wide text-gray-700 dark:text-white/60">
                    {label}
                </span>
            </div>
            {match ? (
                <PriceDisplay
                    yesPrice={`${match.yes_price}¢`}
                    noPrice={`${match.no_price}¢`}
                    accentColor={variant}
                />
            ) : (
                <div className="flex h-12 items-center justify-center rounded-lg bg-gray-50 dark:bg-white/5 dark:text-white/30">
                    <span className="text-[10px] font-semibold uppercase tracking-wide text-gray-400 dark:text-white/30">
                        Not Available
                    </span>
                </div>
            )}
        </div>
    );
}

function getLeftBorderContent(
    hasKalshi: boolean,
    hasPolymarket: boolean,
    hasArb: boolean
): React.ReactNode {
    if (hasArb) {
        return (
            <div
                className="absolute left-0 top-0 bottom-0 w-[3px] rounded-l-xl"
                style={{ backgroundColor: "rgb(37, 99, 235)" }}
            />
        );
    }
    if (hasKalshi && hasPolymarket) {
        return (
            <div
                className="absolute left-0 top-0 bottom-0 w-[3px] rounded-l-xl"
                style={{
                    background: `linear-gradient(to bottom, ${COLORS.purple}, ${COLORS.cyan})`,
                }}
            />
        );
    }
    if (hasKalshi) {
        return (
            <div
                className="absolute left-0 top-0 bottom-0 w-[3px] rounded-l-xl"
                style={{ backgroundColor: COLORS.purple }}
            />
        );
    }
    if (hasPolymarket) {
        return (
            <div
                className="absolute left-0 top-0 bottom-0 w-[3px] rounded-l-xl"
                style={{ backgroundColor: COLORS.cyan }}
            />
        );
    }
    return null;
}

export function EventCard({ event }: EventCardProps) {
    const outcomes = getEventOutcomes(event);
    const hasArb = !!event.arb_opportunity;
    const primaryOutcome = outcomes[0] ?? "Yes";
    const hasKalshi = !!findOutcome(event.kalshi_outcomes, primaryOutcome);
    const hasPolymarket = !!findOutcome(event.polymarket_outcomes, primaryOutcome);

    return (
        <div className="group relative overflow-hidden rounded-xl border border-gray-200 bg-white shadow-sm transition-all hover:border-gray-300 hover:shadow-md dark:border-white/10 dark:bg-[#111827] dark:hover:border-white/20">
            {/* 3px colored left border (gradient when both platforms, solid otherwise) */}
            {getLeftBorderContent(hasKalshi, hasPolymarket, hasArb)}
            {/* Top-edge glow for arb cards */}
            {hasArb && (
                <div
                    className="pointer-events-none absolute inset-x-0 top-0 h-0.5 opacity-60"
                    style={{
                        background: `linear-gradient(90deg, transparent 0%, rgb(37, 99, 235) 30%, rgb(37, 99, 235) 70%, transparent 100%)`,
                    }}
                />
            )}

            {/* Emerald top-right glow corner for arb cards */}
            {hasArb && (
                <div
                    className="pointer-events-none absolute -right-12 -top-12 h-24 w-24 rounded-full opacity-20 blur-2xl"
                    style={{ backgroundColor: COLORS.emerald }}
                />
            )}

            <div className="p-5">
                {/* Card Header */}
                <div className="mb-4 flex items-start justify-between gap-3">
                    <h3 className="text-sm font-semibold leading-snug text-gray-900 line-clamp-2 dark:text-white">
                        {event.event_title}
                    </h3>
                    {hasArb && (
                        <span
                            className="shrink-0 whitespace-nowrap rounded-full px-2.5 py-1 text-xs font-bold text-white"
                            style={{ backgroundColor: COLORS.emerald }}
                        >
                            +{event.arb_opportunity!.margin_pct.toFixed(1)}% ARB
                        </span>
                    )}
                </div>

                {/* Platform Sections with gradient divider */}
                <div className="flex gap-6">
                    <PlatformSection
                        platform="kalshi"
                        outcomes={event.kalshi_outcomes}
                        outcomeName={primaryOutcome}
                        variant="purple"
                    />
                    {/* Gradient vertical divider between sections */}
                    <div
                        className="w-px shrink-0 self-stretch opacity-40"
                        style={{
                            background: `linear-gradient(to bottom, ${COLORS.purple}, ${COLORS.cyan})`,
                        }}
                    />
                    <PlatformSection
                        platform="polymarket"
                        outcomes={event.polymarket_outcomes}
                        outcomeName={primaryOutcome}
                        variant="cyan"
                    />
                </div>
            </div>
        </div>
    );
}
