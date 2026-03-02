"use client";

import {
    createContext,
    useContext,
    useState,
    useEffect,
    useCallback,
    type ReactNode,
} from "react";

type Theme = "light" | "dark";

interface ThemeContextValue {
    theme: Theme;
    toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

const STORAGE_KEY = "arbiagent-theme";

export function ThemeProvider({ children }: { children: ReactNode }) {
    const [theme, setTheme] = useState<Theme>("light");
    const [mounted, setMounted] = useState(false);

    useEffect(() => {
        const stored = localStorage.getItem(STORAGE_KEY) as Theme | null;
        if (stored === "light" || stored === "dark") {
            setTheme(stored);
        }
        setMounted(true);
    }, []);

    useEffect(() => {
        if (!mounted) return;
        const root = document.documentElement;
        if (theme === "dark") {
            root.classList.add("dark");
        } else {
            root.classList.remove("dark");
        }
        localStorage.setItem(STORAGE_KEY, theme);
    }, [theme, mounted]);

    const toggleTheme = useCallback(() => {
        setTheme((prev) => (prev === "light" ? "dark" : "light"));
    }, []);

    return (
        <ThemeContext.Provider value={{ theme, toggleTheme }}>
            {children}
        </ThemeContext.Provider>
    );
}

export function useTheme() {
    const ctx = useContext(ThemeContext);
    if (!ctx) throw new Error("useTheme must be used inside ThemeProvider");
    return ctx;
}
