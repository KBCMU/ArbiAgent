"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
    TrendingUp,
    Search,
    BarChart3,
    GraduationCap
} from "lucide-react";
import { cn } from "@/lib/utils";

const navigation = [
    { name: "Arbitrage", href: "/arbitrage", icon: TrendingUp },
    { name: "Markets", href: "/markets", icon: Search },
    { name: "Bet Tracker", href: "/bet-tracker", icon: BarChart3 },
];

export function Sidebar() {
    const pathname = usePathname();

    return (
        <aside className="fixed left-0 top-0 h-screen w-64 bg-blue-200">
            {/* Logo */}
            <div className="flex h-16 items-center px-6">
                <div className="flex items-center gap-2">
                    <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-white">
                        <TrendingUp className="h-5 w-5 text-blue-600" />
                    </div>
                    <span className="text-xl font-bold text-gray-900">ArbiAgent</span>
                </div>
            </div>

            {/* Navigation */}
            <nav className="flex flex-col gap-1 p-4">
                {navigation.map((item) => {
                    const isActive = pathname === item.href;
                    const Icon = item.icon;

                    return (
                        <Link
                            key={item.name}
                            href={item.href}
                            className={cn(
                                "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all",
                                isActive
                                    ? "bg-blue-600 text-white shadow-sm"
                                    : "text-gray-700 hover:bg-blue-100"
                            )}
                        >
                            <Icon className="h-5 w-5" />
                            {item.name}
                        </Link>
                    );
                })}

                <div className="my-2 border-t border-blue-300/50" />

                <Link
                    href="/learn"
                    className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium text-gray-700 transition-all hover:bg-blue-100"
                >
                    <GraduationCap className="h-5 w-5" />
                    Learn
                </Link>
            </nav>

            {/* Curved corner decoration - positioned at top right relative to where sidebar meets header/filterbar */}
            <div className="absolute top-16 -right-4 w-8 h-8 bg-blue-50 rounded-tl-[2rem] translate-x-3" style={{ marginTop: '0' }}></div>
        </aside>
    );
}
