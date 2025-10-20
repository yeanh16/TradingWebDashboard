import math
from typing import Dict, List, Tuple

import numpy as np
import pandas as pd

from ..models.schemas import Candle


def candles_to_dataframe(candles: List[Candle]) -> pd.DataFrame:
    df = pd.DataFrame([c.model_dump() for c in candles])
    df["timestamp"] = pd.to_datetime(df["timestamp"], utc=True)
    numeric_cols = ["open", "high", "low", "close", "volume"]
    df[numeric_cols] = df[numeric_cols].apply(pd.to_numeric, errors="coerce")
    df = df.dropna(subset=["close"]).sort_values("timestamp")
    return df


def ema(series: pd.Series, period: int) -> pd.Series:
    return series.ewm(span=period, adjust=False).mean()


def rsi(series: pd.Series, period: int = 14) -> pd.Series:
    delta = series.diff()
    up = np.where(delta.gt(0), delta, 0.0)
    down = np.where(delta.lt(0), -delta, 0.0)
    roll_up = pd.Series(up, index=series.index).ewm(span=period, adjust=False).mean()
    roll_down = pd.Series(down, index=series.index).ewm(span=period, adjust=False).mean()
    rs = roll_up / roll_down
    return 100 - (100 / (1 + rs))


def macd(series: pd.Series) -> Tuple[pd.Series, pd.Series, pd.Series]:
    macd_line = ema(series, 12) - ema(series, 26)
    signal = macd_line.ewm(span=9, adjust=False).mean()
    hist = macd_line - signal
    return macd_line, signal, hist


def identify_zones(series: pd.Series, lookback: int, tolerance: float, *, mode: str) -> List[float]:
    """Cluster price levels that price revisits within a tolerance."""
    recent = series.tail(lookback)
    if recent.empty:
        return []

    zones: List[Tuple[float, List[pd.Timestamp]]] = []

    for timestamp, price in recent.items():
        merged = False
        updated_zones: List[Tuple[float, List[pd.Timestamp]]] = []

        for zone_price, hits in zones:
            if abs(price - zone_price) <= tolerance * max(zone_price, 1e-8):
                blended = (zone_price * len(hits) + price) / (len(hits) + 1)
                updated_zones.append((blended, hits + [timestamp]))
                merged = True
            else:
                updated_zones.append((zone_price, hits))

        if not zones or not merged:
            updated_zones.append((price, [timestamp]))

        zones = updated_zones

    scored: List[Tuple[float, float]] = []
    for zone_price, hits in zones:
        count = len(hits)
        recency_bonus = max((lookback - recent.index.get_loc(ts) for ts in hits), default=1)
        weight = count * (1 + recency_bonus / max(lookback, 1))
        scored.append((zone_price, weight))

    scored.sort(key=lambda item: (item[1], item[0] if mode == 'support' else -item[0]), reverse=True)
    ordered = [price for price, _ in scored]

    if not ordered:
        fallback = float(recent.min()) if mode == 'support' else float(recent.max())
        return [fallback]

    return ordered


def find_support_resistance(df: pd.DataFrame, window: int = 100, tolerance: float = 0.002) -> Tuple[float, float]:
    if df.empty:
        return float('nan'), float('nan')

    segment = df.dropna(subset=['low', 'high', 'close']).tail(window)
    if segment.empty:
        return float('nan'), float('nan')

    support_candidates = identify_zones(segment['low'], len(segment), tolerance, mode='support')
    resistance_candidates = identify_zones(segment['high'], len(segment), tolerance, mode='resistance')

    current_price = float(segment['close'].iloc[-1])

    support = next((level for level in support_candidates if level <= current_price), None)
    if support is None:
        support = float(segment['low'].min()) if not segment['low'].empty else float('nan')

    resistance = next((level for level in resistance_candidates if level >= current_price), None)
    if resistance is None:
        resistance = float(segment['high'].max()) if not segment['high'].empty else float('nan')

    return float(support), float(resistance)


def analyse_dataframe(df: pd.DataFrame) -> Dict[str, float]:
    ema_fast = ema(df["close"], 20)
    ema_slow = ema(df["close"], 50)
    rsi_series = rsi(df["close"], 14)
    macd_line, signal_line, hist_line = macd(df["close"])
    support, resistance = find_support_resistance(df)

    latest = df.iloc[-1]
    first_close = df["close"].iloc[0]

    return {
        "close": float(latest["close"]),
        "ema_fast": float(ema_fast.iloc[-1]),
        "ema_slow": float(ema_slow.iloc[-1]),
        "rsi": float(rsi_series.iloc[-1]),
        "macd": float(macd_line.iloc[-1]),
        "macd_signal": float(signal_line.iloc[-1]),
        "macd_hist": float(hist_line.iloc[-1]),
        "support": support,
        "resistance": resistance,
        "change_pct": float((latest["close"] - first_close) / first_close * 100),
    }


def summarise(symbol: str, indicators: Dict[str, float], *, price_precision: int | None = None) -> str:
    ema_fast = indicators["ema_fast"]
    ema_slow = indicators["ema_slow"]
    rsi_value = indicators["rsi"]
    macd_hist = indicators["macd_hist"]
    support = indicators.get("support")
    resistance = indicators.get("resistance")

    trend = "bullish" if ema_fast > ema_slow else "bearish"
    rsi_state = (
        "overbought" if rsi_value >= 70 else "oversold" if rsi_value <= 30 else "neutral"
    )
    macd_bias = "strengthening" if macd_hist > 0 else "weakening"

    summary = (
        f"{symbol} momentum looks {trend}: EMA20={ema_fast:.2f} vs EMA50={ema_slow:.2f}. "
        f"RSI sits at {rsi_value:.1f} ({rsi_state}). MACD histogram {macd_hist:.2f} suggests {macd_bias} momentum."
    )

    def format_level(value: float | None) -> str:
        if value is None or not math.isfinite(value):
            return "n/a"
        if price_precision is not None:
            return f"{value:.{price_precision}f}"
        return f"{value:.2f}"

    if support is not None and resistance is not None:
        summary += (
            f" Key levels: support near {format_level(support)}, resistance near {format_level(resistance)}."
        )

    return summary
