"use client";

import { MatchedEvent } from "@/lib/api";
import { EventCard } from "./EventCard";
import { EventRow } from "./EventRow";
import Image from "next/image";

export type ViewMode = "card" | "row";

interface EventTableProps {
    events: MatchedEvent[];
    isLoading?: boolean;
    viewMode?: ViewMode;
    activeCategory?: string;
}

const CATEGORY_CONFIG: Record<string, { outcomeLabel: string; showSportCol: boolean; showEventCol: boolean }> = {
    sports:    { outcomeLabel: "Team",      showSportCol: true,  showEventCol: false },
    politics:  { outcomeLabel: "Candidate", showSportCol: false, showEventCol: true },
    crypto:    { outcomeLabel: "Asset",     showSportCol: false, showEventCol: true },
    economics: { outcomeLabel: "Indicator", showSportCol: false, showEventCol: true },
    world:     { outcomeLabel: "Outcome",   showSportCol: false, showEventCol: true },
    science:   { outcomeLabel: "Outcome",   showSportCol: false, showEventCol: true },
    culture:   { outcomeLabel: "Topic",     showSportCol: false, showEventCol: true },
};

function getCategoryConfig(cat: string) {
    return CATEGORY_CONFIG[cat] ?? CATEGORY_CONFIG.sports;
}

function SkeletonCard() {
    return (
        <div className="rounded-xl border border-gray-200 bg-white p-5 shadow-card dark:border-white/10 dark:bg-[#111827]">
            <div className="mb-4 flex items-center justify-between">
                <div className="skeleton-shimmer h-5 w-3/5 rounded" />
                <div className="skeleton-shimmer h-5 w-16 rounded-full" />
            </div>
            <div className="grid grid-cols-2 gap-4">
                <div>
                    <div className="skeleton-shimmer mb-2 h-4 w-20 rounded" />
                    <div className="flex gap-2">
                        <div className="skeleton-shimmer h-10 w-16 rounded-lg" />
                        <div className="skeleton-shimmer h-10 w-16 rounded-lg" />
                    </div>
                </div>
                <div>
                    <div className="skeleton-shimmer mb-2 h-4 w-24 rounded" />
                    <div className="flex gap-2">
                        <div className="skeleton-shimmer h-10 w-16 rounded-lg" />
                        <div className="skeleton-shimmer h-10 w-16 rounded-lg" />
                    </div>
                </div>
            </div>
        </div>
    );
}

const ROW_BORDER = "border-b border-[var(--border-subtle)]";
const MONO_HEADER = "text-[11px] font-semibold uppercase tracking-[0.08em] text-gray-500 dark:text-white/45";
const MONO_STYLE = { fontFamily: "var(--font-sans)" };

function SkeletonTableRows({ showSportCol, showEventCol }: { showSportCol: boolean; showEventCol: boolean }) {
    return (
        <>
            {Array.from({ length: 10 }).map((_, i) => (
                <tr key={i} className={ROW_BORDER}>
                    <td className="py-3.5 pl-5 pr-2">
                        <div className="skeleton-shimmer h-3 w-16 rounded" />
                    </td>
                    {showSportCol && (
                        <td className="px-2 py-3.5">
                            <div className="skeleton-shimmer h-5 w-16 rounded-full" />
                        </td>
                    )}
                    {showEventCol && (
                        <td className="px-3 py-3.5">
                            <div className="skeleton-shimmer h-4 w-36 rounded" />
                        </td>
                    )}
                    <td className="px-3 py-3.5">
                        <div className="skeleton-shimmer h-4 w-28 rounded" />
                    </td>
                    <td className="px-3 py-3.5">
                        <div className="skeleton-shimmer mx-auto h-4 w-10 rounded" />
                    </td>
                    <td className="px-3 py-3.5">
                        <div className="skeleton-shimmer mx-auto h-4 w-10 rounded" />
                    </td>
                    <td className="px-3 py-3.5">
                        <div className="skeleton-shimmer mx-auto h-4 w-10 rounded" />
                    </td>
                    <td className="px-3 py-3.5 pr-5">
                        <div className="skeleton-shimmer mx-auto h-4 w-10 rounded" />
                    </td>
                </tr>
            ))}
        </>
    );
}

function EmptyState() {
    return (
        <div className="flex h-64 items-center justify-center">
            <div className="text-center">
                <p className="text-sm font-medium text-gray-900 dark:text-white">
                    No events match your filters
                </p>
                <p className="mt-1 text-sm text-gray-500 dark:text-white/50">
                    Try a different category or search term, or wait for the
                    backend to discover events
                </p>
            </div>
        </div>
    );
}

export function EventTable({ events, isLoading, viewMode = "card", activeCategory = "sports" }: EventTableProps) {
    if (!isLoading && events.length === 0) {
        return <EmptyState />;
    }

    if (viewMode === "row") {
        const config = getCategoryConfig(activeCategory);

        return (
            <div className="px-6 py-6">
                <div className="overflow-hidden rounded-xl border border-[var(--border-default)] bg-white shadow-card dark:border-white/10 dark:bg-[#111827]">
                    <div className="overflow-x-auto">
                        <table className="w-full min-w-[640px] border-collapse">
                            <thead>
                                <tr className="border-b border-[var(--border-default)] bg-gray-50/70 dark:border-white/10 dark:bg-white/[0.025]">
                                    <th className="py-3 pl-5 pr-2 text-left" style={{ width: 110 }}>
                                        <span className={MONO_HEADER} style={MONO_STYLE}>Date</span>
                                    </th>
                                    {config.showSportCol && (
                                        <th className="px-2 py-3 text-left" style={{ width: 100 }}>
                                            <span className={MONO_HEADER} style={MONO_STYLE}>Sport</span>
                                        </th>
                                    )}
                                    {config.showEventCol && (
                                        <th className="px-3 py-3 text-left" style={{ width: 220 }}>
                                            <span className={MONO_HEADER} style={MONO_STYLE}>Event</span>
                                        </th>
                                    )}
                                    <th className="px-3 py-3 text-left">
                                        <span className={MONO_HEADER} style={MONO_STYLE}>{config.outcomeLabel}</span>
                                    </th>
                                    <th className="px-3 py-3 text-center" style={{ width: 100 }}>
                                        <span className={MONO_HEADER} style={MONO_STYLE}>Avg</span>
                                    </th>
                                    <th className="px-3 py-3 text-center" style={{ width: 100 }}>
                                        <span className={MONO_HEADER} style={MONO_STYLE}>Best Odds</span>
                                    </th>
                                    <th className="px-3 py-3 text-center" style={{ width: 110 }}>
                                        <Image
                                            src="/kalshi-logo-v2.png"
                                            alt="Kalshi"
                                            width={22}
                                            height={22}
                                            className="mx-auto rounded"
                                        />
                                    </th>
                                    <th className="px-3 py-3 pr-5 text-center" style={{ width: 110 }}>
                                        <Image
                                            src="/polymarket-logo.png"
                                            alt="Polymarket"
                                            width={22}
                                            height={22}
                                            className="mx-auto rounded-full"
                                        />
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {isLoading ? (
                                    <SkeletonTableRows showSportCol={config.showSportCol} showEventCol={config.showEventCol} />
                                ) : (
                                    events.map((event, i) => (
                                        <EventRow
                                            key={event.id}
                                            event={event}
                                            rowIndex={i}
                                            showSportCol={config.showSportCol}
                                            showEventCol={config.showEventCol}
                                        />
                                    ))
                                )}
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        );
    }

    if (isLoading) {
        return (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                {Array.from({ length: 6 }).map((_, i) => (
                    <SkeletonCard key={i} />
                ))}
            </div>
        );
    }

    return (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {events.map((event) => (
                <EventCard key={event.id} event={event} />
            ))}
        </div>
    );
}
