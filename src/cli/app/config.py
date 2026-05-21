from pathlib import Path
from typing import Optional

from pydantic import Field, field_validator, model_validator
from pydantic_settings import BaseSettings, PydanticBaseSettingsSource


class Settings(BaseSettings):
    model_config = {"env_prefix": "QTCLOUD_DEVOPS_"}

    contract_path: Optional[Path] = None

    @field_validator("contract_path", mode="before")
    @classmethod
    def _empty_str_to_none(cls, v):
        if isinstance(v, str) and v.strip() == "":
            return None
        return v

    @model_validator(mode="after")
    def _default_paths(self):
        if self.contract_path is None:
            self.contract_path = Path.cwd() / "contract.yaml"
        return self

    @classmethod
    def settings_customise_sources(
        cls,
        settings_cls: type[BaseSettings],
        init_settings: PydanticBaseSettingsSource,
        env_settings: PydanticBaseSettingsSource,
        dotenv_settings: PydanticBaseSettingsSource,
        file_secret_settings: PydanticBaseSettingsSource,
    ) -> tuple[PydanticBaseSettingsSource, ...]:
        return (init_settings, env_settings, dotenv_settings, file_secret_settings)


settings = Settings()
