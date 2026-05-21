from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    model_config = {"env_prefix": "QTCLOUD_DEVOPS_"}


settings = Settings()
