"""``process.api537_flare_flame_model`` - the API 537 flare flame.

As NeqSim's ``Api537FlareFlameModel`` computes it. Spec:
``specs/models/process/api537_flare_flame_model.toml``

The Python twin of ``crates/azoth-process/src/models/api537_flare_flame_model.rs``:
Kent's flame length, the tilt, the point-source flux, the sterile zones and the noise.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import Api537FlareFlameModelResult, SterileRadius
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.api537_flare_flame_model"

#: The Stefan-Boltzmann constant, W/(m2 K4).
_SIGMA = 5.670374419e-8
_DEFAULT_STACK_HEIGHT_M = 30.0
_DEFAULT_WIND_SPEED_M_S = 0.0
_DEFAULT_TRANSMISSIVITY = 1.0
_DEFAULT_ACOUSTIC_EFFICIENCY = 3.0e-6


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    heat_release: float
    radiative_power: float
    length: float
    tilt: float
    center_horizontal: float
    center_height: float
    tip_horizontal: float
    tip_height: float
    flux: float
    sterile: list[tuple[float, float]]
    pwl: float
    spl: float
    temperature: float


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    mass_flow: Q,
    heat_of_combustion: float,
    radiative_fraction: float,
    exit_velocity: Q,
    ground_distance: Q,
    sound_distance: Q,
    target_fluxes: list[dict[str, object]],
    stack_height: Q | None = None,
    wind_speed: Q | None = None,
    transmissivity: float | None = None,
    acoustic_efficiency: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    mass = input_to_si(spec, "mass_flow", mass_flow)
    heat = float(heat_of_combustion)
    radiative = float(radiative_fraction)
    exit_v = input_to_si(spec, "exit_velocity", exit_velocity)
    ground = input_to_si(spec, "ground_distance", ground_distance)
    sound = input_to_si(spec, "sound_distance", sound_distance)
    stated_stack = (
        None if stack_height is None else input_to_si(spec, "stack_height", stack_height)
    )
    stated_wind = None if wind_speed is None else input_to_si(spec, "wind_speed", wind_speed)
    stated_tau = None if transmissivity is None else float(transmissivity)
    stated_eta = None if acoustic_efficiency is None else float(acoustic_efficiency)
    targets = input_records_to_si(spec, "target_fluxes", target_fluxes)
    apply_checks(
        checks.on_input,
        {
            "mass_flow": mass,
            "heat_of_combustion": heat,
            "radiative_fraction": radiative,
            "exit_velocity": exit_v,
            "ground_distance": ground,
            "sound_distance": sound,
            "stack_height": stated_stack,
            "wind_speed": stated_wind,
            "transmissivity": stated_tau,
            "acoustic_efficiency": stated_eta,
        }.get,
        warnings,
    )

    _require_finite("mass_flow", mass)
    _require_finite("heat_of_combustion", heat)
    _require_finite("radiative_fraction", radiative)
    _require_finite("exit_velocity", exit_v)
    _require_finite("ground_distance", ground)
    _require_finite("sound_distance", sound)
    stack = _DEFAULT_STACK_HEIGHT_M if stated_stack is None else stated_stack
    _require_finite("stack_height", stack)
    wind = _DEFAULT_WIND_SPEED_M_S if stated_wind is None else stated_wind
    _require_finite("wind_speed", wind)
    tau = _DEFAULT_TRANSMISSIVITY if stated_tau is None else stated_tau
    _require_finite("transmissivity", tau)
    eta = _DEFAULT_ACOUSTIC_EFFICIENCY if stated_eta is None else stated_eta
    _require_finite("acoustic_efficiency", eta)
    for target in targets:
        flux_value = float(target["flux"])
        _require_finite("target_fluxes.flux", flux_value)
        if flux_value <= 0.0:
            raise InvalidInputError(
                "target_fluxes.flux",
                "the class's `sterileZoneRadiusM` refuses a non-positive target flux",
            )

    # The class's clamps, mirrored; the declared ranges make them unreachable.
    wind = max(0.0, wind)
    tau = min(1.0, max(0.0, tau))
    eta = max(0.0, eta)

    heat_release = mass * heat
    radiative_power = radiative * heat_release
    length = 0.0 if heat_release <= 0.0 else 0.00326 * heat_release**0.478
    tilt = math.atan2(wind, exit_v)
    center_horizontal = 0.5 * length * math.sin(tilt)
    center_height = stack + 0.5 * length * math.cos(tilt)
    tip_horizontal = length * math.sin(tilt)
    tip_height = stack + length * math.cos(tilt)
    dx = ground - center_horizontal
    dz = center_height
    r2 = max(1.0e-6, dx * dx + dz * dz)
    flux = tau * radiative_power / (4.0 * math.pi * r2)
    sterile: list[tuple[float, float]] = []
    for target in targets:
        target_flux = float(target["flux"])
        r_sphere = math.sqrt(tau * radiative_power / (4.0 * math.pi * target_flux))
        if r_sphere <= dz:
            radius = 0.0
        else:
            radius = max(0.0, center_horizontal + math.sqrt(r_sphere * r_sphere - dz * dz))
        sterile.append((target_flux, radius))
    acoustic_power = eta * heat_release
    pwl = 0.0 if acoustic_power <= 0.0 else 10.0 * math.log10(acoustic_power / 1.0e-12)
    spl = pwl - 20.0 * math.log10(max(sound, 1.0)) - 11.0
    surface = math.pi * max(length * 0.12, 0.1) * length
    temperature = 0.0 if surface <= 0.0 else (radiative_power / surface / _SIGMA) ** 0.25

    return (
        _Solution(
            heat_release=heat_release,
            radiative_power=radiative_power,
            length=length,
            tilt=tilt,
            center_horizontal=center_horizontal,
            center_height=center_height,
            tip_horizontal=tip_horizontal,
            tip_height=tip_height,
            flux=flux,
            sterile=sterile,
            pwl=pwl,
            spl=spl,
            temperature=temperature,
        ),
        warnings,
    )


def api537_flare_flame_model(
    mass_flow: Q,
    heat_of_combustion: float,
    radiative_fraction: float,
    exit_velocity: Q,
    ground_distance: Q,
    sound_distance: Q,
    target_fluxes: list[dict[str, object]],
    stack_height: Q | None = None,
    wind_speed: Q | None = None,
    transmissivity: float | None = None,
    acoustic_efficiency: float | None = None,
) -> Api537FlareFlameModelResult:
    """Kent's flame length, tilt, flux, sterile zones and noise.

    Args:
        mass_flow: the flare gas mass flow.
        heat_of_combustion: the gas's heat of combustion in J/kg.
        radiative_fraction: the radiated fraction of the heat release.
        exit_velocity: the tip's exit velocity.
        ground_distance: where the incident flux is asked.
        sound_distance: where the sound pressure level is asked.
        target_fluxes: the flux levels, each ``{flux}``; the class's four constants are
            1.58, 4.73, 6.3 and 9.46 kW/m2.
        stack_height: the stack height; absent is 30 m.
        wind_speed: the wind speed; absent is 0.
        transmissivity: the atmospheric transmissivity; absent is 1.
        acoustic_efficiency: the sound fraction of the heat release; absent is ``3e-6``.

    Returns:
        The heat release, the geometry, the ground flux, one sterile radius per target flux,
        the two sound levels and the equivalent flame temperature.

    Raises:
        InvalidInputError: for a negative mass flow, a non-positive heat of combustion or
            exit velocity, a radiative fraction outside ``[0, 1]``, a negative distance,
            stack height or efficiency, a transmissivity outside ``[0, 1]``, a non-positive
            target flux, or a non-finite input.
    """
    solution, warnings = _evaluate(
        mass_flow,
        heat_of_combustion,
        radiative_fraction,
        exit_velocity,
        ground_distance,
        sound_distance,
        target_fluxes,
        stack_height,
        wind_speed,
        transmissivity,
        acoustic_efficiency,
    )
    return Api537FlareFlameModelResult(
        heat_release=from_si(solution.heat_release, "W"),
        radiative_power=from_si(solution.radiative_power, "W"),
        flame_length=from_si(solution.length, "m"),
        flame_tilt=solution.tilt,
        flame_center_horizontal=from_si(solution.center_horizontal, "m"),
        flame_center_height=from_si(solution.center_height, "m"),
        flame_tip_horizontal=from_si(solution.tip_horizontal, "m"),
        flame_tip_height=from_si(solution.tip_height, "m"),
        heat_flux_at_ground_distance=from_si(solution.flux, "W/m**2"),
        sterile_radii=tuple(
            SterileRadius(
                flux=from_si(flux_value, "W/m**2"),
                radius=from_si(radius, "m"),
            )
            for flux_value, radius in solution.sterile
        ),
        sound_power_level=solution.pwl,
        sound_pressure_level=solution.spl,
        equivalent_flame_temperature=from_si(solution.temperature, "K"),
        warnings=tuple(warnings),
    )
