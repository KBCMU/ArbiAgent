import httpx
from typing import Optional, List
from backend.config import get_settings
from backend.models.market import MarketsResponse, Market


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


# Singleton instance
_dome_client: Optional[DomeAPIClient] = None


def get_dome_client() -> DomeAPIClient:
    """Get or create DomeAPI client instance."""
    global _dome_client
    if _dome_client is None:
        _dome_client = DomeAPIClient()
    return _dome_client
