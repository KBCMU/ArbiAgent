"use client";

import { motion } from "framer-motion";
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
        <div className="inline-flex items-center rounded-lg border border-gray-200 bg-gray-50/80 p-0.5 dark:border-white/10 dark:bg-white/[0.04]">
            {OPTIONS.map((opt) => {
                const active = opt.value === value;
                return (
                    <button
                        key={opt.value}
                        onClick={() => onChange(opt.value)}
                        className={cn(
                            "relative rounded-md px-3.5 py-1 text-xs font-semibold tracking-wide transition-colors duration-200",
                            active
                                ? "text-white"
                                : "text-gray-500 hover:text-gray-900 dark:text-white/45 dark:hover:text-white/70",
                        )}
                    >
                        {active && (
                            <motion.span
                                layoutId="market-type-pill"
                                className="absolute inset-0 rounded-md shadow-sm"
                                style={{ background: "var(--purple-brand)" }}
                                transition={{ type: "spring", stiffness: 400, damping: 32 }}
                            />
                        )}
                        <span className="relative z-10">{opt.label}</span>
                    </button>
                );
            })}
        </div>
    );
}
