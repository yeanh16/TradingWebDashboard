from typing import List

from fastapi import APIRouter, Query
from fastapi.responses import JSONResponse

from ..models.schemas import InsightsResponse
from ..services.market import generate_insights
from ..settings import get_settings

router = APIRouter(prefix="/insights", tags=["insights"])
settings = get_settings()


@router.options("", include_in_schema=False)
@router.options("/", include_in_schema=False)
async def insights_options() -> JSONResponse:
    return JSONResponse(
        content={},
        headers={
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "GET, OPTIONS",
            "Access-Control-Allow-Headers": "*",
        },
    )


@router.get("", response_model=InsightsResponse)
@router.get("/", response_model=InsightsResponse, include_in_schema=False)
async def get_insights(
    symbols: str = Query(..., description="Comma-separated symbols, optionally prefixed with exchange (e.g. bybit:BTCUSDT)"),
    interval: str = Query(..., description="Candle interval such as 1m,5m,1h,1d"),
    limit: int = Query(settings.default_limit, ge=50, le=1000),
) -> InsightsResponse:
    symbol_tokens: List[str] = [token.strip() for token in symbols.split(",") if token.strip()]
    insights = await generate_insights(symbol_tokens, interval, limit)
    overview = "\n\n".join(entry.summary for entry in insights) if insights else ""
    return InsightsResponse(interval=interval, insights=insights, overview=overview)
