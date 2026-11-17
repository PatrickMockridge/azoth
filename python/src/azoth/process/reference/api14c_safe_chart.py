"""``process.api14c_safe_chart`` - the API 14C SAFE chart.

As NeqSim's ``Api14cSafeChartBuilder`` computes it. Spec:
``specs/models/process/api14c_safe_chart.toml``

The Python twin of ``crates/azoth-process/src/models/api14c_safe_chart.rs``: the
class-name classifier, the required-device table, and the present-versus-required
comparison.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    Api14cSafeChartResult,
    SafeChartItem,
    SafeChartMissingDevice,
)
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._safety_review import (
    API14C_DEVICES,
    API14C_REQUIRED,
    classify_api14c,
)

MODEL_ID = "process.api14c_safe_chart"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def api14c_safe_chart(
    equipment: list[dict[str, object]] | None = None,
    present_devices: list[dict[str, object]] | None = None,
    assume_complete: bool | None = None,
) -> Api14cSafeChartResult:
    """The SAFE chart over the caller's equipment rows.

    Args:
        equipment: the equipment rows, each ``{equipment, unit_class}``.
        present_devices: the declared-present rows, each ``{equipment, device}``. Absent is
            no declarations.
        assume_complete: the class's assuming-complete mode; absent is false.

    Returns:
        One row per classified equipment, the missing-device rows and the two counts.

    Raises:
        InvalidInputError: for a declared device that is not one of the spec's values.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "equipment", equipment or [])
    declared_rows: list[dict[str, Any]] = input_records_to_si(
        spec, "present_devices", present_devices or []
    )

    declared: dict[str, list[str]] = {}
    for index, row in enumerate(declared_rows):
        device = str(row["device"])
        if device not in API14C_DEVICES:
            raise InvalidInputError(
                f"present_devices[{index}].device",
                f"`{device}` is not one of {', '.join(API14C_DEVICES)}",
            )
        devices = declared.setdefault(str(row["equipment"]), [])
        if device not in devices:
            devices.append(device)

    complete_mode = bool(assume_complete)
    items: list[SafeChartItem] = []
    missing_devices: list[SafeChartMissingDevice] = []
    for row in rows:
        name = str(row["equipment"])
        category = classify_api14c(str(row["unit_class"]))
        if category is None:
            continue
        required = API14C_REQUIRED[category]
        present = required if complete_mode else tuple(declared.get(name, ()))
        missing = tuple(device for device in required if device not in present)
        items.append(
            SafeChartItem(
                equipment=name,
                category=category,
                required_count=float(len(required)),
                present_count=float(len(present)),
                missing_count=float(len(missing)),
                complete=not missing,
            )
        )
        missing_devices.extend(
            SafeChartMissingDevice(equipment=name, device=device) for device in missing
        )

    return Api14cSafeChartResult(
        items=tuple(items),
        missing_devices=tuple(missing_devices),
        equipment_count=float(len(items)),
        missing_count=float(len(missing_devices)),
        complete=all(item.complete for item in items),
        warnings=tuple(warnings),
    )
