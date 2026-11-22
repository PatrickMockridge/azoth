"""The `azoth.process` surface a star import sees.

`__all__` is the half of the module a notebook gets from `from azoth.process import *`, and it
had drifted: forty-four public wrappers were defined and absent, including every relief load the
latest safety tranche added. These tests hold the list to the module in both directions — a
wrapper that is defined is exported, and a name that is exported exists.
"""

from __future__ import annotations

import inspect

import azoth.process as process


def test_every_public_function_the_module_defines_is_exported() -> None:
    defined = {
        name
        for name, value in inspect.getmembers(process, inspect.isfunction)
        if value.__module__ == process.__name__ and not name.startswith("_")
    }
    assert defined - set(process.__all__) == set()


def test_every_exported_name_exists() -> None:
    missing = sorted(name for name in process.__all__ if not hasattr(process, name))
    assert missing == []


def test_the_relief_loads_and_the_protection_study_are_exported() -> None:
    for name in (
        "blocked_outlet_relief",
        "check_valve_leak_relief",
        "control_valve_failure_relief",
        "overpressure_protection_study",
        "tube_rupture_relief",
    ):
        assert name in process.__all__, name
