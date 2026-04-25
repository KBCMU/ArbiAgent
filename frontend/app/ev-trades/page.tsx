"use client";

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Sidebar } from "@/components/Sidebar";
import { Header } from "@/components/Header";
import { ConnectionError } from "@/components/ConnectionError";
import { fetchEvOpportunities, EvOpportunity } from "@/lib/api";
import { Sparkles, BarChart3, ChevronDown, Filter, TrendingUp } from "lucide-react";

const POLL_INTERVAL_MS = 5_000;

const SPORTS_LEAGUES = ["nfl", "nba", "mlb", "nhl", "cfb", "cbb"];

type FilterOption = { value: string; label: string };

const LEAGUE_OPTIONS: FilterOption[] = [
    { value: "all", label: "All Sports" },
    { value: "nfl", label: "NFL" },
    { value: "nba", label: "NBA" },
    { value: "mlb", label: "MLB" },
    { value: "nhl", label: "NHL" },
    { value: "cfb", label: "CFB" },
    { value: "cbb", label: "CBB" },
];

const EDGE_OPTIONS: FilterOption[] = [
    { value: "0", label: "0%+" },
    { value: "2", label: "2%+" },
    { value: "3", label: "3%+" },
    { value: "5", label: "5%+" },
    { value: "8", label: "8%+" },
    { value: "10", label: "10%+" },
];

function StatCard({
    label,
    value,
    icon: Icon,
    accent = false,
}: {
    label: string;
    value: string;
    icon: React.ElementType;
    accent?: boolean;
}) {
    return (
        <div
            className={`relative overflow-hidden rounded-xl border p-4 shadow-sm ${
                accent
                    ? "border-[#367c53]/30 bg-white dark:border-[#367c53]/30 dark:bg-[#111827]"
                    : "border-gray-200 bg-white dark:border-white/10 dark:bg-[#111827]"
            }`}
        >
            {accent && (
                <div
                    className="absolute inset-x-0 top-0 h-0.5"
                    style={{
                        background:
                            "linear-gradient(90deg, var(--emerald-brand), var(--cyan-brand))",
                    }}
                />
            )}
            <div className="flex items-center gap-2">
                <Icon
                    className="h-4 w-4"
                    style={{ color: accent ? "var(--emerald-brand)" : undefined }}
                />
                <span
                    className={`text-xs font-medium ${
                        accent
                            ? "text-[#367c53]"
                            : "text-gray-500 dark:text-white/50"
                    }`}
                >
                    {label}
                </span>
            </div>
            <p
                className={`mt-2 text-2xl font-bold ${
                    accent
                        ? "text-[#367c53]"
                        : "text-gray-900 dark:text-white"
                }`}
            >
                {value}
            </p>
        </div>
    );
}

function FilterSelect({
    value,
    onChange,
    options,
}: {
    value: string;
    onChange: (v: string) => void;
    options: FilterOption[];
}) {
    const [open, setOpen] = useState(false);
    const ref = useRef<HTMLDivElement>(null);
    const selected = options.find((o) => o.value === value);

    useEffect(() => {
        function handleClick(e: MouseEvent) {
            if (ref.current && !ref.current.contains(e.target as Node))
                setOpen(false);
        }
        document.addEventListener("mousedown", handleClick);
        return () => document.removeEventListener("mousedown", handleClick);
    }, []);

    return (
        <div ref={ref} className="relative">
            <button
                onClick={() => setOpen(!open)}
                className="flex items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm font-medium text-gray-700 shadow-sm transition-colors hover:bg-gray-50 dark:border-white/10 dark:bg-[#111827] dark:text-white/80 dark:hover:bg-white/5"
            >
                <span>{selected?.label}</span>
                <ChevronDown className="h-3.5 w-3.5 text-gray-400" />
            </button>
            {open && (
                <div className="absolute left-0 top-full z-50 mt-1 min-w-[160px] overflow-hidden rounded-lg border border-gray-200 bg-white shadow-lg dark:border-white/10 dark:bg-[#1a1f2e]">
                    {options.map((opt) => (
                        <button
                            key={opt.value}
                            onClick={() => {
                                onChange(opt.value);
                                setOpen(false);
                            }}
                            className={`block w-full px-3 py-2 text-left text-sm transition-colors ${
                                opt.value === value
                                    ? "bg-emerald-50 font-semibold text-emerald-700 dark:bg-emerald-900/20 dark:text-emerald-400"
                                    : "text-gray-700 hover:bg-gray-50 dark:text-white/70 dark:hover:bg-white/5"
                            }`}
                        >
                            {opt.label}
                        </button>
                    ))}
                </div>
            )}
        </div>
    );
}

function edgeColor(edgePct: number): string {
    if (edgePct >= 5) return "text-emerald-500 dark:text-emerald-400";
    if (edgePct >= 2) return "text-amber-500 dark:text-amber-400";
    return "text-gray-600 dark:text-white/70";
}

function edgeBadgeBg(edgePct: number): string {
    if (edgePct >= 5)
        return "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400";
    if (edgePct >= 2)
        return "bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400";
    return "bg-gray-100 text-gray-700 dark:bg-white/10 dark:text-white/70";
}

function formatMoneyline(ml: number): string {
    return ml >= 0 ? `+${ml.toFixed(0)}` : ml.toFixed(0);
}

function platformLabel(p: string): string {
    if (p === "kalshi") return "Kalshi";
    if (p === "polymarket") return "Polymarket";
    return p;
}

export default function EvTradesPage() {
    const [opps, setOpps] = useState<EvOpportunity[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const [league, setLeague] = useState("all");
    const [minEdge, setMinEdge] = useState("0");

    const filteredOpps = useMemo(() => {
        return opps.filter((opp) => {
            if (league !== "all" && opp.sport.toLowerCase() !== league)
                return false;
            if (Number(minEdge) > 0 && opp.edge_pct < Number(minEdge))
                return false;
            return true;
        });
    }, [opps, league, minEdge]);

    const avgEdge = useMemo(() => {
        if (filteredOpps.length === 0) return null;
        const sum = filteredOpps.reduce((acc, o) => acc + o.edge_pct, 0);
        return sum / filteredOpps.length;
    }, [filteredOpps]);

    const loadData = useCallback(
        async (showSpinner: boolean) => {
            if (showSpinner) setIsLoading(true);
            try {
                const data = await fetchEvOpportunities();
                setOpps(data);
                setError(null);
            } catch (err) {
                setError(
                    err instanceof Error
                        ? err.message
                        : "Failed to load +EV data"
                );
            } finally {
                setIsLoading(false);
            }
        },
        []
    );

    useEffect(() => {
        loadData(true);
    }, [loadData]);

    useEffect(() => {
        const interval = setInterval(() => loadData(false), POLL_INTERVAL_MS);
        return () => clearInterval(interval);
    }, [loadData]);

    return (
        <div
            className="flex min-h-screen"
            style={{ background: "var(--bg-main)" }}
        >
            <Sidebar />

            <main className="flex-1 pl-48">
                <Header />

                <div className="p-6">
                    {/* Stats */}
                    <div className="mb-6 grid grid-cols-2 gap-4">
                        <StatCard
                            label="+EV Opportunities"
                            value={
                                isLoading ? "—" : String(filteredOpps.length)
                            }
                            icon={Sparkles}
                            accent
                        />
                        <StatCard
                            label="Avg Edge"
                            value={
                                avgEdge != null
                                    ? `${avgEdge.toFixed(2)}%`
                                    : "—"
                            }
                            icon={BarChart3}
                        />
                    </div>

                    {/* Filters */}
                    <div className="mb-4 flex items-center gap-3 flex-wrap">
                        <div className="flex items-center gap-1.5 text-gray-400 dark:text-white/30">
                            <Filter className="h-4 w-4" />
                            <span className="text-[11px] font-bold uppercase tracking-widest">
                                Filters
                            </span>
                        </div>

                        <FilterSelect
                            value={league}
                            onChange={setLeague}
                            options={LEAGUE_OPTIONS}
                        />

                        <FilterSelect
                            value={minEdge}
                            onChange={setMinEdge}
                            options={EDGE_OPTIONS}
                        />
                    </div>

                    {error ? (
                        <ConnectionError
                            error={error}
                            onRetry={() => loadData(true)}
                        />
                    ) : isLoading ? (
                        <div className="flex items-center justify-center py-20">
                            <div className="h-8 w-8 animate-spin rounded-full border-2 border-emerald-500 border-t-transparent" />
                        </div>
                    ) : filteredOpps.length === 0 ? (
                        <div className="flex flex-col items-center justify-center py-20 text-gray-400 dark:text-white/30">
                            <TrendingUp className="mb-3 h-10 w-10" />
                            <p className="text-sm font-medium">
                                No +EV opportunities right now
                            </p>
                            <p className="mt-1 text-xs">
                                The scanner is checking prediction market
                                prices against Vegas consensus odds
                            </p>
                        </div>
                    ) : (
                        <div className="overflow-hidden rounded-xl border border-gray-200 bg-white shadow-sm dark:border-white/10 dark:bg-[#111827]">
                            <table className="w-full text-sm">
                                <thead>
                                    <tr className="border-b border-gray-100 bg-gray-50/50 dark:border-white/5 dark:bg-white/[0.02]">
                                        <th className="px-4 py-3 text-left text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Event
                                        </th>
                                        <th className="px-3 py-3 text-left text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Sport
                                        </th>
                                        <th className="px-3 py-3 text-left text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Outcome
                                        </th>
                                        <th className="px-3 py-3 text-left text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Platform
                                        </th>
                                        <th className="px-3 py-3 text-right text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Mkt Price
                                        </th>
                                        <th className="px-3 py-3 text-right text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Fair Prob
                                        </th>
                                        <th className="px-3 py-3 text-right text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Edge
                                        </th>
                                        <th className="px-3 py-3 text-right text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            ML
                                        </th>
                                        <th className="px-3 py-3 text-right text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-white/40">
                                            Kelly
                                        </th>
                                    </tr>
                                </thead>
                                <tbody className="divide-y divide-gray-100 dark:divide-white/5">
                                    {filteredOpps.map((opp, i) => (
                                        <tr
                                            key={`${opp.canonical_event_id}-${opp.outcome}-${opp.market_platform}-${i}`}
                                            className="transition-colors hover:bg-gray-50/50 dark:hover:bg-white/[0.02]"
                                        >
                                            <td className="max-w-[220px] truncate px-4 py-3 font-medium text-gray-900 dark:text-white">
                                                {opp.event_title}
                                            </td>
                                            <td className="px-3 py-3">
                                                <span className="inline-block rounded-md bg-gray-100 px-2 py-0.5 text-xs font-semibold uppercase text-gray-600 dark:bg-white/10 dark:text-white/60">
                                                    {opp.sport}
                                                </span>
                                            </td>
                                            <td className="px-3 py-3 font-medium text-gray-800 dark:text-white/80">
                                                {opp.outcome}
                                            </td>
                                            <td className="px-3 py-3 text-gray-600 dark:text-white/60">
                                                {platformLabel(
                                                    opp.market_platform
                                                )}
                                            </td>
                                            <td className="px-3 py-3 text-right tabular-nums text-gray-800 dark:text-white/80">
                                                {(opp.market_price * 100).toFixed(1)}¢
                                            </td>
                                            <td className="px-3 py-3 text-right tabular-nums text-gray-800 dark:text-white/80">
                                                {(opp.vegas_fair_prob * 100).toFixed(1)}%
                                            </td>
                                            <td className="px-3 py-3 text-right">
                                                <span
                                                    className={`inline-block rounded-md px-2 py-0.5 text-xs font-bold tabular-nums ${edgeBadgeBg(opp.edge_pct)}`}
                                                >
                                                    +{opp.edge_pct.toFixed(1)}%
                                                </span>
                                            </td>
                                            <td className="px-3 py-3 text-right tabular-nums text-gray-600 dark:text-white/60">
                                                {formatMoneyline(
                                                    opp.consensus_moneyline
                                                )}
                                            </td>
                                            <td className="px-3 py-3 text-right tabular-nums text-gray-600 dark:text-white/60">
                                                {opp.kelly_fraction != null
                                                    ? `${(opp.kelly_fraction * 100).toFixed(1)}%`
                                                    : "—"}
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                </div>
            </main>
        </div>
    );
}
