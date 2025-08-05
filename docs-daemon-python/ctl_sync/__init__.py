"""
CTL v3.0 Tensor Sync Daemon (Python)

Fast and flexible Python implementation for rapid CTL language adaptation.
"""

__version__ = "3.0.0"
__author__ = "MAGRAY_CLI Team"

from .core import CtlSync
from .schema import CtlSchema

__all__ = ["CtlSync", "CtlSchema"]