"use client";

import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Sidebar } from "@/components/Sidebar";
import { Header } from "@/components/Header";
import { ArbitrageTable } from "@/components/ArbitrageTable";
import { ConnectionError } from "@/components/ConnectionError";
import { fetchActiveArbs, fetchArbStats, ActiveArb } from "@/lib/api";
import { BarChart3, Zap, ChevronDown, Filter } from "lucide-react";

const POLL_INTERVAL_MS = 5_000;

const SPORTS_LEAGUES = ["nfl", "nba", "mlb", "nhl", "cfb", "cbb", "tennis"];

type FilterOption = { value: string; label: string; disabled?: boolean };

const DISCIPLINES: FilterOption[] = [
    { value: "all", label: "All Events" },
    { value: "sports", label: "Sports" },
    { value: "politics", label: "Politics", disabled: true },
    { value: "crypto", label: "Crypto", disabled: true },
    { value: "economics", label: "Economics", disabled: true },
    { value: "world", label: "World", disabled: true },
    { value: "science", label: "Science", disabled: true },
    { value: "culture", label: "Culture", disabled: true },
];

const LEAGUE_OPTIONS: Record<string, FilterOption[]> = {
    sports: [
        { value: "all", label: "All Leagues" },
        { value: "nfl", label: "NFL" },
        { value: "nba", label: "NBA" },
        { value: "mlb", label: "MLB" },
        { value: "nhl", label: "NHL" },
        { value: "cfb", label: "CFB" },
        { value: "cbb", label: "CBB" },
        { value: "tennis", label: "Tennis" },
    ],
};

const MARGIN_OPTIONS = [
    { value: 0, label: "0%+" },
    { value: 1, label: "1%+" },
    { value: 2, label: "2%+" },
    { value: 3, label: "3%+" },
    { value: 5, label: "5%+" },
    { value: 10, label: "10%+" },
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
            className={`relative overflow-hidden rounded-xl border p-4 shadow-sm ${accent
                ? "border-[#367c53]/30 bg-white dark:border-[#367c53]/30 dark:bg-[#111827]"
                : "border-gray-200 bg-white dark:border-white/10 dark:bg-[#111827]"
                }`}
        >
            {accent && (
                <div
                    className="absolute inset-x-0 top-0 h-0.5"
                    style={{ background: 'linear-gradient(90deg, var(--emerald-brand), var(--cyan-brand))' }}
                />
            )}
            <div className="flex items-center gap-2">
                <Icon
                    className="h-4 w-4"
                    style={{ color: accent ? 'var(--emerald-brand)' : undefined }}
                />
                <span
                    className={`text-xs font-medium ${accent ? "text-[#367c53]" : "text-gray-500 dark:text-white/50"}`}
                >
                    {label}
                </span>
            </div>
            <p
                className={`mt-2 text-2xl font-bold ${accent ? "text-[#367c53]" : "text-gray-900 dark:text-white"}`}
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
            if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
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
                    {options.map((opt) => {
                        const disabled = opt.disabled === true;
                        return (
                            <button
                                key={opt.value}
                                onClick={() => {
                                    if (disabled) return;
                                    onChange(opt.value);
                                    setOpen(false);
                                }}
                                disabled={disabled}
                                title={disabled ? "Coming soon" : undefined}
                                aria-disabled={disabled}
                                className={`block w-full px-3 py-2 text-left text-sm transition-colors ${disabled
                                    ? "cursor-not-allowed text-gray-300 dark:text-white/20"
                                    : opt.value === value
                                        ? "bg-emerald-50 font-semibold text-emerald-700 dark:bg-emerald-900/20 dark:text-emerald-400"
                                        : "text-gray-700 hover:bg-gray-50 dark:text-white/70 dark:hover:bg-white/5"
                                    }`}
                            >
                                {opt.label}
                                {disabled && (
                                    <span className="ml-2 text-[10px] uppercase tracking-wide text-gray-300 dark:text-white/25">
                                        Soon
                                    </span>
                                )}
                            </button>
                        );
                    })}
                </div>
            )}
        </div>
    );
}

export default function ArbitragePage() {
    const [arbs, setArbs] = useState<ActiveArb[]>([]);
    const [stats, setStats] = useState<Record<string, number>>({});
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const [discipline, setDiscipline] = useState("all");
    const [league, setLeague] = useState("all");
    const [minMargin, setMinMargin] = useState("0");

    const handleDisciplineChange = useCallback((v: string) => {
        setDiscipline(v);
        setLeague("all");
    }, []);

    const leagueOptions = LEAGUE_OPTIONS[discipline] ?? null;

    const filteredArbs = useMemo(() => {
        return arbs.filter((arb) => {
            if (discipline !== "all") {
                if (discipline === "sports") {
                    if (!SPORTS_LEAGUES.includes(arb.sport.toLowerCase())) return false;
                } else {
                    if (arb.sport.toLowerCase() !== discipline) return false;
                }
            }

            if (league !== "all" && discipline === "sports") {
                if (arb.sport.toLowerCase() !== league) return false;
            }

            if (Number(minMargin) > 0 && arb.margin_pct < Number(minMargin)) {
                return false;
            }

            return true;
        });
    }, [arbs, discipline, league, minMargin]);

    const activeAvgMargin = useMemo(() => {
        if (filteredArbs.length === 0) return null;
        const sum = filteredArbs.reduce((acc, arb) => acc + arb.margin_pct, 0);
        return sum / filteredArbs.length;
    }, [filteredArbs]);

    const loadArbs = useCallback(async (showSpinner: boolean) => {
        if (showSpinner) setIsLoading(true);

        try {
            const [arbData, statsData] = await Promise.all([
                fetchActiveArbs(),
                fetchArbStats(),
            ]);
            setArbs(arbData);
            setStats(statsData);
            setError(null);
        } catch (err) {
            setError(
                err instanceof Error
                    ? err.message
                    : "Failed to load arbitrage data",
            );
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        loadArbs(true);
    }, [loadArbs]);

    useEffect(() => {
        const interval = setInterval(() => loadArbs(false), POLL_INTERVAL_MS);
        return () => clearInterval(interval);
    }, [loadArbs]);

    return (
        <div className="flex min-h-screen" style={{ background: 'var(--bg-main)' }}>
            <Sidebar />

            <main className="flex-1 pl-48">
                <Header />

                <div className="p-6">
                    {/* Stats row */}
                    <div className="mb-6 grid grid-cols-2 gap-4">
                        <StatCard
                            label="Active Opportunities"
                            value={isLoading ? "—" : String(filteredArbs.length)}
                            icon={Zap}
                            accent
                        />
                        <StatCard
                            label="Avg Margin"
                            value={
                                activeAvgMargin != null
                                    ? `${activeAvgMargin.toFixed(2)}%`
                                    : "—"
                            }
                            icon={BarChart3}
                        />
                    </div>

                    {/* Filters */}
                    <div className="mb-4 flex items-center gap-3 flex-wrap">
                        <div className="flex items-center gap-1.5 text-gray-400 dark:text-white/30">
                            <Filter className="h-4 w-4" />
                            <span className="text-[11px] font-bold uppercase tracking-widest">Filters</span>
                        </div>

                        <FilterSelect
                            value={discipline}
                            onChange={handleDisciplineChange}
                            options={DISCIPLINES}
                        />

                        {leagueOptions && (
                            <FilterSelect
                                value={league}
                                onChange={setLeague}
                                options={leagueOptions}
                            />
                        )}

                        <FilterSelect
                            value={minMargin}
                            onChange={setMinMargin}
                            options={MARGIN_OPTIONS.map((o) => ({
                                value: String(o.value),
                                label: o.label,
                            }))}
                        />
                    </div>

                    {error ? (
                        <ConnectionError
                            error={error}
                            onRetry={() => loadArbs(true)}
                        />
                    ) : (
                        <ArbitrageTable
                            arbs={filteredArbs}
                            isLoading={isLoading}
                        />
                    )}
                </div>
            </main>
        </div>
    );
}
