"""Small deterministic helpers for dependency resolution policy tests."""

from __future__ import annotations


def consolidate_versions(versions: list[str]) -> list[str]:
    """Return unique versions with the highest stable version first."""
    stable = [v for v in versions if not is_pre_release(v)]
    return sorted(set(stable), key=_version_key, reverse=True)


def is_pre_release(version: str) -> bool:
    lowered = version.lower()
    return any(marker in lowered for marker in ("alpha", "beta", "rc", "nightly"))


def select_latest_usable(versions: list[str], yanked: set[str] | None = None) -> str | None:
    """Choose the latest stable, non-yanked version."""
    yanked = yanked or set()
    candidates = [
        version
        for version in versions
        if not is_pre_release(version) and version not in yanked
    ]
    return max(candidates, key=_version_key) if candidates else None


def rollback_allowed(required_checks_green: bool, release_ready: bool) -> bool:
    """A batch may roll back, but never be released with a red required check."""
    return not (release_ready and not required_checks_green)


def _version_key(version: str) -> tuple[int, ...]:
    parts = []
    for piece in version.split("."):
        digits = "".join(ch for ch in piece if ch.isdigit())
        parts.append(int(digits or 0))
    return tuple(parts)
