import Image from "next/image";

interface LogoProps {
    className?: string;
    height?: number;
    variant?: "dark" | "light";
}

export function Logo({ className, height = 32, variant = "dark" }: LogoProps) {
    const src = variant === "light"
        ? "/arbiagent-logo-white.png"
        : "/arbiagent-logo-v2.png";

    const aspectRatio = 1024 / 576;
    const width = Math.round(height * aspectRatio);

    return (
        <Image
            src={src}
            alt="ArbiAgent"
            width={width}
            height={height}
            className={className}
            priority
        />
    );
}
