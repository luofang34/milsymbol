"""Paths shared by the explicit, offline PDF conversion command."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "target/standards-drawings"
SOURCES = {}
