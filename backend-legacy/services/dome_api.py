import httpx
from typing import Optional, List
from backend.config import get_settings
from backend.models.market import MarketsResponse, Market, PaginationInfo


class DomeAPIClient:
    """Client for interacting with DomeAPI."""
    
    def __init__(self):
        self.settings = get_settings()
        self.base_url = self.settings.dome_api_base_url
        self.api_key = self.settings.dome_api_key
        self.headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        # Simple in-memory cache: {cache_key: (data, timestamp)}
        self._cache = {}
        self._cache_ttl = 30  # seconds

    def _get_cache_key(self, prefix: str, **kwargs) -> str:
        """Generate a deterministic cache key from arguments."""
        # Sort kwargs to ensure consistent keys
        sorted_items = sorted([(k, v) for k, v in kwargs.items() if v is not None])
        key_parts = [f"{k}={v}" for k, v in sorted_items]
        return f"{prefix}:{':'.join(key_parts)}"

    def _get_cached(self, key: str) -> Optional[dict]:
        """Get data from cache if valid."""
        import time
        if key in self._cache:
            data, timestamp = self._cache[key]
            if time.time() - timestamp < self._cache_ttl:
                return data
            else:
                del self._cache[key]
        return None

    def _set_cache(self, key: str, data: dict):
        """Set data in cache."""
        import time
        self._cache[key] = (data, time.time())
    
    async def get_polymarket_markets(
        self,
        market_slug: Optional[List[str]] = None,
        event_slug: Optional[List[str]] = None,
        tags: Optional[List[str]] = None,
        search: Optional[str] = None,
        status: Optional[str] = "open",
        min_volume: Optional[float] = None,
        limit: int = 10,
        pagination_key: Optional[str] = None
    ) -> MarketsResponse:
        """
        Fetch Polymarket markets with optional filters.
        
        Args:
            market_slug: Filter by market slug(s)
            event_slug: Filter by event slug(s)
            tags: Filter by tag(s)
            search: Search keywords in title/description
            status: Filter by status ("open" or "closed")
            min_volume: Minimum total volume
            limit: Number of results (1-100)
            pagination_key: Pagination cursor
            
        Returns:
            MarketsResponse with markets and pagination info
        """
        # Generate cache key
        cache_key = self._get_cache_key(
            "polymarket",
            market_slug=str(market_slug),
            event_slug=str(event_slug),
            tags=str(tags),
            search=search,
            status=status,
            min_volume=min_volume,
            limit=limit,
            pagination_key=pagination_key
        )
        
        # Check cache
        cached_data = self._get_cached(cache_key)
        if cached_data:
            return MarketsResponse(**cached_data)

        params = {}
        
        if market_slug:
            params["market_slug"] = market_slug
        if event_slug:
            params["event_slug"] = event_slug
        if tags:
            params["tags"] = tags
        if search:
            params["search"] = search
        if status:
            params["status"] = status
        if min_volume is not None:
            params["min_volume"] = min_volume
        if limit:
            params["limit"] = limit
        if pagination_key:
            params["pagination_key"] = pagination_key
        
        async with httpx.AsyncClient() as client:
            response = await client.get(
                f"{self.base_url}/polymarket/markets",
                headers=self.headers,
                params=params,
                timeout=30.0
            )
            response.raise_for_status()
            data = response.json()
            
            # Cache the raw data dict
            self._set_cache(cache_key, data)
            
            return MarketsResponse(**data)
    
    async def get_market_by_slug(self, market_slug: str) -> Optional[Market]:
        """
        Get a specific market by its slug.
        
        Args:
            market_slug: The market slug to fetch
            
        Returns:
            Market object or None if not found
        """
        response = await self.get_polymarket_markets(
            market_slug=[market_slug],
            limit=1
        )
        
        if response.markets:
            return response.markets[0]
        return None
    
    async def search_markets(
        self,
        query: str,
        limit: int = 20,
        status: Optional[str] = "open"
    ) -> MarketsResponse:
        """
        Search markets by keywords.
        
        Args:
            query: Search query
            limit: Number of results
            status: Filter by status
            
        Returns:
            MarketsResponse with matching markets
        """
        return await self.get_polymarket_markets(
            search=query,
            limit=limit,
            status=status
        )
    
    async def get_kalshi_markets(
        self,
        market_ticker: Optional[List[str]] = None,
        event_ticker: Optional[List[str]] = None,
        status: Optional[str] = "open",
        min_volume: Optional[float] = None,
        limit: int = 10,
        cursor: Optional[str] = None
    ) -> MarketsResponse:
        """
        Fetch Kalshi markets with optional filters.
        
        Args:
            market_ticker: Filter by market ticker(s)
            event_ticker: Filter by event ticker(s)
            status: Filter by status ("open" or "closed")
            min_volume: Minimum total volume
            limit: Number of results (1-100)
            cursor: Pagination cursor
            
        Returns:
            MarketsResponse with Kalshi markets
        """
        # Generate cache key
        cache_key = self._get_cache_key(
            "kalshi",
            market_ticker=str(market_ticker),
            event_ticker=str(event_ticker),
            status=status,
            min_volume=min_volume,
            limit=limit,
            cursor=cursor
        )
        
        # Check cache
        cached_data = self._get_cached(cache_key)
        raw_json_data = None
        
        if cached_data:
            raw_json_data = cached_data
        else:
            params = {}
            
            if market_ticker:
                params["market_ticker"] = market_ticker
            if event_ticker:
                params["event_ticker"] = event_ticker
            if status:
                params["status"] = status
            if min_volume is not None:
                params["min_volume"] = min_volume
            if limit:
                params["limit"] = limit
            if cursor:
                params["cursor"] = cursor
            
            async with httpx.AsyncClient() as client:
                response = await client.get(
                    f"{self.base_url}/kalshi/markets",
                    headers=self.headers,
                    params=params,
                    timeout=30.0
                )
                response.raise_for_status()
                raw_json_data = response.json()
                
                # Cache the raw response
                self._set_cache(cache_key, raw_json_data)
        
        # Transform Kalshi response to match our Market model
        markets = []
        raw_markets = raw_json_data.get("markets", [])
        for m in raw_markets:
                market = Market(
                    source="kalshi",
                    ticker=m.get("ticker"),
                    event_ticker=m.get("event_ticker"),
                    title=m.get("title", ""),
                    description=m.get("subtitle", "") or m.get("description", ""),
                    status=m.get("status", "open"),
                    yes_price=m.get("yes_bid"),
                    no_price=m.get("no_bid"),
                    volume_total=m.get("volume", 0),
                    end_time=m.get("close_time"),
                    image=m.get("image_url"),
                )
                markets.append(market)
            
        pagination = PaginationInfo(
            limit=limit,
            total=len(markets),
            has_more=raw_json_data.get("cursor") is not None,
            pagination_key=raw_json_data.get("cursor")
        )
        
        return MarketsResponse(markets=markets, pagination=pagination)
    
    async def get_all_markets(
        self,
        status: Optional[str] = "open",
        limit: int = 25
    ) -> MarketsResponse:
        """
        Fetch markets from both Polymarket and Kalshi.
        
        Args:
            status: Filter by status
            limit: Number of results per source
            
        Returns:
            Combined MarketsResponse from both sources
        """
        import asyncio
        
        # Fetch from both sources concurrently
        poly_task = self.get_polymarket_markets(status=status, limit=limit)
        kalshi_task = self.get_kalshi_markets(status=status, limit=limit)
        
        poly_response, kalshi_response = await asyncio.gather(
            poly_task, kalshi_task, return_exceptions=True
        )
        
        all_markets = []
        
        # Add Polymarket markets (with source tag)
        if isinstance(poly_response, MarketsResponse):
            for m in poly_response.markets:
                m.source = "polymarket"
            all_markets.extend(poly_response.markets)
        
        # Add Kalshi markets
        if isinstance(kalshi_response, MarketsResponse):
            all_markets.extend(kalshi_response.markets)
        
        return MarketsResponse(
            markets=all_markets,
            pagination=PaginationInfo(
                limit=limit * 2,
                total=len(all_markets),
                has_more=False
            )
        )


# Singleton instance
_dome_client: Optional[DomeAPIClient] = None


def get_dome_client() -> DomeAPIClient:
    """Get or create DomeAPI client instance."""
    global _dome_client
    if _dome_client is None:
        _dome_client = DomeAPIClient()
    return _dome_client
