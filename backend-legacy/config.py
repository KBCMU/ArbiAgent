from pydantic_settings import BaseSettings, SettingsConfigDict
from functools import lru_cache


class Settings(BaseSettings):
    """Application settings loaded from environment variables."""
    
    # DomeAPI Configuration
    dome_api_key: str
    dome_api_base_url: str = "https://api.domeapi.io/v1"
    
    # Supabase Configuration (optional for markets-only mode)
    supabase_url: str = ""
    supabase_service_role_key: str = ""
    supabase_jwt_secret: str = ""
    
    # API Configuration
    api_title: str = "ArbiAgent Backend"
    api_version: str = "1.0.0"
    
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore"
    )



@lru_cache()
def get_settings() -> Settings:
    """Get cached settings instance."""
    return Settings()
