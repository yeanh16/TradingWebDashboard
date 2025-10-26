from google import genai
from google.genai import types
import os
from dotenv import load_dotenv  
import time
from collections import deque
from threading import Lock

load_dotenv()

GEMINI_MODEL = "gemini-2.5-flash"
MAX_OUTPUT_TOKENS = 300
REQUEST_LIMIT = 10
WINDOW_SECONDS = 60
_request_times = deque()
_rate_lock = Lock()

# The client gets the API key from the environment variable `GEMINI_API_KEY`.
client = genai.Client()

grounding_tool = types.Tool(
    google_search=types.GoogleSearch()
)

config = types.GenerateContentConfig(
    tools=[grounding_tool],
    thinking_config=types.ThinkingConfig(thinking_budget=None), # 0 Disables thinking, None lets the model decide
)

def _acquire_rate_slot() -> None:
    while True:
        with _rate_lock:
            now = time.monotonic()

            # drop timestamps older than the window
            while _request_times and now - _request_times[0] >= WINDOW_SECONDS:
                _request_times.popleft()

            if len(_request_times) < REQUEST_LIMIT:
                _request_times.append(now)
                return

            sleep_for = WINDOW_SECONDS - (now - _request_times[0])

        time.sleep(max(sleep_for, 0))


def generate_gemini_market_insight(symbol: str, summary: str, interval: str, limit: int) -> str:
    _acquire_rate_slot()
    prompt = f"Provide a concise market insight for cryptocurrency {symbol} based on the following technical analysis summary from the last {limit} {interval} candles:\n\n{summary}\n\n Research and include up to date relevant market news that has impacted the price action recently. Suggest potential future price movements based on this combined information and advise on possible trading strategies based on the interval suggested. Keep the response under {MAX_OUTPUT_TOKENS} tokens."

    response = client.models.generate_content(
        model=GEMINI_MODEL,
        contents=prompt,
        config=config
    )

    print(f"Gemini response for {symbol}: {response.text}")
    if response.text:
        return response.text
    else:
        return summary