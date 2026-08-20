"""
ForgeCode — shared pytest configuration.

Provides custom markers and reusable fixtures for the test suite.

Usage:
    pytest              # run all tests
    pytest -m unit      # unit tests only
    pytest -m a11y      # accessibility tests only
"""

import pytest


# ── Custom Markers ──────────────────────────────

def pytest_configure(config):
    """Register custom markers."""
    config.addinivalue_line("markers", "unit: Unit tests (fast, no external deps)")
    config.addinivalue_line("markers", "integration: Integration tests (may use network)")
    config.addinivalue_line("markers", "a11y: Accessibility tests")
    config.addinivalue_line("markers", "slow: Slow-running tests")


# ── Fixtures ────────────────────────────────────


@pytest.fixture(scope="session")
def project_root():
    """Return the absolute path to the project root."""
    from pathlib import Path
    return Path(__file__).resolve().parent


@pytest.fixture(scope="session")
def sample_config():
    """Provide a minimal configuration dict for testing."""
    return {
        "version": "0.1.0",
        "debug": False,
        "log_level": "INFO",
    }


@pytest.fixture
def tmp_output(tmp_path):
    """Provide a temporary output directory for test artifacts."""
    output_dir = tmp_path / "output"
    output_dir.mkdir()
    return output_dir
