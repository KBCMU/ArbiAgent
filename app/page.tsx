"use client";

import { useState, useEffect } from "react";
import { Sidebar } from "@/components/Sidebar";
import { Header } from "@/components/Header";
import { FilterBar } from "@/components/FilterBar";
import { MarketTable } from "@/components/MarketTable";
import { fetchMarkets, Market } from "@/lib/api";

export default function DashboardPage() {
  const [markets, setMarkets] = useState<Market[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [activeStatus, setActiveStatus] = useState("open");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadMarkets() {
      setIsLoading(true);
      setError(null);

      try {
        const response = await fetchMarkets({
          limit: 50,
          status: activeStatus === "all" ? undefined : activeStatus,
        });
        setMarkets(response.markets);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load markets");
        console.error("Failed to fetch markets:", err);
      } finally {
        setIsLoading(false);
      }
    }

    loadMarkets();
  }, [activeStatus]);

  return (
    <div className="flex min-h-screen bg-gray-50">
      <Sidebar />

      <main className="flex-1 pl-64">
        <Header />
        <FilterBar
          activeStatus={activeStatus}
          onStatusChange={setActiveStatus}
        />

        <div className="p-8">
          {/* Stats Cards */}
          <div className="mb-6 grid grid-cols-4 gap-4">
            <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
              <div className="text-sm font-medium text-gray-500">Total Markets</div>
              <div className="mt-2 text-2xl font-bold text-gray-900">
                {isLoading ? "—" : markets.length}
              </div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
              <div className="text-sm font-medium text-gray-500">Open Markets</div>
              <div className="mt-2 text-2xl font-bold text-blue-600">
                {isLoading ? "—" : markets.filter(m => m.status === "open").length}
              </div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm">
              <div className="text-sm font-medium text-gray-500">Total Volume</div>
              <div className="mt-2 text-2xl font-bold text-gray-900">
                {isLoading
                  ? "—"
                  : `$${(markets.reduce((sum, m) => sum + m.volume_total, 0) / 1000000).toFixed(1)}M`
                }
              </div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-gradient-to-br from-orange-50 to-orange-100 p-4 shadow-sm">
              <div className="text-sm font-medium text-orange-700">Opportunities</div>
              <div className="mt-2 text-2xl font-bold text-orange-600">0</div>
            </div>
          </div>

          {/* Error State */}
          {error && (
            <div className="mb-6 rounded-lg border border-red-200 bg-red-50 p-4">
              <p className="text-sm font-medium text-red-800">{error}</p>
            </div>
          )}

          {/* Market Table */}
          <MarketTable markets={markets} isLoading={isLoading} />
        </div>
      </main>
    </div>
  );
}
