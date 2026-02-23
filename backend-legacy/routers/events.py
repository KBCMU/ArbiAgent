from fastapi import APIRouter, HTTPException, Query
from typing import Optional
from backend.services import get_dome_client, match_markets, MatchedEventsResponse

router = APIRouter(prefix="/api/events", tags=["events"])


@router.get("/matched", response_model=MatchedEventsResponse)
async def get_matched_events(
    status: Optional[str] = Query("open", description="Filter by status (open/closed)"),
    limit: int = Query(25, ge=1, le=100, description="Number of results per source")
):
    """
    Get matched events from both Kalshi and Polymarket.
    
    Returns canonical events with markets from both platforms grouped together.
    
    Example:
        GET /api/events/matched?limit=20&status=open
    """
    try:
        client = get_dome_client()
        
        # Fetch from both sources
        kalshi_response = await client.get_kalshi_markets(status=status, limit=limit)
        poly_response = await client.get_polymarket_markets(status=status, limit=limit)
        
        # Match markets into events
        events = match_markets(
            kalshi_markets=kalshi_response.markets,
            polymarket_markets=poly_response.markets,
            threshold=0.7
        )
        
        return MatchedEventsResponse(events=events, total=len(events))
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch matched events: {str(e)}")
