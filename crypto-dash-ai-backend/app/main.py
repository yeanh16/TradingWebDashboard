from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from .api import insights
from .settings import get_settings

settings = get_settings()

app = FastAPI(title='crypto-dash-ai-backend')
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=False,
    allow_methods=['*'],
    allow_headers=['*'],
)
app.include_router(insights.router)
