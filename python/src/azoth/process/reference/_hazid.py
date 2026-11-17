"""The shared internals of the ``process.*`` HAZID worksheets.

The guide-word and parameter orders, the equipment-type seed table and the class's
``TBD`` marker. The Python twin of ``crates/azoth-process/src/models/hazid.rs``; the
arithmetic is NeqSim's ``process/safety/hazid/HAZOPTemplate.java``.
"""

from __future__ import annotations

GUIDE_WORDS = ("no", "more", "less", "reverse", "as_well_as", "part_of", "other_than")
PARAMETERS = ("flow", "pressure", "temperature", "level", "composition", "reaction")

TBD = "TBD"

_SEED_TABLE: tuple[tuple[tuple[str, ...], tuple[tuple[str, str], ...]], ...] = (
    (
        ("separator", "scrubber"),
        (
            ("no", "flow"),
            ("more", "level"),
            ("less", "level"),
            ("more", "pressure"),
            ("less", "pressure"),
        ),
    ),
    (
        ("compressor", "expander"),
        (("no", "flow"), ("reverse", "flow"), ("more", "pressure"), ("more", "temperature")),
    ),
    (("pump",), (("no", "flow"), ("reverse", "flow"), ("more", "pressure"))),
    (("valve",), (("no", "flow"), ("more", "flow"), ("less", "flow"))),
    (
        ("cooler", "heater", "heatexchanger", "reboiler", "condenser"),
        (("more", "temperature"), ("less", "temperature"), ("no", "flow")),
    ),
    (("pipe",), (("no", "flow"), ("less", "flow"), ("less", "pressure"))),
)

_DEFAULT_SEEDS = (
    ("no", "flow"),
    ("more", "flow"),
    ("more", "pressure"),
    ("less", "pressure"),
)


def seed_rows(equipment_type: str) -> tuple[tuple[str, str], ...]:
    """The class's equipment-type seed set, by lower-case substring in its own order."""
    kind = equipment_type.lower()
    for markers, rows in _SEED_TABLE:
        if any(marker in kind for marker in markers):
            return rows
    return _DEFAULT_SEEDS
