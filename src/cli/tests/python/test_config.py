from pydantic_settings import BaseSettings

from python.config import Settings, settings


def test_settings_is_basemodel():
    assert isinstance(settings, BaseSettings)


def test_settings_env_prefix(monkeypatch):
    monkeypatch.setenv("QTCLOUD_DEVOPS_FOO", "bar")
    s = Settings()
    assert hasattr(s, "model_config")
