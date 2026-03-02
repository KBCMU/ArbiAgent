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
                <span className="text-sm font-bold tabular-nums text-gray-900 dark:text-white">
                    {price}
                </span>
            </div>
        </div>
    );
}

function SkeletonRow() {
    return (
        <div className="grid grid-cols-11 items-center gap-4 border-b border-gray-100 px-6 py-4 dark:border-white/10 dark:bg-[#0a0f1a]">
            <div className="col-span-1">
                <div className="mx-auto h-5 w-12 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
            </div>
            <div className="col-span-3">
                <div className="h-4 w-3/4 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
                <div className="mt-1 h-3 w-1/3 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
            </div>
            <div className="col-span-1">
                <div className="h-3 w-10 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
            </div>
            <div className="col-span-3">
                <div className="h-4 w-24 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
            </div>
            <div className="col-span-3">
                <div className="h-4 w-24 animate-pulse rounded bg-gray-200 dark:bg-white/10" />
            </div>
        </div>
    );
}

export function ArbitrageTable({ arbs, isLoading }: ArbitrageTableProps) {
    return (
        <div className="overflow-hidden rounded-xl border border-gray-200 bg-white shadow-sm dark:border-white/10 dark:bg-[#111827]">
            {/* Header */}
            <div className="grid grid-cols-11 items-center gap-4 border-b border-gray-200 bg-gray-50 px-6 py-3 dark:border-white/10 dark:bg-[#0a0f1a]">
                <div className="col-span-1 text-center">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-white/40">
                        Margin
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-white/40">
                        Event
                    </span>
                </div>
                <div className="col-span-1">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-white/40">
                        Outcome
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-white/40">
                        Buy Leg
                    </span>
                </div>
                <div className="col-span-3">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-white/40">
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
                    arbs.map((arb, idx) => (
                        <div
                            key={`${arb.canonical_event_id}-${arb.outcome}-${arb.buy_platform}-${idx}`}
                            className="group grid grid-cols-11 items-center gap-4 border-b border-gray-100 bg-white px-6 py-3 transition-colors hover:bg-[rgb(37,99,235)]/5 dark:border-white/10 dark:bg-[#111827] dark:hover:bg-white/5"
                            style={{ borderLeft: "3px solid var(--emerald-brand)" }}
                        >
                            {/* Margin */}
                            <div className="col-span-1 text-center">
                                <span
                                    className="inline-block rounded-md px-2 py-1 text-sm font-bold tabular-nums text-white"
                                    style={{ background: 'var(--emerald-brand)' }}
                                >
                                    +{arb.margin_pct.toFixed(1)}%
                                </span>
                            </div>

                            {/* Event info */}
                            <div className="col-span-3 min-w-0">
                                <p className="truncate text-sm font-medium text-gray-900 group-hover:text-[rgb(37,99,235)] dark:text-white dark:group-hover:text-[rgb(37,99,235)]">
                                    {arb.event_title}
                                </p>
                                <div className="mt-0.5 flex items-center gap-2">
                                    <span className="inline-flex items-center rounded bg-gray-100 px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide text-gray-500 dark:bg-white/10 dark:text-white/70">
                                        {arb.sport}
                                    </span>
                                    <span className="text-[10px] text-gray-400 dark:text-white/40">
                                        {timeSince(arb.detected_at)}
                                    </span>
                                </div>
                            </div>

                            {/* Outcome */}
                            <div className="col-span-1">
                                <span className="text-xs font-bold uppercase tracking-wide text-gray-600 dark:text-white/70">
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
                    ))
                )}
            </div>
        </div>
    );
}
