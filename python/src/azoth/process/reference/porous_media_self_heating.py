"""``process.porous_media_self_heating`` - the Frank-Kamenetskii criticality screen.

As NeqSim's ``PorousMediaSelfHeatingAnalyzer.analyze`` computes it. Spec:
``specs/models/process/porous_media_self_heating.toml``

The Python twin of ``crates/azoth-process/src/models/porous_media_self_heating.rs``.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PorousMediaSelfHeatingResult, SelfHeatingGeometry, SelfHeatingVerdict
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._selfheating import (
    BRACKET_WARNING,
    DEFAULT_MARGINAL_RATIO,
    FK_WARNING,
    GEOMETRIES,
    PIPE_INSULATION_WARNING,
    R_GAS,
    critical_temperature,
    delta_crit_of,
    fk_critical_dimension,
    fk_delta,
    verdict_of,
)

MODEL_ID = "process.porous_media_self_heating"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def porous_media_self_heating(
    geometry: str | None,
    characteristic_dimension: Q,
    effective_conductivity: Q,
    activation_energy: Q,
    volumetric_pre_factor: float,
    boundary_temperature: Q,
    marginal_ratio: float | None = None,
    pipe_insulation_screening: bool | None = None,
) -> PorousMediaSelfHeatingResult:
    """The Frank-Kamenetskii criticality screen for a porous body.

    Raises:
        InvalidInputError: for an unknown geometry, a non-positive dimension, conductivity,
            activation energy, pre-factor or boundary temperature, or a marginal ratio
            outside (0, 1).
    """
    spec = _spec()
    warnings: list[Warning] = []
    dimension = input_to_si(spec, "characteristic_dimension", characteristic_dimension)
    conductivity = input_to_si(spec, "effective_conductivity", effective_conductivity)
    energy = input_to_si(spec, "activation_energy", activation_energy)
    pre_factor = float(volumetric_pre_factor)
    boundary = input_to_si(spec, "boundary_temperature", boundary_temperature)
    apply_checks(
        checks_for(spec).on_input,
        {
            "characteristic_dimension": dimension,
            "effective_conductivity": conductivity,
            "activation_energy": energy,
            "volumetric_pre_factor": pre_factor,
            "boundary_temperature": boundary,
            "marginal_ratio": None if marginal_ratio is None else float(marginal_ratio),
        }.get,
        warnings,
    )

    shape = "slab" if geometry is None else str(geometry)
    if shape not in GEOMETRIES:
        raise InvalidInputError(
            "geometry",
            f"`{shape}` is not `slab`, `infinite_cylinder`, `sphere`, `cube` or `equicylinder`",
        )
    insulation = bool(pipe_insulation_screening)
    if insulation:
        shape = "slab"
    marginal = DEFAULT_MARGINAL_RATIO if marginal_ratio is None else float(marginal_ratio)
    delta_crit = delta_crit_of(shape)
    delta = fk_delta(energy, pre_factor, dimension, conductivity, boundary)
    delta_ratio = delta / delta_crit
    critical = critical_temperature(
        lambda temperature: (
            fk_delta(energy, pre_factor, dimension, conductivity, temperature) - delta_crit
        )
    )
    critical_dimension = fk_critical_dimension(
        energy, pre_factor, conductivity, boundary, delta_crit
    )

    messages = [] if not insulation else [PIPE_INSULATION_WARNING]
    if critical is None:
        messages.append(BRACKET_WARNING)
    messages.append(FK_WARNING)
    return PorousMediaSelfHeatingResult(
        geometry=SelfHeatingGeometry(shape),
        characteristic_dimension=from_si(dimension, "m"),
        effective_conductivity=from_si(conductivity, "W/(m*K)"),
        activation_energy=from_si(energy, "J/mol"),
        volumetric_pre_factor=pre_factor,
        boundary_temperature=from_si(boundary, "K"),
        delta=delta,
        delta_crit=delta_crit,
        delta_ratio=delta_ratio,
        critical_temperature=None if critical is None else from_si(critical, "K"),
        critical_dimension=from_si(critical_dimension, "m"),
        temperature_margin=None if critical is None else from_si(critical - boundary, "K"),
        dimension_margin=from_si(critical_dimension - dimension, "m"),
        fk_temperature_scale=from_si(R_GAS * boundary * boundary / energy, "K"),
        verdict=SelfHeatingVerdict(verdict_of(delta_ratio, marginal)),
        messages=tuple(messages),
        warnings=tuple(warnings),
    )
