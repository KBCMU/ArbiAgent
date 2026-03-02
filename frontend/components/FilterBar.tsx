"use client";

import { useState, useRef, useEffect } from "react";
import { Filter, LayoutGrid, List, Search, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ViewMode } from "./EventTable";

interface FilterBarProps {
    activeStatus: string;
    onStatusChange: (status: string) => void;
    activeCategory: string;
    onCategoryChange: (category: string) => void;
    activeSubCategory: string;
    onSubCategoryChange: (subCategory: string) => void;
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

const categories = [
    { value: "all", label: "All" },
    { value: "sports", label: "Sports" },
    { value: "politics", label: "Politics" },
    { value: "crypto", label: "Crypto" },
    { value: "economics", label: "Economics" },
    { value: "world", label: "World" },
    { value: "science", label: "Science" },
    { value: "culture", label: "Culture" },
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
    searchQuery,
    onSearchChange,
    viewMode,
    onViewModeChange,
}: FilterBarProps) {
    const isSports = activeCategory === "sports";
    const currentFilters = isSports ? sportLeagues : statuses;
    const currentValue = isSports ? activeSubCategory : activeStatus;
    const handleFilterChange = isSports ? onSubCategoryChange : onStatusChange;
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
                </div>

                <div className="ml-auto flex items-center rounded-lg border border-gray-200 bg-white p-0.5 dark:border-white/10 dark:bg-white/5">
                    <button
                        onClick={() => onViewModeChange("card")}
                        className={cn(
                            "rounded-md p-1.5 transition-colors",
                            viewMode === "card"
                                ? "text-white shadow-sm"
                                : "text-gray-500 hover:text-gray-900 dark:text-white/50 dark:hover:text-white"
                        )}
                        style={viewMode === "card" ? { background: 'var(--purple-brand)' } : undefined}
                    >
                        <LayoutGrid className="h-4 w-4" />
                    </button>
                    <button
                        onClick={() => onViewModeChange("row")}
                        className={cn(
                            "rounded-md p-1.5 transition-colors",
                            viewMode === "row"
                                ? "text-white shadow-sm"
                                : "text-gray-500 hover:text-gray-900 dark:text-white/50 dark:hover:text-white"
                        )}
                        style={viewMode === "row" ? { background: 'var(--purple-brand)' } : undefined}
                    >
                        <List className="h-4 w-4" />
                    </button>
                </div>
            </div>

            {/* Row 2: Category filters */}
            <div className="mt-3 flex gap-1.5 overflow-x-auto">
                {categories
                    .filter((cat) => cat.value !== "all")
                    .map((cat) => (
                        <button
                            key={cat.value}
                            onClick={() => onCategoryChange(cat.value)}
                            className={cn(
                                "shrink-0 rounded-lg px-3 py-1 text-xs font-medium transition-all",
                                activeCategory === cat.value
                                    ? "text-white shadow-sm"
                                    : "border border-gray-200 bg-gray-50 text-gray-600 hover:bg-gray-100 dark:border-white/10 dark:bg-white/5 dark:text-white/70 dark:hover:bg-white/5",
                            )}
                            style={activeCategory === cat.value ? { background: '#3b82f6' } : undefined}
                        >
                            {cat.label}
                        </button>
                    ))}
            </div>
        </div>
    );
}
