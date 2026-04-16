"use client";

import { useState, useRef, useEffect } from "react";
import { Filter, LayoutGrid, List, Search, ChevronDown } from "lucide-react";
import { motion } from "framer-motion";
import { cn } from "@/lib/utils";
import type { MarketType } from "@/lib/api";
import { MarketTypeFilter } from "./MarketTypeFilter";
import type { ViewMode } from "./EventTable";

interface FilterBarProps {
    activeStatus: string;
    onStatusChange: (status: string) => void;
    activeCategory: string;
    onCategoryChange: (category: string) => void;
    activeSubCategory: string;
    onSubCategoryChange: (subCategory: string) => void;
    activeMarketType?: MarketType;
    onMarketTypeChange?: (type: MarketType) => void;
    searchQuery: string;
    onSearchChange: (query: string) => void;
    viewMode: ViewMode;
    onViewModeChange: (mode: ViewMode) => void;
}

const statuses = [
    { value: "open", label: "Open Markets" },
    { value: "closed", label: "Closed Markets" },
    { value: "all", label: "All Markets" },
];

const sportLeagues = [
    { value: "all", label: "All Leagues" },
    { value: "nfl", label: "NFL" },
    { value: "nba", label: "NBA" },
    { value: "mlb", label: "MLB" },
    { value: "nhl", label: "NHL" },
    { value: "cfb", label: "CFB" },
    { value: "cbb", label: "CBB" },
    { value: "pga", label: "PGA" },
    { value: "tennis", label: "Tennis" },
];

// Soft tinted pill backgrounds per category — matches the premium-fintech palette
// used by sport tags in EventRow. Kept subtle so the active state (solid brand
// colour) reads as the primary highlight.
const categories: { value: string; label: string; idle: string }[] = [
    { value: "all", label: "All", idle: "bg-gray-50 text-gray-700 hover:bg-gray-100 dark:bg-white/[0.04] dark:text-white/70 dark:hover:bg-white/10" },
    { value: "sports", label: "Sports", idle: "bg-orange-50/60 text-orange-700 hover:bg-orange-100/70 dark:bg-orange-500/10 dark:text-orange-300 dark:hover:bg-orange-500/15" },
    { value: "politics", label: "Politics", idle: "bg-blue-50/60 text-blue-700 hover:bg-blue-100/70 dark:bg-blue-500/10 dark:text-blue-300 dark:hover:bg-blue-500/15" },
    { value: "crypto", label: "Crypto", idle: "bg-yellow-50/70 text-yellow-800 hover:bg-yellow-100/80 dark:bg-yellow-500/10 dark:text-yellow-300 dark:hover:bg-yellow-500/15" },
    { value: "economics", label: "Economics", idle: "bg-amber-50/60 text-amber-700 hover:bg-amber-100/70 dark:bg-amber-500/10 dark:text-amber-300 dark:hover:bg-amber-500/15" },
    { value: "world", label: "World", idle: "bg-teal-50/60 text-teal-700 hover:bg-teal-100/70 dark:bg-teal-500/10 dark:text-teal-300 dark:hover:bg-teal-500/15" },
    { value: "science", label: "Science", idle: "bg-indigo-50/60 text-indigo-700 hover:bg-indigo-100/70 dark:bg-indigo-500/10 dark:text-indigo-300 dark:hover:bg-indigo-500/15" },
    { value: "culture", label: "Culture", idle: "bg-pink-50/60 text-pink-700 hover:bg-pink-100/70 dark:bg-pink-500/10 dark:text-pink-300 dark:hover:bg-pink-500/15" },
];

const SPORTS = new Set(["nfl", "nba", "mlb", "nhl", "cfb", "cbb", "pga", "tennis"]);

export function isSportCategory(cat: string): boolean {
    return SPORTS.has(cat);
}

function FilterSelect({
    value,
    onChange,
    options,
}: {
    value: string;
    onChange: (v: string) => void;
    options: { value: string; label: string }[];
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
                    {options.map((opt) => (
                        <button
                            key={opt.value}
                            onClick={() => { onChange(opt.value); setOpen(false); }}
                            className={`block w-full px-3 py-2 text-left text-sm transition-colors ${opt.value === value
                                ? "bg-blue-50 font-semibold text-blue-700 dark:bg-[rgb(37,99,235)]/20 dark:text-[rgb(59,130,246)]"
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

export function FilterBar({
    activeStatus,
    onStatusChange,
    activeCategory,
    onCategoryChange,
    activeSubCategory,
    onSubCategoryChange,
    activeMarketType,
    onMarketTypeChange,
    searchQuery,
    onSearchChange,
    viewMode,
    onViewModeChange,
}: FilterBarProps) {
    const isShowingSports = activeCategory === "sports" || isSportCategory(activeCategory);
    const isSportsRoot = activeCategory === "sports";
    const currentFilters = isSportsRoot ? sportLeagues : statuses;
    const currentValue = isSportsRoot ? activeSubCategory : activeStatus;
    const handleFilterChange = isSportsRoot ? onSubCategoryChange : onStatusChange;
    return (
        <div className="border-b border-gray-200 bg-white px-8 py-3 dark:border-white/10 dark:bg-[#0a0f1a]">
            {/* Row 1: Search + Status filters */}
            <div className="flex items-center gap-4">
                <div className="relative w-64 shrink-0">
                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400 dark:text-white/40" />
                    <input
                        type="text"
                        placeholder="Search markets..."
                        value={searchQuery}
                        onChange={(e) => onSearchChange(e.target.value)}
                        className="w-full rounded-lg border border-gray-200 bg-gray-50 py-1.5 pl-10 pr-4 text-sm text-gray-900 placeholder-gray-400 transition-colors focus:border-[rgb(37,99,235)] focus:outline-none focus:ring-2 focus:ring-[rgb(37,99,235)]/20 dark:border-white/10 dark:bg-white/5 dark:text-white dark:placeholder-white/40 dark:focus:border-[rgb(37,99,235)]"
                    />
                </div>

                <div className="flex items-center gap-2">
                    <Filter className="h-4 w-4 shrink-0 text-gray-400 dark:text-white/40" />
                    <FilterSelect
                        value={currentValue}
                        onChange={handleFilterChange}
                        options={currentFilters}
                    />
                    {isShowingSports && activeMarketType && onMarketTypeChange && (
                        <div className="ml-2 flex items-center">
                            <div className="mr-4 h-4 w-px bg-gray-200 dark:bg-white/10" />
                            <MarketTypeFilter
                                value={activeMarketType}
                                onChange={onMarketTypeChange}
                            />
                        </div>
                    )}
                </div>

                <div className="ml-auto inline-flex items-center rounded-lg border border-gray-200 bg-gray-50/80 p-0.5 dark:border-white/10 dark:bg-white/[0.04]">
                    {[
                        { key: "card" as ViewMode, icon: LayoutGrid },
                        { key: "row" as ViewMode, icon: List },
                    ].map(({ key, icon: Icon }) => {
                        const active = viewMode === key;
                        return (
                            <button
                                key={key}
                                onClick={() => onViewModeChange(key)}
                                className={cn(
                                    "relative rounded-md p-1.5 transition-colors duration-150",
                                    active
                                        ? "text-white"
                                        : "text-gray-500 hover:text-gray-900 dark:text-white/50 dark:hover:text-white",
                                )}
                                aria-pressed={active}
                            >
                                {active && (
                                    <motion.span
                                        layoutId="view-mode-pill"
                                        className="absolute inset-0 rounded-md shadow-sm"
                                        style={{ background: "var(--purple-brand)" }}
                                        transition={{ type: "spring", stiffness: 400, damping: 32 }}
                                    />
                                )}
                                <Icon className="relative z-10 h-4 w-4" />
                            </button>
                        );
                    })}
                </div>
            </div>

            {/* Row 2: Category filters — tinted pills with sliding active state */}
            <div className="mt-3 flex gap-1.5 overflow-x-auto pb-0.5">
                {categories
                    .filter((cat) => cat.value !== "all")
                    .map((cat) => {
                        const active = activeCategory === cat.value;
                        return (
                            <button
                                key={cat.value}
                                onClick={() => onCategoryChange(cat.value)}
                                className={cn(
                                    "relative shrink-0 rounded-lg px-3 py-1 text-xs font-medium transition-colors duration-150",
                                    active ? "text-white" : cat.idle,
                                )}
                                aria-pressed={active}
                            >
                                {active && (
                                    <motion.span
                                        layoutId="filter-category-pill"
                                        className="absolute inset-0 rounded-lg shadow-sm"
                                        style={{ background: "var(--purple-brand)" }}
                                        transition={{ type: "spring", stiffness: 400, damping: 32 }}
                                    />
                                )}
                                <span className="relative z-10">{cat.label}</span>
                            </button>
                        );
                    })}
            </div>
        </div>
    );
}
