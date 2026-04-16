"use client";

import { useEffect } from "react";

export default function GlobalError({
    error,
    reset,
}: {
    error: Error & { digest?: string };
    reset: () => void;
}) {
    useEffect(() => {
        console.error("GlobalError:", error);
    }, [error]);

    return (
        <html lang="en">
            <body style={{ fontFamily: "system-ui, sans-serif", padding: "2rem", maxWidth: "600px", margin: "0 auto" }}>
                <h1 style={{ fontSize: "1.5rem", marginBottom: "1rem" }}>
                    Something went wrong
                </h1>
                <p style={{ color: "#666", marginBottom: "1.5rem" }}>
                    {error.message || "A client-side exception occurred."}
                </p>
                <button
                    onClick={() => reset()}
                    style={{
                        padding: "0.5rem 1rem",
                        background: "#2563eb",
                        color: "white",
                        border: "none",
                        borderRadius: "6px",
                        cursor: "pointer",
                        fontSize: "0.9rem",
                    }}
                >
                    Try again
                </button>
                <p style={{ marginTop: "1.5rem", fontSize: "0.85rem", color: "#999" }}>
                    Check the browser console for more details.
                </p>
            </body>
        </html>
    );
}
