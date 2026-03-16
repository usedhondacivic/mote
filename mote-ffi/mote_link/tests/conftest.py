import sys
from unittest.mock import MagicMock

# Mock the compiled Rust extension before link.py is imported so that tests
# can exercise pure-Python logic without requiring a built maturin extension.
sys.modules["mote_link.mote_ffi"] = MagicMock()
