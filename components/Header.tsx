"use client";

import { Bell, User, LogOut } from "lucide-react";
import { useState, useEffect } from "react";
import { createClient } from "@/lib/supabase/client";
import { useRouter } from "next/navigation";
import type { User as SupabaseUser } from "@supabase/supabase-js";

export function Header() {
    const [user, setUser] = useState<SupabaseUser | null>(null);
    const [showDropdown, setShowDropdown] = useState(false);
    const router = useRouter();
    const supabase = createClient();

    useEffect(() => {
        const getUser = async () => {
            const { data: { user } } = await supabase.auth.getUser();
            setUser(user);
        };

        getUser();

        const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
            setUser(session?.user ?? null);
        });

        return () => subscription.unsubscribe();
    }, [supabase.auth]);

    const handleLogout = async () => {
        await supabase.auth.signOut();
        router.push("/auth/login");
        router.refresh();
    };

    return (
        <header className="sticky top-0 z-10 bg-blue-200">
            <div className="flex h-16 items-center justify-between px-8">
                {/* Category Toggles */}
                <div className="flex items-center gap-2">
                    <button className="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-blue-700">
                        Predictions
                    </button>
                    <button className="rounded-lg bg-gray-700 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-gray-600">
                        Betting
                    </button>
                    <button className="rounded-lg bg-gray-700 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-gray-600">
                        Crypto
                    </button>
                </div>

                {/* Right section */}
                <div className="flex items-center gap-4">
                    {/* Notifications */}
                    <button className="relative rounded-lg p-2 text-gray-600 transition-colors hover:bg-gray-100 hover:text-gray-900">
                        <Bell className="h-5 w-5" />
                        <span className="absolute right-1.5 top-1.5 h-2 w-2 rounded-full bg-orange-500"></span>
                    </button>

                    {/* User / Auth */}
                    {user ? (
                        <div className="relative">
                            <button
                                onClick={() => setShowDropdown(!showDropdown)}
                                className="flex items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 py-1.5 transition-colors hover:bg-gray-50"
                            >
                                <div className="flex h-7 w-7 items-center justify-center rounded-full bg-gradient-to-br from-blue-500 to-blue-600">
                                    <User className="h-4 w-4 text-white" />
                                </div>
                                <span className="text-sm font-medium text-gray-700">
                                    {user.email?.split('@')[0]}
                                </span>
                            </button>

                            {showDropdown && (
                                <div className="absolute right-0 mt-2 w-48 rounded-lg border border-gray-200 bg-white shadow-lg">
                                    <div className="p-3 border-b border-gray-100">
                                        <p className="text-sm font-medium text-gray-900 truncate">
                                            {user.email}
                                        </p>
                                    </div>
                                    <button
                                        onClick={handleLogout}
                                        className="flex w-full items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
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
                                onClick={() => router.push('/auth/login')}
                                className="rounded-lg px-4 py-1.5 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-100"
                            >
                                Sign in
                            </button>
                            <button
                                onClick={() => router.push('/auth/signup')}
                                className="rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-blue-700"
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
