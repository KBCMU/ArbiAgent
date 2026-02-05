export interface MarketSide {
    id: string;
    label: string;
}

export interface Market {
    market_slug: string;
    event_slug: string;
    condition_id: string;
    title: string;
    description: string;
    start_time?: number;
    end_time?: number;
    completed_time?: number;
    close_time?: number;
    side_a: MarketSide;
    side_b: MarketSide;
    winning_side?: string;
    status: string;
    tags: string[];
    volume_1_week: number;
    volume_1_month: number;
    volume_1_year: number;
    volume_total: number;
    resolution_source?: string;
    image?: string;
}

export interface PaginationInfo {
    limit: number;
    total: number;
    has_more: boolean;
    pagination_key?: string;
}

export interface MarketsResponse {
    markets: Market[];
    pagination: PaginationInfo;
}

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";

export async function fetchMarkets(params?: {
    limit?: number;
    status?: string;
    tags?: string[];
    search?: string;
}): Promise<MarketsResponse> {
    const queryParams = new URLSearchParams();

    if (params?.limit) queryParams.append("limit", params.limit.toString());
    if (params?.status) queryParams.append("status", params.status);
    if (params?.tags) params.tags.forEach(tag => queryParams.append("tags", tag));
    if (params?.search) queryParams.append("search", params.search);

    const response = await fetch(`${API_BASE_URL}/api/markets?${queryParams}`);

    if (!response.ok) {
        throw new Error("Failed to fetch markets");
    }

    return response.json();
}

export async function searchMarkets(query: string, limit = 20): Promise<MarketsResponse> {
    const response = await fetch(
        `${API_BASE_URL}/api/markets/search?q=${encodeURIComponent(query)}&limit=${limit}`
    );

    if (!response.ok) {
        throw new Error("Failed to search markets");
    }

    return response.json();
}
