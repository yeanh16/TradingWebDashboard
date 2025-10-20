from typing import Dict, List, Optional

from pydantic import BaseModel


class Candle(BaseModel):
    timestamp: str
    open: float
    high: float
    low: float
    close: float
    volume: float


class SymbolInsight(BaseModel):
    requested: str
    resolved_exchange: str
    symbol: str
    summary: str
    indicators: Dict[str, float]
    price_precision: Optional[int] = None


class InsightsResponse(BaseModel):
    interval: str
    insights: List[SymbolInsight]
    overview: str
