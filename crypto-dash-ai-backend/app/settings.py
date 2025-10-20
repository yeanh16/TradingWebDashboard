from functools import lru_cache
from typing import List, Tuple

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    market_api_base_url: str = Field(default='http://localhost:8080', alias='market_data_backend_url')
    exchange_priority: Tuple[str, ...] = ("binance", "bybit")
    quote_priority: Tuple[str, ...] = ("USDT", "USDC", "TUSD", "BUSD", "USD")
    default_limit: int = 500
    http_timeout_seconds: float = 20.0
    cors_origins: List[str] | str = Field(default_factory=lambda: ["http://localhost:3000"])

    model_config = SettingsConfigDict(
        env_prefix='AI_',
        env_file=('.env', '.env.local', '.env.example'),
        env_file_encoding='utf-8',
        case_sensitive=False,
        extra='ignore',
        populate_by_name=True,
    )

    @field_validator('cors_origins', mode='before')
    @classmethod
    def _split_origins(cls, value: List[str] | str) -> List[str]:
        if isinstance(value, str):
            return [item.strip() for item in value.split(',') if item.strip()]
        return value


@lru_cache()
def get_settings() -> Settings:
    return Settings()
