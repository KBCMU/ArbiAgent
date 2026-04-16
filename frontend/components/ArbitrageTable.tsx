"use client";

import { ActiveArb } from "@/lib/api";
import Image from "next/image";

interface ArbitrageTableProps {
    arbs: ActiveArb[];
    isLoading?: boolean;
}

const PLATFORM_META: Record<string, { logo: string; label: string }> = {
    kalshi: { logo: "/kalshi-logo-v2.png", label: "Kalshi" },
    polymarket: { logo: "/polymarket-logo.png", label: "Polymarket" },
};

// Tinted pill backgrounds per sport — mirrors SPORT_META in EventRow so the
// whole product feels like one design system.
const SPORT_PILL: Record<string, string> = {
    nba: "bg-orange-50 text-orange-700 dark:bg-orange-500/10 dark:text-orange-300",
    nfl: "bg-amber-50 text-amber-800 dark:bg-amber-500/10 dark:text-amber-300",
    mlb: "bg-red-50 text-red-700 dark:bg-red-500/10 dark:text-red-300",
    nhl: "bg-sky-50 text-sky-700 dark:bg-sky-500/10 dark:text-sky-300",
    soccer: "bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300",
    tennis: "bg-lime-50 text-lime-800 dark:bg-lime-500/10 dark:text-lime-300",
    golf: "bg-green-50 text-green-700 dark:bg-green-500/10 dark:text-green-300",
    ufc: "bg-rose-50 text-rose-700 dark:bg-rose-500/10 dark:text-rose-300",
    mma: "bg-rose-50 text-rose-700 dark:bg-rose-500/10 dark:text-rose-300",
    boxing: "bg-rose-50 text-rose-700 dark:bg-rose-500/10 dark:text-rose-300",
    ncaa: "bg-indigo-50 text-indigo-700 dark:bg-indigo-500/10 dark:text-indigo-300",
};

function sportPillClass(sport: string): string {
    return (
        SPORT_PILL[sport.toLowerCase()] ??
        "bg-gray-100 text-gray-700 dark:bg-white/10 dark:text-white/70"
    );
}

function formatPrice(price: number): string {
    return `${Math.round(price * 100)}¢`;
}

function timeSince(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const seconds = Math.floor(diff / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
}

function PlatformBadge({
    platform,
    side,
    price,
}: {
    platform: string;
    side: "YES" | "NO";
    price: string;
}) {
    const meta = PLATFORM_META[platform] ?? {
        logo: "",
        label: platform,
    };

    return (
        <div className="flex items-center gap-2">
            {meta.logo && (
                <Image
                    src={meta.logo}
                    alt={meta.label}
                    width={18}
                    height={18}
                    className="rounded"
                />
            )}
            <div className="flex items-baseline gap-1.5">
                <span className="text-xs font-medium text-gray-500 dark:text-white/70">
                    {side}
                </span>
                <span className="text-sm font-semibold tabular-nums text-gray-900 dark:text-white">
                    {price}
                </span>
            </div>
        </div>
    );
}

function SkeletonRow() {
    return (
        <div className="grid grid-cols-12 items-center gap-4 border-b border-[var(--border-subtle)] px-6 py-4 dark:border-white/10">
            <div className="col-span-1">
                <div className="skeleton-shimmer mx-auto h-5 w-12 rounded" />
            </div>
            <div className="col-span-3">
                <div className="skeleton-shimmer h-4 w-3/4 rounded" />
                <div className="skeleton-shimmer mt-1.5 h-3 w-1/3 rounded" />
            </div>
            <div className="col-span-2">
                <div className="skeleton-shimmer h-3 w-10 rounded" />
            </div>
            <div className="col-span-3">
                <div className="skeleton-shimmer h-4 w-24 rounded" />
            </div>
            <div className="col-span-3">
                <div className="skeleton-shimmer h-4 w-24 rounded" />
            </div>
        </div>
    );
}

export function ArbitrageTable({ arbs, isLoading }: ArbitrageTableProps) {
    return (
        <div className="overflow-hidden rounded-xl border border-[var(--border-default)] bg-white shadow-card dark:border-white/10 dark:bg-[#111827]">
            {/* Header */}
            <div className="grid grid-cols-12 items-center gap-4 border-b border-[var(--border-default)] bg-gray-50/70 px-6 py-3 dark:border-white/10 dark:bg-white/[0.025]">
                <div className="col-span-1 text-center">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-500 dark:text-white/50">
                        Margin
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-500 dark:text-white/50">
                        Event
                    </span>
                </div>
                <div className="col-span-2">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-500 dark:text-white/50">
                        Outcome
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-500 dark:text-white/50">
                        Buy Leg
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-500 dark:text-white/50">
                        Sell Leg
                    </span>
                </div>
            </div>

            {/* Body */}
            <div className="max-h-[calc(100vh-300px)] overflow-y-auto">
                {isLoading ? (
                    Array.from({ length: 6 }).map((_, i) => (
                        <SkeletonRow key={i} />
                    ))
                ) : arbs.length === 0 ? (
                    <div className="flex h-64 flex-col items-center justify-center gap-2">
                        <p className="text-sm font-medium text-gray-900 dark:text-white">
                            No active arbitrage opportunities
                        </p>
                        <p className="max-w-sm text-center text-sm text-gray-500 dark:text-white/70">
                            The scanner checks every 2 seconds. Opportunities
                            will appear here as soon as a cross-platform price
                            discrepancy is detected.
                        </p>
                    </div>
                ) : (
                    arbs.map((arb, idx) => {
                        const isEven = idx % 2 === 0;
                        return (
                            <div
                                key={`${arb.canonical_event_id}-${arb.outcome}-${arb.buy_platform}-${idx}`}
                                className="group grid grid-cols-12 items-center gap-4 border-b border-[var(--border-subtle)] px-6 py-3 transition-colors duration-150 hover:bg-[var(--row-hover)] dark:border-white/10 dark:hover:bg-white/[0.04]"
                                style={{
                                    borderLeft: "3px solid var(--emerald-brand)",
                                    backgroundColor: isEven
                                        ? "transparent"
                                        : "var(--row-zebra)",
                                }}
                            >
                                {/* Margin */}
                                <div className="col-span-1 text-center">
                                    <span
                                        className="inline-block rounded-md px-2 py-1 text-sm font-semibold tabular-nums text-white"
                                        style={{
                                            background: "var(--emerald-brand)",
                                        }}
                                    >
                                        +{arb.margin_pct.toFixed(1)}%
                                    </span>
                                </div>

                                {/* Event info */}
                                <div className="col-span-3 min-w-0">
                                    <p className="truncate text-sm font-medium text-gray-900 transition-colors group-hover:text-[rgb(37,99,235)] dark:text-white dark:group-hover:text-[rgb(37,99,235)]">
                                        {arb.event_title}
                                    </p>
                                    <div className="mt-1 flex items-center gap-2">
                                        <span
                                            className={`inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${sportPillClass(arb.sport)}`}
                                        >
                                            {arb.sport}
                                        </span>
                                        <span className="text-[10px] text-gray-400 dark:text-white/40">
                                            {timeSince(arb.detected_at)}
                                        </span>
                                    </div>
                                </div>

                                {/* Outcome */}
                                <div className="col-span-2">
                                    <span className="text-xs font-semibold uppercase tracking-wide text-gray-700 dark:text-white/80">
                                        {arb.outcome}
                                    </span>
                                </div>

                                {/* Buy leg */}
                                <div className="col-span-3">
                                    <PlatformBadge
                                        platform={arb.buy_platform}
                                        side="YES"
                                        price={formatPrice(arb.buy_price)}
                                    />
                                </div>

                                {/* Sell leg */}
                                <div className="col-span-3">
                                    <PlatformBadge
                                        platform={arb.sell_platform}
                                        side="NO"
                                        price={formatPrice(
                                            1.0 - arb.sell_price,
                                        )}
                                    />
                                </div>
                            </div>
                        );
                    })
                )}
            </div>
        </div>
    );
}
