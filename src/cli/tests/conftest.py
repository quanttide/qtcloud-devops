import pytest
from pathlib import Path

FIXTURES_DIR = Path(__file__).resolve().parent.parent.parent / "tests" / "fixtures"


@pytest.fixture
def contract_path():
    return FIXTURES_DIR / "contract.yaml"
