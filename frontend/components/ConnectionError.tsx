"use client";

import { WifiOff, RefreshCw } from "lucide-react";

interface ConnectionErrorProps {
    error: string;
    onRetry?: () => void;
}

export function ConnectionError({ error, onRetry }: ConnectionErrorProps) {
    const isNetworkError =
        error.includes("fetch") ||
        error.includes("network") ||
        error.includes("ECONNREFUSED") ||
        error.includes("Failed to");

    return (
        <div className="rounded-lg border border-red-200 bg-red-50 p-6 dark:border-red-500/30 dark:bg-red-950/50">
            <div className="flex items-start gap-3">
                <WifiOff className="mt-0.5 h-5 w-5 flex-shrink-0 text-red-500 dark:text-red-400" />
                <div className="flex-1">
                    <h3 className="text-sm font-semibold text-red-900 dark:text-red-400">
                        {isNetworkError
                            ? "Cannot connect to backend"
                            : "Error loading data"}
                    </h3>
                    <p className="mt-1 text-sm text-red-700 dark:text-red-400">{error}</p>

                    {isNetworkError && (
                        <div className="mt-3 rounded-md bg-red-100/60 p-3 dark:bg-red-900/30">
                            <p className="text-xs font-medium text-red-800 dark:text-red-400">
                                Troubleshooting:
                            </p>
                            <ul className="mt-1 list-inside list-disc space-y-0.5 text-xs text-red-700 dark:text-red-400">
                                <li>
                                    Make sure the Rust backend is running:{" "}
                                    <code className="rounded bg-red-200/60 px-1 py-0.5 font-mono text-[11px] dark:bg-red-900/40 dark:text-red-400">
                                        cd backend-rust && cargo run
                                    </code>
                                </li>
                                <li>
                                    Backend should be listening on{" "}
                                    <code className="rounded bg-red-200/60 px-1 py-0.5 font-mono text-[11px] dark:bg-red-900/40 dark:text-red-400">
                                        http://localhost:8080
                                    </code>
                                </li>
                                <li>
                                    Check that{" "}
                                    <code className="rounded bg-red-200/60 px-1 py-0.5 font-mono text-[11px] dark:bg-red-900/40 dark:text-red-400">
                                        DOME_API_KEY
                                    </code>{" "}
                                    is set in{" "}
                                    <code className="rounded bg-red-200/60 px-1 py-0.5 font-mono text-[11px] dark:bg-red-900/40 dark:text-red-400">
                                        backend-rust/.env
                                    </code>
                                </li>
                            </ul>
                        </div>
                    )}

                    {onRetry && (
                        <button
                            onClick={onRetry}
                            className="mt-3 inline-flex items-center gap-1.5 rounded-md bg-red-600 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-red-700 dark:bg-red-700 dark:hover:bg-red-600"
                        >
                            <RefreshCw className="h-3 w-3" />
                            Retry now
                        </button>
                    )}
                </div>
            </div>
        </div>
    );
}
