"""``pvt.generalized_material_balance`` - the generalized oil and gas material balance.

The Python twin of ``crates/azoth-pvt/src/generalized_material_balance.rs``: the four fits,
the column-scaled least squares with the class's condition gate, and the inventory checks.
The linear algebra is :mod:`azoth.reactions.reference._linalg`, the same Jacobi decomposition
the Rust side carries.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import GeneralizedMaterialBalanceResult
from azoth.core.units import input_to_si, quantity, to_si
from azoth.core.warnings import Warning, WarningCode
from azoth.reactions.reference._linalg import condition_number_2, solve_least_squares

MODEL_ID = "pvt.generalized_material_balance"

_MAX_CONDITION_NUMBER = 1.0e8
_MIN_SPLIT_DETERMINANT = 1.0e-10


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _coefficients(record: dict[str, float]) -> tuple[float, float]:
    if (
        not math.isfinite(record["pressure"])
        or record["pressure"] <= 0.0
        or not math.isfinite(record["bo"])
        or record["bo"] <= 0.0
        or not math.isfinite(record["bg"])
        or record["bg"] <= 0.0
        or not math.isfinite(record["bw"])
        or record["bw"] <= 0.0
        or not math.isfinite(record["rs"])
        or record["rs"] < 0.0
        or not math.isfinite(record["rv"])
        or record["rv"] < 0.0
    ):
        raise InvalidInputError(
            "pvt",
            "the PVT needs a positive finite pressure and formation volume factors and "
            "nonnegative finite Rs and Rv",
        )
    determinant = 1.0 - record["rs"] * record["rv"]
    if determinant <= _MIN_SPLIT_DETERMINANT:
        raise InvalidInputError(
            "rs_rv_split",
            "Rs*Rv must stay below one by more than 1e-10; the split is near critical or invalid",
        )
    a = (record["bo"] - record["rs"] * record["bg"]) / determinant
    b = (record["bg"] - record["rv"] * record["bo"]) / determinant
    if not math.isfinite(a) or not math.isfinite(b):
        raise InvalidInputError("pvt", "an effective formation volume factor is not finite")
    return a, b


def _observe(initial: dict[str, float], observation: dict[str, float]) -> list[float]:
    for name in (
        "net_oil_produced",
        "net_gas_produced",
        "net_water_produced",
        "water_influx",
        "expansion_fraction",
    ):
        if not math.isfinite(observation[name]):
            raise InvalidInputError(name, "the observation must be finite")
    a, b = _coefficients(observation)
    eo = a + initial["rs"] * b - initial["bo"] + initial["bo"] * observation["expansion_fraction"]
    eg = initial["rv"] * a + b - initial["bg"] + initial["bg"] * observation["expansion_fraction"]
    withdrawal = (
        a * observation["net_oil_produced"]
        + b * observation["net_gas_produced"]
        + observation["bw"] * observation["net_water_produced"]
        - observation["water_influx"]
    )
    for name, value in (("oil_expansion", eo), ("gas_expansion", eg), ("withdrawal", withdrawal)):
        if not math.isfinite(value):
            raise InvalidInputError(name, "an observation column is not finite")
    return [
        eo,
        eg,
        withdrawal,
        observation["net_oil_produced"],
        observation["net_gas_produced"],
    ]


def _fit(
    initial: dict[str, float],
    history: list[list[float]],
    mode: int,
    gas_per_oil: float,
) -> tuple[float, float, float, float, float | None, list[float], float, float, tuple[Warning, ...]]:
    columns = 2 if mode == 2 else 1
    count = len(history)
    if count < max(2, columns):
        raise InvalidInputError(
            "observations", "the fit needs at least two non-initial observations"
        )
    matrix = [[0.0] * columns for _ in range(count)]
    response = [0.0] * count
    scale = [0.0] * columns
    response_scale = 0.0
    for index, row in enumerate(history):
        matrix[index][0] = row[1] if mode == 1 else row[0] + gas_per_oil * row[1]
        if mode == 2:
            matrix[index][1] = row[1]
        for column in range(columns):
            if not math.isfinite(matrix[index][column]):
                raise InvalidInputError("observations", "an expansion predictor is not finite")
            scale[column] = max(scale[column], abs(matrix[index][column]))
        response[index] = row[2]
        response_scale = max(response_scale, abs(response[index]))
    if response_scale == 0.0:
        raise InvalidInputError("observations", "the history has no net withdrawal signal")
    for column in range(columns):
        if scale[column] == 0.0:
            raise InvalidInputError(
                "observations", "the history has no expansion signal for a fitted phase"
            )
        for row in matrix:
            row[column] /= scale[column]
    response = [value / response_scale for value in response]
    try:
        condition = condition_number_2(matrix)
    except InvalidInputError as error:
        raise InvalidInputError("observations", f"the scaled matrix: {error}") from error
    if not math.isfinite(condition) or condition > _MAX_CONDITION_NUMBER:
        raise InvalidInputError(
            "observations",
            "the history cannot identify independent oil and gas volumes; the scaled "
            f"condition number is {condition}",
        )
    try:
        fitted = solve_least_squares(matrix, response)
    except InvalidInputError as error:
        raise InvalidInputError("observations", f"the scaled least squares: {error}") from error
    oil = 0.0 if mode == 1 else fitted[0] * response_scale / scale[0]
    if mode == 1:
        gas = fitted[0] * response_scale / scale[0]
    elif mode == 2:
        gas = fitted[1] * response_scale / scale[1]
    else:
        gas = gas_per_oil * oil
    if not math.isfinite(oil) or not math.isfinite(gas):
        raise InvalidInputError("observations", "a fitted phase volume is not finite")
    if oil < 0.0 or gas < 0.0 or oil + gas == 0.0:
        raise InvalidInputError(
            "observations",
            "the fit answered unphysical volumes; check the history, PVT and drive assumptions",
        )
    ooip = oil + initial["rv"] * gas
    ogip = gas + initial["rs"] * oil
    if not math.isfinite(ooip) or not math.isfinite(ogip):
        raise InvalidInputError("observations", "a total in-place volume is not finite")
    residuals: list[float] = []
    for index, row in enumerate(history):
        if row[3] > ooip + 1.0e-10 * max(1.0, ooip) or row[4] > ogip + 1.0e-10 * max(1.0, ogip):
            raise InvalidInputError(
                f"observations[{index}]",
                "the net production exceeds the fitted component inventory at this observation",
            )
        residual = row[2] - oil * row[0] - gas * row[1]
        if not math.isfinite(residual):
            raise InvalidInputError(
                f"observations[{index}]", "the balance residual is not finite"
            )
        residuals.append(residual)
    norm = 0.0
    for value in residuals:
        norm = math.hypot(norm, value)
    rms_residual = norm / math.sqrt(count)
    warnings: tuple[Warning, ...] = ()
    if oil > 0.0:
        gas_cap_ratio: float | None = gas * initial["bg"] / (oil * initial["bo"])
    else:
        gas_cap_ratio = None
        warnings = (
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "the fit has no free oil, so the gas-cap ratio is absent rather than infinite",
            ),
        )
    return (
        oil,
        gas,
        ooip,
        ogip,
        gas_cap_ratio,
        residuals,
        rms_residual,
        condition,
        warnings,
    )


def _row(record: dict[str, Any], spec: dict[str, Any], name: str) -> dict[str, float]:
    fields = spec["inputs"][name]["fields"]
    out: dict[str, float] = {}
    for field, declaration in fields.items():
        unit = declaration.get("unit")
        value = record[field]
        if unit is None or unit == "dimensionless":
            out[field] = (
                float(value)
                if isinstance(value, int | float)
                else to_si(value, "dimensionless", f"{name}.{field}")
            )
        else:
            out[field] = to_si(value, unit, f"{name}.{field}")
    return out


def generalized_material_balance(
    fit: str,
    *,
    gas_cap_ratio: float | None = None,
    initial_pressure: Any = None,
    initial_rs: float | None = None,
    initial_bo: float | None = None,
    initial_bg: float | None = None,
    initial_bw: float | None = None,
    initial_rv: float | None = None,
    observations: Any = None,
) -> GeneralizedMaterialBalanceResult:
    """The generalized material balance over a stated initial PVT and history.

    Returns:
        The free-phase and total in-place volumes, the optional gas-cap ratio, the
        residuals and their RMS, and the scaled condition number.

    Raises:
        InvalidInputError: for an unknown fit, a missing input, a history that is too short,
            a zero withdrawal or expansion signal, an ill-conditioned or singular solve,
            unphysical volumes, or production above the fitted inventory.
    """
    spec = _spec()
    if initial_pressure is None:
        raise InvalidInputError("initial_pressure", "the calculation needs this field")
    initial = {
        "pressure": input_to_si(spec, "initial_pressure", initial_pressure),
        "rs": _required("initial_rs", initial_rs),
        "bo": _required("initial_bo", initial_bo),
        "bg": _required("initial_bg", initial_bg),
        "bw": _required("initial_bw", initial_bw),
        "rv": _required("initial_rv", initial_rv),
    }
    _coefficients(initial)
    if fit == "initially_oil":
        mode, gas_per_oil = 0, 0.0
    elif fit == "initially_gas":
        mode, gas_per_oil = 1, 0.0
    elif fit == "known_gas_cap":
        if gas_cap_ratio is None:
            raise InvalidInputError(
                "gas_cap_ratio", "the known-gas-cap fit needs the stated ratio"
            )
        ratio = float(gas_cap_ratio)
        if not math.isfinite(ratio) or ratio < 0.0:
            raise InvalidInputError(
                "gas_cap_ratio", "the gas-cap ratio must be finite and nonnegative"
            )
        mode, gas_per_oil = 0, ratio * initial["bo"] / initial["bg"]
    elif fit == "oil_and_gas":
        mode, gas_per_oil = 2, 0.0
    else:
        raise InvalidInputError(
            "fit", f"{fit!r} is not one of the generalized material-balance fits"
        )
    if observations is None:
        raise InvalidInputError("observations", "the calculation needs the history")
    history: list[list[float]] = []
    for index, record in enumerate(observations):
        try:
            history.append(_observe(initial, _row(record, spec, "observations")))
        except InvalidInputError as error:
            raise InvalidInputError(f"observations[{index}]", str(error)) from error
    (
        oil,
        gas,
        ooip,
        ogip,
        ratio,
        residuals,
        rms_residual,
        condition,
        warnings,
    ) = _fit(initial, history, mode, gas_per_oil)
    return GeneralizedMaterialBalanceResult(
        free_oil=quantity(oil, "m**3"),
        free_gas=quantity(gas, "m**3"),
        ooip=quantity(ooip, "m**3"),
        ogip=quantity(ogip, "m**3"),
        gas_cap_ratio=ratio,
        residuals=tuple(quantity(value, "m**3") for value in residuals),
        rms_residual=quantity(rms_residual, "m**3"),
        condition_number=condition,
        warnings=warnings,
    )


def _required(name: str, value: float | None) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    return float(value)
