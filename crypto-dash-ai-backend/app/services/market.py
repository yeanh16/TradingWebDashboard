from typing import Any, Dict, List, Optional, Sequence, Tuple

import httpx
from fastapi import HTTPException

from ..models.schemas import Candle, SymbolInsight
from ..settings import get_settings
from .analysis import analyse_dataframe, candles_to_dataframe, summarise
from .geminiai import generate_gemini_market_insight

settings = get_settings()
SymbolMetadata = Dict[str, Dict[str, Any]]
_symbol_metadata_cache: SymbolMetadata = {}


async def load_symbol_metadata(client: httpx.AsyncClient) -> SymbolMetadata:
    global _symbol_metadata_cache
    if _symbol_metadata_cache:
        return _symbol_metadata_cache

    url = f"{settings.market_api_base_url.rstrip('/')}/api/symbols"
    try:
        response = await client.get(url, timeout=settings.http_timeout_seconds)
        response.raise_for_status()
    except httpx.HTTPError:
        return _symbol_metadata_cache

    payload = response.json()
    entries: List[Dict[str, Any]] = []
    if isinstance(payload, dict):
        if isinstance(payload.get('exchanges'), list):
            entries = payload['exchanges']
        elif isinstance(payload.get('symbols'), list):
            entries = [payload]

    for entry in entries:
        exchange = entry.get('exchange')
        if not exchange:
            continue
        exchange_key = exchange.lower()
        mapping = _symbol_metadata_cache.setdefault(exchange_key, {})
        for symbol_info in entry.get('symbols', []):
            symbol_code = str(symbol_info.get('symbol') or '').upper()
            if symbol_code:
                mapping[symbol_code] = symbol_info

    return _symbol_metadata_cache


def derive_price_precision(symbol_info: Dict[str, Any]) -> Optional[int]:
    if not symbol_info:
        return None
    precision = symbol_info.get('price_precision')
    if isinstance(precision, str) and precision.isdigit():
        return int(precision)
    if isinstance(precision, int):
        return precision
    tick_size = symbol_info.get('tick_size')
    if tick_size is None:
        return None
    tick_str = str(tick_size)
    if '.' in tick_str:
        decimals = tick_str.split('.')[-1].rstrip('0')
        return len(decimals)
    return None


def _to_float(value: Optional[str]) -> float:
    if value is None or value == "":
        return float("nan")
    try:
        return float(value)
    except (TypeError, ValueError, RuntimeError):
        return float("nan")


def split_symbol(token: str) -> Tuple[str, Optional[str]]:
    cleaned = token.replace("/", "").replace("-", "").upper()
    for quote in settings.quote_priority:
        if cleaned.endswith(quote):
            base = cleaned[: -len(quote)]
            if base:
                return base, quote
    return cleaned, None


async def fetch_candles(
    client: httpx.AsyncClient,
    exchange: str,
    symbol: str,
    interval: str,
    limit: int,
) -> Optional[List[Candle]]:
    url = f"{settings.market_api_base_url.rstrip('/')}/api/candles"
    params = {
        "exchange": exchange,
        "symbol": symbol,
        "interval": interval,
        "limit": str(limit),
    }

    try:
        response = await client.get(url, params=params, timeout=settings.http_timeout_seconds)
        response.raise_for_status()
    except httpx.HTTPStatusError:
        return None
    except httpx.HTTPError as exc:
        raise HTTPException(status_code=502, detail=f"Market API request failed: {exc}")

    payload = response.json()
    result = []
    for row in payload.get("candles", []):
        result.append(
            Candle(
                timestamp=row["timestamp"],
                open=_to_float(row["open"]),
                high=_to_float(row["high"]),
                low=_to_float(row["low"]),
                close=_to_float(row["close"]),
                volume=_to_float(row["volume"]),
            )
        )

    return result or None


async def build_insight(
    client: httpx.AsyncClient,
    requested_token: str,
    interval: str,
    limit: int,
    metadata: SymbolMetadata,
) -> SymbolInsight:
    token = requested_token.strip()
    if not token:
        raise HTTPException(status_code=400, detail="Empty symbol provided")

    if ":" in token:
        preferred_exchange, raw_symbol = token.split(":", 1)
        preferred_exchange = preferred_exchange.lower()
    else:
        preferred_exchange, raw_symbol = None, token

    base, quote = split_symbol(raw_symbol)
    symbol_candidates: List[str] = []
    if quote:
        symbol_candidates.append(base + quote)
    symbol_candidates.extend(base + q for q in settings.quote_priority if q != quote)

    exchanges: List[str] = []
    if preferred_exchange:
        exchanges.append(preferred_exchange)
    exchanges.extend(e for e in settings.exchange_priority if e != preferred_exchange)

    for exchange in exchanges:
        if not exchange:
            continue
        exchange_key = exchange.lower()
        exchange_metadata = metadata.get(exchange_key, {})
        for symbol in symbol_candidates:
            candles = await fetch_candles(client, exchange, symbol, interval, limit)
            if not candles:
                continue
            df = candles_to_dataframe(candles)
            if df.empty:
                continue

            symbol_key = symbol.upper()
            symbol_info = exchange_metadata.get(symbol_key)
            precision = derive_price_precision(symbol_info)

            indicators = analyse_dataframe(df)
            summary = summarise(
                f"{exchange.upper()}:{symbol}",
                indicators,
                price_precision=precision,
            )
            gemini_insight = generate_gemini_market_insight(symbol, summary, interval, limit)
            return SymbolInsight(
                requested=requested_token,
                resolved_exchange=exchange,
                symbol=symbol,
                summary=gemini_insight,
                indicators={k: float(v) for k, v in indicators.items()},
                price_precision=precision,
            )

    raise HTTPException(status_code=404, detail=f"Unable to fetch candles for '{requested_token}'")


async def generate_insights(symbols: Sequence[str], interval: str, limit: int) -> List[SymbolInsight]:
    symbol_tokens = [token.strip() for token in symbols if token.strip()]
    if not symbol_tokens:
        raise HTTPException(status_code=400, detail="No symbols provided")

    async with httpx.AsyncClient(timeout=settings.http_timeout_seconds) as client:
        metadata = await load_symbol_metadata(client)
        insights: List[SymbolInsight] = []
        for token in symbol_tokens:
            insight = await build_insight(client, token, interval, limit, metadata)
            insights.append(insight)

    return insights
