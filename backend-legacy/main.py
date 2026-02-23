from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from dotenv import load_dotenv
from backend.config import get_settings
from backend.routers import markets_router, events_router

# Load environment variables
load_dotenv()

# Get settings
settings = get_settings()

app = FastAPI(
    title=settings.api_title,
    version=settings.api_version
)

# CORS Configuration
origins = [
    "http://localhost:3000",
    "http://127.0.0.1:3000",
]

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(markets_router)
app.include_router(events_router)


@app.get("/")
async def root():
    return {"status": "ok", "message": "ArbiAgent Backend is running"}


@app.get("/health")
async def health_check():
    return {"status": "healthy", "api_key_configured": bool(settings.dome_api_key)}


@app.on_event("startup")
async def startup_event():
    """Validate configuration on startup."""
    print(f"🚀 {settings.api_title} v{settings.api_version}")
    print(f"✅ DomeAPI configured: {bool(settings.dome_api_key)}")


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="0.0.0.0", port=8000, reload=True)

