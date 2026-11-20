"""The key-value evidence lookup the review engines share.

The Python twin of ``crates/azoth-process/src/models/review_evidence.rs``. NeqSim's
``OpenDrainReviewItem`` reads a caller's evidence map by alias: the first **present** key
wins, a string that does not parse as a number yields the default, and the boolean tokens
are ``true/yes/y/1`` and ``false/no/n/0``.
"""

from __future__ import annotations

import math

#: A review item's evidence map. ``None`` is a present key with no value.
Evidence = dict[str, str | list[str] | None]


def parse_java_double(raw: str) -> float | None:
    """Java's ``Double.parseDouble`` on a trimmed string, with the type suffix it accepts."""
    text = raw.strip()
    lower = text.lower()
    if lower == "nan":
        return math.nan
    if lower in ("infinity", "+infinity"):
        return math.inf
    if lower == "-infinity":
        return -math.inf
    if text and text[-1] in "fFdD":
        text = text[:-1]
    try:
        return float(text)
    except ValueError:
        return None


def parse_bool(raw: str) -> bool | None:
    """The class's boolean tokens."""
    text = raw.strip()
    lower = text.lower()
    if lower in ("true", "yes", "y") or text == "1":
        return True
    if lower in ("false", "no", "n") or text == "0":
        return False
    return None


def has(evidence: Evidence, key: str) -> bool:
    """Whether the key is present, whatever its value."""
    return key in evidence


def has_any(evidence: Evidence, keys: tuple[str, ...]) -> bool:
    """``hasAny``, in the class's key order."""
    return any(key in evidence for key in keys)


def get_double(evidence: Evidence, default: float, keys: tuple[str, ...]) -> float:
    """``getDouble``: the first present string that parses, or the default."""
    for key in keys:
        if key not in evidence:
            continue
        value = evidence[key]
        if isinstance(value, str):
            parsed = parse_java_double(value)
            return default if parsed is None else parsed
    return default


def get_bool(evidence: Evidence, keys: tuple[str, ...]) -> bool | None:
    """``getBooleanObject``: the first present string that parses, or ``None``."""
    for key in keys:
        if key not in evidence:
            continue
        value = evidence[key]
        if isinstance(value, str):
            parsed = parse_bool(value)
            if parsed is not None:
                return parsed
    return None


def get_bool_default(evidence: Evidence, default: bool, keys: tuple[str, ...]) -> bool:
    """``getBoolean``: ``getBooleanObject`` with the class's default."""
    value = get_bool(evidence, keys)
    return default if value is None else value


def get_string_list(evidence: Evidence, keys: tuple[str, ...]) -> list[str]:
    """``getStringList``: the first present list, a present string as a singleton, else empty."""
    for key in keys:
        if key not in evidence:
            continue
        value = evidence[key]
        if isinstance(value, list):
            return list(value)
        if isinstance(value, str):
            return [value]
    return []


def contains_ignore_case(text: str, token: str) -> bool:
    """``containsIgnoreCase``."""
    return token.lower() in text.lower()
