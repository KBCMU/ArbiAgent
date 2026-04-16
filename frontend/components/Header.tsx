"use client";

import { User, LogOut, TrendingUp, Bitcoin } from "lucide-react";
import { useState, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { createClient } from "@/lib/supabase/client";
import { usePathname, useRouter } from "next/navigation";
import type { SupabaseClient } from "@supabase/supabase-js";
import type { User as SupabaseUser } from "@supabase/supabase-js";
import { cn } from "@/lib/utils";

export function Header() {
    const pathname = usePathname();
    const activeView = pathname?.startsWith("/crypto") ? "crypto" : "predictions";
    const [user, setUser] = useState<SupabaseUser | null>(null);
    const [showDropdown, setShowDropdown] = useState(false);
    const router = useRouter();

    const supabaseRef = useRef<SupabaseClient | null>(null);
    function getSupabase() {
        if (!supabaseRef.current) {
            supabaseRef.current = createClient();
        }
        return supabaseRef.current;
    }

    useEffect(() => {
        const supabase = getSupabase();

        const fetchUser = async () => {
            const {
                data: { user },
            } = await supabase.auth.getUser();
            setUser(user);
        };

        fetchUser();

        const {
            data: { subscription },
        } = supabase.auth.onAuthStateChange((_event, session) => {
            setUser(session?.user ?? null);
        });

        return () => subscription.unsubscribe();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const handleLogout = async () => {
        const supabase = getSupabase();
        await supabase.auth.signOut();
        router.push("/auth/login");
        router.refresh();
    };

    return (
        <header className="sticky top-0 z-10 border-b border-gray-200 bg-white dark:border-white/10 dark:bg-[#0a0f1a]">
            <div className="flex h-16 items-center justify-between px-8">
                <div className="inline-flex items-center rounded-lg border border-gray-200 bg-gray-50/80 p-0.5 dark:border-white/10 dark:bg-white/[0.04]">
                    {[
                        { key: "predictions", label: "Predictions", icon: TrendingUp, path: "/markets" },
                        { key: "crypto", label: "Crypto", icon: Bitcoin, path: "/crypto" },
                    ].map(({ key, label, icon: Icon, path }) => {
                        const active = activeView === key;
                        return (
                            <button
                                key={key}
                                onClick={() => router.push(path)}
                                className={cn(
                                    "relative flex items-center gap-1.5 rounded-md px-4 py-1.5 text-sm font-medium transition-colors duration-200",
                                    active
                                        ? "text-white"
                                        : "text-gray-600 hover:text-gray-900 dark:text-white/60 dark:hover:text-white/90",
                                )}
                            >
                                {active && (
                                    <motion.span
                                        layoutId="header-view-pill"
                                        className="absolute inset-0 rounded-md shadow-sm"
                                        style={{ background: "var(--purple-brand)" }}
                                        transition={{ type: "spring", stiffness: 400, damping: 32 }}
                                    />
                                )}
                                <Icon className="relative z-10 h-4 w-4" />
                                <span className="relative z-10">{label}</span>
                            </button>
                        );
                    })}
                </div>

                <div className="flex items-center gap-4">
                    {user ? (
                        <div className="relative">
                            <button
                                onClick={() => setShowDropdown(!showDropdown)}
                                className="flex items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 py-1.5 transition-colors hover:bg-gray-50 dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
                            >
                                <div className="flex h-7 w-7 items-center justify-center rounded-full" style={{ background: 'linear-gradient(135deg, var(--purple-brand), var(--cyan-brand))' }}>
                                    <User className="h-4 w-4 text-white" />
                                </div>
                                <span className="text-sm font-medium text-gray-700 dark:text-white/70">
                                    {user.email?.split("@")[0]}
                                </span>
                            </button>

                            {showDropdown && (
                                <div className="absolute right-0 mt-2 w-48 rounded-lg border border-gray-200 bg-white shadow-lg dark:border-white/10 dark:bg-[#111827]">
                                    <div className="border-b border-gray-100 p-3 dark:border-white/10">
                                        <p className="truncate text-sm font-medium text-gray-900 dark:text-white">
                                            {user.email}
                                        </p>
                                    </div>
                                    <button
                                        onClick={handleLogout}
                                        className="flex w-full items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50 dark:text-white/70 dark:hover:bg-white/5"
                                    >
                                        <LogOut className="h-4 w-4" />
                                        Sign out
                                    </button>
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="flex items-center gap-2">
                            <button
                                onClick={() => router.push("/auth/login")}
                                className="rounded-lg px-4 py-1.5 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-100 dark:text-white/70 dark:hover:bg-white/5"
                            >
                                Sign in
                            </button>
                            <button
                                onClick={() => router.push("/auth/signup")}
                                className="rounded-lg px-4 py-1.5 text-sm font-medium text-white transition-colors hover:opacity-90"
                                style={{ background: 'var(--purple-brand)' }}
                            >
                                Sign up
                            </button>
                        </div>
                    )}
                </div>
            </div>
        </header>
    );
}
