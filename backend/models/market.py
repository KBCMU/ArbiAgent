from pydantic import BaseModel, Field
from typing import Optional, List
from datetime import datetime


class MarketSide(BaseModel):
    """Represents one side/outcome of a prediction market."""
    id: str
    label: str


class Market(BaseModel):
    """Prediction market data from DomeAPI."""
    market_slug: str
    event_slug: str
    condition_id: str
    title: str
    description: str
    
    # Timing
    start_time: Optional[int] = None
    end_time: Optional[int] = None
    completed_time: Optional[int] = None
    close_time: Optional[int] = None
    game_start_time: Optional[str] = None
    
    # Market sides (outcomes)
    side_a: MarketSide
    side_b: MarketSide
    winning_side: Optional[str] = None
    
    # Metadata
    status: str  # "open" or "closed"
    tags: List[str] = Field(default_factory=list)
    
    # Volume metrics
    volume_1_week: float = 0.0
    volume_1_month: float = 0.0
    volume_1_year: float = 0.0
    volume_total: float = 0.0
    
    # Additional fields
    resolution_source: Optional[str] = None
    image: Optional[str] = None
    negative_risk_id: Optional[str] = None
    extra_fields: Optional[dict] = None


class PaginationInfo(BaseModel):
    """Pagination metadata."""
    limit: int
    total: int
    has_more: bool
    pagination_key: Optional[str] = None


class MarketsResponse(BaseModel):
    """Response containing markets and pagination info."""
    markets: List[Market]
    pagination: PaginationInfo
