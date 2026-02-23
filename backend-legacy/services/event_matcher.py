"""
Event matching service for grouping markets from different platforms.
Uses title-based heuristic matching (can be upgraded to LLM-based matching).
"""
from typing import List, Optional, Dict
from difflib import SequenceMatcher
from backend.models.market import Market, MarketsResponse, PaginationInfo
from pydantic import BaseModel


class PlatformMarket(BaseModel):
    """Market data from a specific platform."""
    platform: str
    market_id: str
    outcome: str = "YES"
    yes_price: Optional[float] = None
    no_price: Optional[float] = None
    close_time: Optional[int] = None


class CanonicalEvent(BaseModel):
    """A canonical event with markets from multiple platforms."""
    canonical_event_id: str
    event_title: str
    markets: List[PlatformMarket]
    volume_total: float = 0.0
    status: str = "open"


class MatchedEventsResponse(BaseModel):
    """Response containing matched canonical events."""
    events: List[CanonicalEvent]
    total: int


def normalize_title(title: str) -> str:
    """Normalize title for comparison."""
    # Remove common prefixes/suffixes and normalize whitespace
    title = title.lower().strip()
    # Remove common question words
    for word in ["will ", "would ", "does ", "is ", "are ", "can "]:
        if title.startswith(word):
            title = title[len(word):]
    # Remove trailing question mark
    title = title.rstrip("?")
    return title


def title_similarity(title1: str, title2: str) -> float:
    """Calculate similarity between two titles."""
    norm1 = normalize_title(title1)
    norm2 = normalize_title(title2)
    return SequenceMatcher(None, norm1, norm2).ratio()


def match_markets(
    kalshi_markets: List[Market],
    polymarket_markets: List[Market],
    threshold: float = 0.75
) -> List[CanonicalEvent]:
    """
    Match markets from Kalshi and Polymarket into canonical events.
    
    Uses title similarity matching. Threshold of 0.75 means titles must be
    at least 75% similar to be considered a match.
    """
    events: List[CanonicalEvent] = []
    used_poly_indices: set = set()
    
    for kalshi in kalshi_markets:
        best_match: Optional[Market] = None
        best_score = 0.0
        best_idx = -1
        
        for idx, poly in enumerate(polymarket_markets):
            if idx in used_poly_indices:
                continue
            
            score = title_similarity(kalshi.title, poly.title)
            if score > best_score and score >= threshold:
                best_score = score
                best_match = poly
                best_idx = idx
        
        # Create canonical event
        markets: List[PlatformMarket] = []
        
        # Add Kalshi market
        markets.append(PlatformMarket(
            platform="kalshi",
            market_id=kalshi.ticker or kalshi.market_slug or "",
            yes_price=kalshi.yes_price,
            no_price=kalshi.no_price,
            close_time=kalshi.end_time
        ))
        
        # Add Polymarket match if found
        if best_match and best_idx >= 0:
            used_poly_indices.add(best_idx)
            markets.append(PlatformMarket(
                platform="polymarket",
                market_id=best_match.market_slug or "",
                yes_price=None,  # Polymarket doesn't have direct prices in our model
                no_price=None,
                close_time=best_match.end_time
            ))
        
        # Create event ID from title
        event_id = kalshi.title.lower()
        event_id = "".join(c if c.isalnum() else "_" for c in event_id)
        event_id = "_".join(event_id.split("_")[:6])  # Limit length
        
        events.append(CanonicalEvent(
            canonical_event_id=event_id,
            event_title=kalshi.title,
            markets=markets,
            volume_total=kalshi.volume_total + (best_match.volume_total if best_match else 0),
            status=kalshi.status
        ))
    
    # Add unmatched Polymarket markets as their own events
    for idx, poly in enumerate(polymarket_markets):
        if idx not in used_poly_indices:
            event_id = poly.title.lower()
            event_id = "".join(c if c.isalnum() else "_" for c in event_id)
            event_id = "_".join(event_id.split("_")[:6])
            
            events.append(CanonicalEvent(
                canonical_event_id=event_id,
                event_title=poly.title,
                markets=[PlatformMarket(
                    platform="polymarket",
                    market_id=poly.market_slug or "",
                    yes_price=None,
                    no_price=None,
                    close_time=poly.end_time
                )],
                volume_total=poly.volume_total,
                status=poly.status
            ))
    
    # Sort by volume
    events.sort(key=lambda e: e.volume_total, reverse=True)
    
    return events
