"use client";

import { Market } from "@/lib/api";
import { MarketRow } from "./MarketRow";

interface MarketTableProps {
    markets: Market[];
    isLoading?: boolean;
}

function SkeletonRow() {
    return (
        <div className="border-b border-gray-100 bg-white px-6 py-4">
            <div className="grid grid-cols-12 gap-4">
                <div className="col-span-5 flex items-center gap-3">
                    <div className="h-10 w-10 rounded-lg bg-gray-200 skeleton"></div>
                    <div className="flex-1 space-y-2">
                        <div className="h-4 w-3/4 rounded bg-gray-200 skeleton"></div>
                        <div className="h-3 w-1/2 rounded bg-gray-200 skeleton"></div>
                    </div>
                </div>
                <div className="col-span-2">
                    <div className="h-4 w-16 rounded bg-gray-200 skeleton"></div>
                </div>
                <div className="col-span-2">
                    <div className="h-4 w-16 rounded bg-gray-200 skeleton"></div>
                </div>
                <div className="col-span-2">
                    <div className="h-4 w-20 rounded bg-gray-200 skeleton"></div>
                </div>
                <div className="col-span-1">
                    <div className="h-6 w-16 rounded-full bg-gray-200 skeleton ml-auto"></div>
                </div>
            </div>
        </div>
    );
}

export function MarketTable({ markets, isLoading }: MarketTableProps) {
    return (
        <div className="flex-1 overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
            {/* Table Header */}
            <div className="border-b border-gray-200 bg-gray-50 px-6 py-3">
                <div className="grid grid-cols-12 gap-4">
                    <div className="col-span-5">
                        <span className="text-xs font-semibold uppercase tracking-wide text-gray-600">
                            Market
                        </span>
                    </div>
                    <div className="col-span-2">
                        <span className="text-xs font-semibold uppercase tracking-wide text-gray-600">
                            Side A
                        </span>
                    </div>
                    <div className="col-span-2">
                        <span className="text-xs font-semibold uppercase tracking-wide text-gray-600">
                            Side B
                        </span>
                    </div>
                    <div className="col-span-2">
                        <span className="text-xs font-semibold uppercase tracking-wide text-gray-600">
                            Volume
                        </span>
                    </div>
                    <div className="col-span-1">
                        <span className="text-xs font-semibold uppercase tracking-wide text-gray-600 text-right block">
                            Status
                        </span>
                    </div>
                </div>
            </div>

            {/* Table Body */}
            <div className="max-h-[calc(100vh-280px)] overflow-y-auto">
                {isLoading ? (
                    <>
                        {Array.from({ length: 8 }).map((_, i) => (
                            <SkeletonRow key={i} />
                        ))}
                    </>
                ) : markets.length === 0 ? (
                    <div className="flex h-64 items-center justify-center">
                        <div className="text-center">
                            <p className="text-sm font-medium text-gray-900">No markets found</p>
                            <p className="mt-1 text-sm text-gray-500">Try adjusting your filters</p>
                        </div>
                    </div>
                ) : (
                    markets.map((market) => (
                        <MarketRow key={market.market_slug} market={market} />
                    ))
                )}
            </div>
        </div>
    );
}
