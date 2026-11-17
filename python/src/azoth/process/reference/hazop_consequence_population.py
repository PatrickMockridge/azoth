"""``process.hazop_consequence_population`` - the consequence catalogue's fill.

As NeqSim's ``HazopConsequenceAutoPopulator.populate`` computes it. Spec:
``specs/models/process/hazop_consequence_population.toml``

The Python twin of ``crates/azoth-process/src/models/hazop_consequence_population.rs``:
the deviation-to-calculation catalogue, the equipment categorisation, the neutral flow
fallback and the placeholder-only fill.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import HazopConsequencePopulationResult, HazopPopulationRow
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._hazid import (
    GUIDE_WORDS,
    PARAMETERS,
    is_placeholder,
    mapping_for,
    recommendation_for,
)

MODEL_ID = "process.hazop_consequence_population"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def hazop_consequence_population(
    deviations: list[dict[str, object]],
    equipment_type: str | None = None,
) -> HazopConsequencePopulationResult:
    """The catalogue-filled deviation rows.

    Args:
        deviations: the deviation rows, each ``{guide_word, parameter, cause, consequence,
            safeguard, recommendation}``.
        equipment_type: the unit's simple class name or category; absent or blank is a
            manual, equipment-neutral node.

    Returns:
        One populated row per input row, and the number of rows a mapping was found for.

    Raises:
        InvalidInputError: for a guide word or parameter that is not one of the spec's words.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "deviations", deviations)
    populated: list[HazopPopulationRow] = []
    mapped_count = 0
    for index, row in enumerate(rows):
        guide_word = str(row["guide_word"])
        parameter = str(row["parameter"])
        if guide_word not in GUIDE_WORDS:
            raise InvalidInputError(
                f"deviations[{index}].guide_word",
                f"`{guide_word}` is not one of {', '.join(GUIDE_WORDS)}",
            )
        if parameter not in PARAMETERS:
            raise InvalidInputError(
                f"deviations[{index}].parameter",
                f"`{parameter}` is not one of {', '.join(PARAMETERS)}",
            )
        cause = str(row["cause"])
        consequence = str(row["consequence"])
        safeguard = str(row["safeguard"])
        recommendation = None if row.get("recommendation") is None else str(row["recommendation"])
        chosen = mapping_for(guide_word, parameter, equipment_type)
        if chosen is None:
            populated.append(
                HazopPopulationRow(
                    guide_word=guide_word,
                    parameter=parameter,
                    cause=cause,
                    consequence=consequence,
                    safeguard=safeguard,
                    recommendation=recommendation,
                )
            )
            continue
        mapped_count += 1
        populated.append(
            HazopPopulationRow(
                guide_word=guide_word,
                parameter=parameter,
                cause=cause,
                consequence=(
                    str(chosen["consequence"]) if is_placeholder(consequence) else consequence
                ),
                safeguard=(str(chosen["safeguard"]) if is_placeholder(safeguard) else safeguard),
                recommendation=(
                    recommendation_for(chosen) if is_placeholder(recommendation) else recommendation
                ),
            )
        )

    return HazopConsequencePopulationResult(
        populated=tuple(populated),
        mapped_count=float(mapped_count),
        warnings=tuple(warnings),
    )
