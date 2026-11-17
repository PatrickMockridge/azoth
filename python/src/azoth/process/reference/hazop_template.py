"""``process.hazop_template`` - the deviation grid a HAZOP node starts from.

As NeqSim's ``HAZOPTemplate.generateGrid`` and the seed table ``fromProcessSystem``
applies. Spec: ``specs/models/process/hazop_template.toml``

The Python twin of ``crates/azoth-process/src/models/hazop_template.rs``: seven guide
words per parameter, or the equipment-type seed set, all with ``TBD`` placeholders.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import HazopTemplateDeviation, HazopTemplateResult
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._hazid import GUIDE_WORDS, PARAMETERS, TBD, seed_rows

MODEL_ID = "process.hazop_template"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def hazop_template(
    grid_parameters: list[dict[str, object]],
    equipment_type: str | None = None,
) -> HazopTemplateResult:
    """The generated and seeded deviation rows.

    Args:
        grid_parameters: the parameters to generate, each ``{parameter}``; an empty list is
            the class's no-argument ``generateGrid()``.
        equipment_type: the unit's simple class name or category, matched by lower-case
            substring; absent runs no seed arm, a stated value (even blank) runs the class's
            selection.

    Returns:
        The grid rows in caller and guide-word order, then the seed rows.

    Raises:
        InvalidInputError: for a grid parameter that is not one of the six spec words.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    deviations: list[HazopTemplateDeviation] = []
    # The class seeds a process-derived node before the caller appends a grid.
    if equipment_type is not None:
        for guide_word, parameter in seed_rows(equipment_type):
            deviations.append(
                HazopTemplateDeviation(
                    guide_word=guide_word,
                    parameter=parameter,
                    cause=TBD,
                    consequence=TBD,
                    safeguard=TBD,
                    recommendation=None,
                )
            )

    rows: list[dict[str, Any]] = input_records_to_si(spec, "grid_parameters", grid_parameters)
    for index, row in enumerate(rows):
        parameter = str(row["parameter"])
        if parameter not in PARAMETERS:
            raise InvalidInputError(
                f"grid_parameters[{index}].parameter",
                f"`{parameter}` is not one of {', '.join(PARAMETERS)}",
            )
        for guide_word in GUIDE_WORDS:
            deviations.append(
                HazopTemplateDeviation(
                    guide_word=guide_word,
                    parameter=parameter,
                    cause=TBD,
                    consequence=TBD,
                    safeguard=TBD,
                    recommendation=None,
                )
            )

    return HazopTemplateResult(
        deviations=tuple(deviations),
        deviation_count=float(len(deviations)),
        warnings=tuple(warnings),
    )
