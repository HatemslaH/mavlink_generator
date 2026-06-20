"""Add generated/py to sys.path for mavlink_protocols imports."""

from __future__ import annotations

import sys
from pathlib import Path

_GENERATED = Path(__file__).resolve().parent.parent.parent / "generated" / "py"
if str(_GENERATED) not in sys.path:
    sys.path.insert(0, str(_GENERATED))
