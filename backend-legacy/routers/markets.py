from fastapi import APIRouter, HTTPException, Query
from typing import Optional, List
from backend.services import get_dome_client
from backend.models import MarketsResponse, Market

router = APIRouter(prefix="/api/markets", tags=["markets"])


@router.get("", response_model=MarketsResponse)
async def get_markets(
    market_slug: Optional[List[str]] = Query(None, description="Filter by market slug(s)"),
    event_slug: Optional[List[str]] = Query(None, description="Filter by event slug(s)"),
    tags: Optional[List[str]] = Query(None, description="Filter by tag(s)"),
    status: Optional[str] = Query("open", description="Filter by status (open/closed)"),
    min_volume: Optional[float] = Query(None, description="Minimum total volume"),
    limit: int = Query(10, ge=1, le=100, description="Number of results"),
    pagination_key: Optional[str] = Query(None, description="Pagination cursor")
):
    """
    Get Polymarket markets with optional filters.
    
    Example:
        GET /api/markets?limit=5&status=open
        GET /api/markets?tags=politics&min_volume=10000
    """
    try:
        client = get_dome_client()
        response = await client.get_polymarket_markets(
            market_slug=market_slug,
            event_slug=event_slug,
            tags=tags,
            status=status,
            min_volume=min_volume,
            limit=limit,
            pagination_key=pagination_key
        )
        return response
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch markets: {str(e)}")


@router.get("/search", response_model=MarketsResponse)
async def search_markets(
    q: str = Query(..., description="Search query"),
    limit: int = Query(20, ge=1, le=100, description="Number of results"),
    status: Optional[str] = Query("open", description="Filter by status")
):
    """
    Search markets by keywords in title and description.
    
    Example:
        GET /api/markets/search?q=bitcoin&limit=10
    """
    try:
        client = get_dome_client()
        response = await client.search_markets(
            query=q,
            limit=limit,
            status=status
        )
        return response
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Search failed: {str(e)}")


@router.get("/{market_slug}", response_model=Market)
async def get_market_by_slug(market_slug: str):
    """
    Get a specific market by its slug.
    
    Example:
        GET /api/markets/will-gavin-newsom-win-the-2028-us-presidential-election
    """
    try:
        client = get_dome_client()
        market = await client.get_market_by_slug(market_slug)
        
        if not market:
            raise HTTPException(status_code=404, detail=f"Market '{market_slug}' not found")
        
        return market
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch market: {str(e)}")


@router.get("/source/kalshi", response_model=MarketsResponse)
async def get_kalshi_markets(
    market_ticker: Optional[List[str]] = Query(None, description="Filter by market ticker(s)"),
    event_ticker: Optional[List[str]] = Query(None, description="Filter by event ticker(s)"),
    status: Optional[str] = Query("open", description="Filter by status (open/closed)"),
    min_volume: Optional[float] = Query(None, description="Minimum total volume"),
    limit: int = Query(10, ge=1, le=100, description="Number of results"),
    cursor: Optional[str] = Query(None, description="Pagination cursor")
):
    """
    Get Kalshi markets with optional filters.
    
    Example:
        GET /api/markets/source/kalshi?limit=10&status=open
    """
    try:
        client = get_dome_client()
        response = await client.get_kalshi_markets(
            market_ticker=market_ticker,
            event_ticker=event_ticker,
            status=status,
            min_volume=min_volume,
            limit=limit,
            cursor=cursor
        )
        return response
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch Kalshi markets: {str(e)}")


@router.get("/source/all", response_model=MarketsResponse)
async def get_all_markets(
    status: Optional[str] = Query("open", description="Filter by status (open/closed)"),
    limit: int = Query(25, ge=1, le=100, description="Number of results per source")
):
    """
    Get markets from both Polymarket and Kalshi.
    
    Example:
        GET /api/markets/source/all?limit=20&status=open
    """
    try:
        client = get_dome_client()
        response = await client.get_all_markets(
            status=status,
            limit=limit
        )
        return response
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch markets: {str(e)}")
