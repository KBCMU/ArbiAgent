"use client";

import { Filter, SlidersHorizontal, Search } from "lucide-react";
import { cn } from "@/lib/utils";
import { useState } from "react";

interface FilterBarProps {
    activeStatus: string;
    onStatusChange: (status: string) => void;
}

const statuses = [
    { value: "open", label: "Open Markets" },
    { value: "closed", label: "Closed Markets" },
    { value: "all", label: "All Markets" },
];

export function FilterBar({ activeStatus, onStatusChange }: FilterBarProps) {
    const [searchQuery, setSearchQuery] = useState("");

    return (
        <div className="flex items-center justify-between bg-blue-50 px-8 py-4">
            {/* Search and Status filters */}
            <div className="flex items-center gap-4">
                {/* Search */}
                <div className="relative w-80">
                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
                    <input
                        type="text"
                        placeholder="Search markets..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="w-full rounded-lg border border-gray-200 bg-white py-2 pl-10 pr-4 text-sm text-gray-900 placeholder-gray-500 transition-colors focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                    />
                </div>

                {/* Status filters */}
                <div className="flex items-center gap-2">
                    <Filter className="h-4 w-4 text-gray-500" />
                    <div className="flex gap-2">
                        {statuses.map((status) => (
                            <button
                                key={status.value}
                                onClick={() => onStatusChange(status.value)}
                                className={cn(
                                    "rounded-lg px-4 py-2 text-sm font-medium transition-all",
                                    activeStatus === status.value
                                        ? "bg-blue-500 text-white shadow-sm"
                                        : "bg-gray-100 text-gray-700 hover:bg-gray-200"
                                )}
                            >
                                {status.label}
                            </button>
                        ))}
                    </div>
                </div>
            </div>

            {/* Advanced filters */}
            <button className="flex items-center gap-2 rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50">
                <SlidersHorizontal className="h-4 w-4" />
                More Filters
            </button>
        </div>
    );
}
