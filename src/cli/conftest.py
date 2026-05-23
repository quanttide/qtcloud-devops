import sys
from pathlib import Path

_app_path = str(Path(__file__).resolve().parent / "app")
if _app_path not in sys.path:
    sys.path.insert(0, _app_path)
