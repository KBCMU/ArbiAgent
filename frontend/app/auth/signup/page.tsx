"use client";

import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { useRouter } from "next/navigation";
import Link from "next/link";

export default function SignupPage() {
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [confirmPassword, setConfirmPassword] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);
    const router = useRouter();
    const supabase = createClient();

    const handleSignup = async (e: React.FormEvent) => {
        e.preventDefault();
        setError(null);

        if (password !== confirmPassword) {
            setError("Passwords do not match");
            return;
        }

        if (password.length < 6) {
            setError("Password must be at least 6 characters");
            return;
        }

        setLoading(true);

        try {
            const { error } = await supabase.auth.signUp({
                email,
                password,
                options: {
                    emailRedirectTo: `${window.location.origin}/auth/callback`,
                },
            });

            if (error) throw error;

            // Successfully signed up
            router.push("/");
            router.refresh();
        } catch (err) {
            setError(err instanceof Error ? err.message : "Failed to sign up");
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="flex min-h-screen items-center justify-center bg-gray-50 px-4 dark:bg-[#0a0f1a]">
            <div className="w-full max-w-md">
                <div className="rounded-lg border border-gray-200 bg-white p-8 shadow-sm dark:border-white/10 dark:bg-[#111827]">
                    <h1 className="mb-6 text-2xl font-bold text-gray-900 dark:text-white">
                        Create your account
                    </h1>

                    {error && (
                        <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-3 dark:border-red-500/30 dark:bg-red-950/30">
                            <p className="text-sm text-red-800 dark:text-red-300">{error}</p>
                        </div>
                    )}

                    <form onSubmit={handleSignup} className="space-y-4">
                        <div>
                            <label
                                htmlFor="email"
                                className="block text-sm font-medium text-gray-700 dark:text-white/70"
                            >
                                Email
                            </label>
                            <input
                                id="email"
                                type="email"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                required
                                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-white/10 dark:bg-white/5 dark:text-white dark:placeholder-white/40 dark:focus:border-sky-400 dark:focus:ring-sky-400"
                                placeholder="you@example.com"
                            />
                        </div>

                        <div>
                            <label
                                htmlFor="password"
                                className="block text-sm font-medium text-gray-700 dark:text-white/70"
                            >
                                Password
                            </label>
                            <input
                                id="password"
                                type="password"
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                required
                                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-white/10 dark:bg-white/5 dark:text-white dark:placeholder-white/40 dark:focus:border-sky-400 dark:focus:ring-sky-400"
                                placeholder="••••••••"
                            />
                        </div>

                        <div>
                            <label
                                htmlFor="confirmPassword"
                                className="block text-sm font-medium text-gray-700 dark:text-white/70"
                            >
                                Confirm Password
                            </label>
                            <input
                                id="confirmPassword"
                                type="password"
                                value={confirmPassword}
                                onChange={(e) => setConfirmPassword(e.target.value)}
                                required
                                className="mt-1 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-white/10 dark:bg-white/5 dark:text-white dark:placeholder-white/40 dark:focus:border-sky-400 dark:focus:ring-sky-400"
                                placeholder="••••••••"
                            />
                        </div>

                        <button
                            type="submit"
                            disabled={loading}
                            className="w-full rounded-lg bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 dark:focus:ring-offset-[#111827]"
                        >
                            {loading ? "Creating account..." : "Sign up"}
                        </button>
                    </form>

                    <p className="mt-4 text-center text-sm text-gray-600 dark:text-white/70">
                        Already have an account?{" "}
                        <Link
                            href="/auth/login"
                            className="font-medium text-blue-600 hover:text-blue-500 dark:text-sky-400 dark:hover:text-sky-300"
                        >
                            Sign in
                        </Link>
                    </p>
                </div>
            </div>
        </div>
    );
}
