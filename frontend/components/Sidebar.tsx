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
import { Logo } from "@/components/Logo";

const navigation = [
    { name: "Arbitrage", href: "/arbitrage", icon: TrendingUp },
    { name: "Markets", href: "/markets", icon: Search },
    { name: "Bet Tracker", href: "/bet-tracker", icon: BarChart3 },
];

export function Sidebar() {
    const pathname = usePathname();

    return (
        <aside className="fixed left-0 top-0 h-screen w-48 border-r border-gray-200 bg-white dark:border-white/10 dark:bg-[#0d1321]">
            <div className="flex h-16 items-center justify-center overflow-hidden">
                <Logo height={96} className="-my-4" />
            </div>

            <nav className="flex flex-col gap-1 pt-0 px-4 pb-4">
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
                                    ? "border-l-4 bg-[rgb(37,99,235)]/10 text-[rgb(37,99,235)] font-semibold dark:border-sky-400 dark:bg-sky-500/15 dark:text-sky-400"
                                    : "text-gray-600 hover:bg-gray-100 hover:text-gray-900 dark:text-white/70 dark:hover:bg-white/5 dark:hover:text-white"
                            )}
                            style={isActive ? { borderColor: 'var(--purple-brand,rgb(37,99,235))' } : undefined}
                        >
                            <Icon className={cn("h-5 w-5 shrink-0", isActive ? "text-[rgb(37,99,235)] dark:text-sky-400" : "")} />
                            {item.name}
                        </Link>
                    );
                })}

                <div className="my-2 border-t border-gray-200 dark:border-white/10" />

                <Link
                    href="/learn"
                    className="flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium text-gray-600 transition-all hover:bg-gray-100 hover:text-gray-900 dark:text-white/70 dark:hover:bg-white/5 dark:hover:text-white"
                >
                    <GraduationCap className="h-5 w-5" />
                    Learn
                </Link>
            </nav>
        </aside>
    );
}
