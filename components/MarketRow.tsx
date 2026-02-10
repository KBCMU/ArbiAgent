"use client";

import { ArrowUpRight, TrendingUp, Clock } from "lucide-react";
import { Market } from "@/lib/api";
import { cn } from "@/lib/utils";

interface MarketRowProps {
    market: Market;
}

function formatVolume(volume: number): string {
    if (volume >= 1000000) return `$${(volume / 1000000).toFixed(1)}M`;
    if (volume >= 1000) return `$${(volume / 1000).toFixed(1)}K`;
    return `$${volume.toFixed(0)}`;
}

function formatTime(timestamp?: number): string {
    if (!timestamp) return "—";
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    return "Just now";
}

export function MarketRow({ market }: MarketRowProps) {
    const isOpen = market.status === "open";

    return (
        <div className="group relative border-b border-gray-100 bg-white transition-all hover:bg-gray-50 hover:shadow-sm">
            <div className="grid grid-cols-12 gap-4 px-6 py-4">
                {/* Market Title */}
                <div className="col-span-5 flex items-center gap-3">
                    <div className="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-blue-50 to-blue-100">
                        <TrendingUp className="h-5 w-5 text-blue-600" />
                    </div>
                    <div className="min-w-0 flex-1">
                        <h3 className="truncate text-sm font-semibold text-gray-900 group-hover:text-blue-600">
                            {market.title}
                        </h3>
                        <div className="mt-0.5 flex items-center gap-2">
                            {market.tags.slice(0, 2).map((tag) => (
                                <span
                                    key={tag}
                                    className="text-xs text-gray-500"
                                >
                                    {tag}
                                </span>
                            ))}
                        </div>
                    </div>
                </div>

                {/* Side A */}
                <div className="col-span-2 flex flex-col justify-center">
                    <div className="text-xs font-medium text-gray-500">{market.side_a.label}</div>
                    <div className="mt-1 text-lg font-bold text-blue-600">—</div>
                </div>

                {/* Side B */}
                <div className="col-span-2 flex flex-col justify-center">
                    <div className="text-xs font-medium text-gray-500">{market.side_b.label}</div>
                    <div className="mt-1 text-lg font-bold text-orange-600">—</div>
                </div>

                {/* Volume */}
                <div className="col-span-2 flex flex-col justify-center">
                    <div className="text-xs font-medium text-gray-500">Volume</div>
                    <div className="mt-1 text-sm font-semibold text-gray-900">
                        {formatVolume(market.volume_total)}
                    </div>
                </div>

                {/* Status & Time */}
                <div className="col-span-1 flex items-center justify-end gap-2">
                    <div className="flex flex-col items-end gap-1">
                        <span
                            className={cn(
                                "inline-flex items-center rounded-full px-2 py-1 text-xs font-medium",
                                isOpen
                                    ? "bg-blue-50 text-blue-700"
                                    : "bg-gray-100 text-gray-600"
                            )}
                        >
                            {isOpen ? "Open" : "Closed"}
                        </span>
                        <div className="flex items-center gap-1 text-xs text-gray-500">
                            <Clock className="h-3 w-3" />
                            {formatTime(market.end_time)}
                        </div>
                    </div>
                    <ArrowUpRight className="h-4 w-4 text-gray-400 opacity-0 transition-opacity group-hover:opacity-100" />
                </div>
            </div>
        </div>
    );
}
