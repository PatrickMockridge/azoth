"""``process.mass_balance_leak_detection`` - the mass-balance leak sensitivity.

The Python twin of ``crates/azoth-process/src/models/mass_balance_leak_detection.rs``.
The port source is ``MassBalanceLeakDetector.calculateSensitivity``: the measured
imbalance, the two meters' uncertainty, the linepack's pressure and temperature
uncertainty, and the minimum detectable rate.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import MassBalanceLeakDetectionResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning, WarningCode
from azoth.eos import components as _components
from azoth.process.reference import _fluid_state

MODEL_ID = "process.mass_balance_leak_detection"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(spec: dict[str, Any], field: str, value: Any) -> float:
    """A scalar input the caller may state as a number (already SI) or a quantity."""
    if hasattr(value, "to") or hasattr(value, "magnitude"):
        return input_to_si(spec, field, value)
    return float(value)


def mass_balance_leak_detection(
    components: list[str],
    z: list[float],
    inlet_pressure: Q,
    inlet_temperature: Q,
    inlet_flow: Q,
    outlet_flow: Q,
    eos: str | None = None,
    inlet_density: Q | None = None,
    linepack_volume: Q | None = None,
    flow_measurement_uncertainty_fraction: float | None = None,
    pressure_uncertainty: Q | None = None,
    temperature_uncertainty: float | None = None,
    detection_window: Q | None = None,
    confidence_multiplier: float | None = None,
    inlet_stream_name: str | None = None,
    outlet_stream_name: str | None = None,
) -> MassBalanceLeakDetectionResult:
    """The mass-balance leak sensitivity.

    Raises:
        InvalidInputError: for a non-finite or negative flow, a non-positive pressure,
            temperature or supplied density, or an unknown equation of state.
    """
    spec = _spec()
    warnings: list[Warning] = []
    pressure = input_to_si(spec, "inlet_pressure", inlet_pressure)
    temperature = input_to_si(spec, "inlet_temperature", inlet_temperature)
    inlet = input_to_si(spec, "inlet_flow", inlet_flow)
    outlet = input_to_si(spec, "outlet_flow", outlet_flow)
    density_value = (
        None if inlet_density is None else input_to_si(spec, "inlet_density", inlet_density)
    )
    volume_value = (
        None if linepack_volume is None else input_to_si(spec, "linepack_volume", linepack_volume)
    )
    pressure_delta = (
        None
        if pressure_uncertainty is None
        else input_to_si(spec, "pressure_uncertainty", pressure_uncertainty)
    )
    window_value = (
        None
        if detection_window is None
        else input_to_si(spec, "detection_window", detection_window)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "inlet_pressure": pressure,
            "inlet_temperature": temperature,
            "inlet_density": density_value,
        }.get,
        warnings,
    )
    for name, value in (
        ("inlet_pressure", pressure),
        ("inlet_temperature", temperature),
        ("inlet_flow", inlet),
        ("outlet_flow", outlet),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if pressure <= 0.0 or temperature <= 0.0:
        raise InvalidInputError(
            "inlet_pressure",
            "the inlet state must be a positive absolute pressure and temperature",
        )

    volume = 0.0 if volume_value is None else max(0.0, volume_value)
    fraction = max(
        0.0,
        0.01
        if flow_measurement_uncertainty_fraction is None
        else _scalar(
            spec, "flow_measurement_uncertainty_fraction", flow_measurement_uncertainty_fraction
        ),
    )
    delta_pressure = 0.0 if pressure_delta is None else max(0.0, pressure_delta)
    delta_temperature = max(
        0.0,
        0.0
        if temperature_uncertainty is None
        else _scalar(spec, "temperature_uncertainty", temperature_uncertainty),
    )
    window = 60.0 if window_value is None else max(1.0, window_value)
    confidence = max(
        0.0,
        3.0
        if confidence_multiplier is None
        else _scalar(spec, "confidence_multiplier", confidence_multiplier),
    )

    if density_value is not None:
        if density_value <= 0.0:
            raise InvalidInputError("inlet_density", "a density is positive")
        density: float | None = density_value
    elif volume > 0.0:
        mixture, ideal_gas = _components.mixture_of(
            [str(name) for name in components], eos="pr" if eos is None else eos
        )
        try:
            state = _fluid_state.flash(
                mixture,
                ideal_gas,
                temperature,
                pressure,
                [float(value) for value in z],
                False,
            )
            density = state.bulk_density
        except Exception:
            warnings.append(
                Warning(
                    WarningCode.SOLVER_NOT_CONVERGED,
                    "the inlet density flash failed; the linepack uncertainty is zero",
                    "inlet_density",
                )
            )
            density = None
    else:
        density = None

    measured_imbalance = inlet - outlet
    flow_uncertainty = confidence * math.sqrt((inlet * fraction) ** 2 + (outlet * fraction) ** 2)
    if density is not None and volume > 0.0:
        relative = math.sqrt(
            (delta_pressure / pressure) ** 2 + (delta_temperature / temperature) ** 2
        )
        linepack_uncertainty = confidence * density * volume * relative / window
    else:
        linepack_uncertainty = 0.0
    combined_uncertainty = math.sqrt(flow_uncertainty**2 + linepack_uncertainty**2)
    minimum_detectable_leak_rate = abs(measured_imbalance) + combined_uncertainty
    reference = max(abs(inlet), abs(outlet))
    minimum_detectable_leak_fraction = (
        minimum_detectable_leak_rate / reference if reference > 0.0 else None
    )

    return MassBalanceLeakDetectionResult(
        inlet_stream_name=inlet_stream_name,
        outlet_stream_name=outlet_stream_name,
        inlet_flow=from_si(inlet, "kg/s"),
        outlet_flow=from_si(outlet, "kg/s"),
        measured_imbalance=from_si(measured_imbalance, "kg/s"),
        flow_uncertainty=from_si(flow_uncertainty, "kg/s"),
        linepack_uncertainty=from_si(linepack_uncertainty, "kg/s"),
        combined_uncertainty=from_si(combined_uncertainty, "kg/s"),
        minimum_detectable_leak_rate=from_si(minimum_detectable_leak_rate, "kg/s"),
        minimum_detectable_leak_fraction=minimum_detectable_leak_fraction,
        density=None if density is None else from_si(density, "kg/m**3"),
        detection_window=from_si(window, "s"),
        confidence_multiplier=confidence,
        warnings=tuple(warnings),
    )
