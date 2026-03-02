"use client";

import { GraduationCap } from "lucide-react";
import { Sidebar } from "@/components/Sidebar";
import { Header } from "@/components/Header";

export default function LearnPage() {
    return (
        <div className="flex min-h-screen" style={{ background: 'var(--bg-main)' }}>
            <Sidebar />
            <main className="min-w-0 flex-1 overflow-x-hidden pl-48">
                <Header />
                <div className="flex h-[calc(100vh-4rem)] items-center justify-center">
                    <div className="text-center">
                        <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl" style={{ background: 'linear-gradient(135deg, var(--purple-brand), var(--cyan-brand))' }}>
                            <GraduationCap className="h-8 w-8 text-white" />
                        </div>
                        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                            Learn
                        </h1>
                        <p className="mt-2 text-sm text-gray-500 dark:text-white/50">
                            Coming soon
                        </p>
                    </div>
                </div>
            </main>
        </div>
    );
}
