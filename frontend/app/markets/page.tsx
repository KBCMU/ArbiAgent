"use client";

import { useState, useEffect, useCallback, useMemo } from "react";
import { Sidebar } from "@/components/Sidebar";
import { Header } from "@/components/Header";
import { FilterBar, isSportCategory } from "@/components/FilterBar";
import { EventTable, type ViewMode } from "@/components/EventTable";
import { ConnectionError } from "@/components/ConnectionError";
import { fetchMatchedEvents, MatchedEvent, type MarketType } from "@/lib/api";

const POLL_INTERVAL_MS = 15_000;

export default function MarketsPage() {
    const [allEvents, setAllEvents] = useState<MatchedEvent[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [activeStatus, setActiveStatus] = useState("open");
    const [activeCategory, setActiveCategory] = useState("sports");
    const [activeSubCategory, setActiveSubCategory] = useState("all");
    const [activeMarketType, setActiveMarketType] = useState<MarketType>("moneyline");
    const [searchQuery, setSearchQuery] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [viewMode, setViewMode] = useState<ViewMode>("row");

    const handleViewModeChange = useCallback((mode: ViewMode) => {
        setViewMode(mode);
        if (mode === "row" && activeCategory === "all") {
            setActiveCategory("sports");
        }
    }, [activeCategory]);

    const loadEvents = useCallback(
        async (showLoadingSpinner: boolean) => {
            if (showLoadingSpinner) setIsLoading(true);

            try {
                const response = await fetchMatchedEvents({
                    limit: 500,
                    status: activeStatus === "all" ? undefined : activeStatus,
                });
                setAllEvents(response.events);
                setError(null);
            } catch (err) {
                setError(
                    err instanceof Error
                        ? err.message
                        : "Failed to load events",
                );
            } finally {
                setIsLoading(false);
            }
        },
        [activeStatus],
    );

    useEffect(() => {
        loadEvents(true);
    }, [loadEvents]);

    useEffect(() => {
        const interval = setInterval(
            () => loadEvents(false),
            POLL_INTERVAL_MS,
        );
        return () => clearInterval(interval);
    }, [loadEvents]);

    const isSportsView = activeCategory === "sports" || isSportCategory(activeCategory);

    const filteredEvents = useMemo(() => {
        let result = allEvents;

        if (activeCategory !== "all") {
            if (activeCategory === "sports") {
                if (activeSubCategory !== "all") {
                    result = result.filter((e) => e.sport === activeSubCategory);
                } else {
                    result = result.filter((e) => isSportCategory(e.sport));
                }
            } else {
                result = result.filter((e) => e.sport === activeCategory);
            }
        }

        if (isSportsView) {
            result = result.filter((e) => e.market_type === activeMarketType);
        }

        if (searchQuery.trim()) {
            const q = searchQuery.toLowerCase();
            result = result.filter((e) =>
                e.event_title.toLowerCase().includes(q),
            );
        }

        return result;
    }, [allEvents, activeCategory, activeSubCategory, activeMarketType, isSportsView, searchQuery]);

    return (
        <div className="flex min-h-screen" style={{ background: 'var(--bg-main)' }}>
            <Sidebar />

            <main className="min-w-0 flex-1 overflow-x-hidden pl-48">
                <Header />
                <FilterBar
                    activeStatus={activeStatus}
                    onStatusChange={setActiveStatus}
                    activeCategory={activeCategory}
                    onCategoryChange={(val) => {
                        setActiveCategory(val);
                        setActiveSubCategory("all");
                        setActiveMarketType("moneyline");
                    }}
                    activeSubCategory={activeSubCategory}
                    onSubCategoryChange={(val) => {
                        setActiveSubCategory(val);
                        setActiveMarketType("moneyline");
                    }}
                    activeMarketType={activeMarketType}
                    onMarketTypeChange={setActiveMarketType}
                    searchQuery={searchQuery}
                    onSearchChange={setSearchQuery}
                    viewMode={viewMode}
                    onViewModeChange={handleViewModeChange}
                />

                <div className={viewMode === "row" ? "" : "p-6"}>
                    {error ? (
                        <ConnectionError
                            error={error}
                            onRetry={() => loadEvents(true)}
                        />
                    ) : (
                        <EventTable
                            events={filteredEvents}
                            isLoading={isLoading}
                            viewMode={viewMode}
                            activeCategory={activeCategory}
                        />
                    )}
                </div>
            </main>
        </div>
    );
}
