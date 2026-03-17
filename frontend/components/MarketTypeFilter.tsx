"use client";

import { cn } from "@/lib/utils";
import type { MarketType } from "@/lib/api";

interface MarketTypeFilterProps {
    value: MarketType;
    onChange: (type: MarketType) => void;
}

const OPTIONS: { value: MarketType; label: string }[] = [
    { value: "moneyline", label: "Moneyline" },
    { value: "spread", label: "Spread" },
    { value: "total", label: "Total (O/U)" },
];

export function MarketTypeFilter({ value, onChange }: MarketTypeFilterProps) {
    return (
        <div className="inline-flex items-center rounded-lg border border-white/10 bg-white/[0.04] p-0.5">
            {OPTIONS.map((opt) => {
                const active = opt.value === value;
                return (
                    <button
                        key={opt.value}
                        onClick={() => onChange(opt.value)}
                        className={cn(
                            "relative rounded-md px-3.5 py-1 text-xs font-semibold tracking-wide transition-all duration-200",
                            active
                                ? "text-white shadow-sm"
                                : "text-white/45 hover:text-white/70",
                        )}
                        style={active ? { background: "var(--purple-brand, #6d28d9)" } : undefined}
                    >
                        {opt.label}
                    </button>
                );
            })}
        </div>
    );
}
