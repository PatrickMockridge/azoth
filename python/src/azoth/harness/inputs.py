"""A spec's declared inputs, as JSON documents and as call arguments.

The declaration in ``_registry_gen.spec(calc_id)["inputs"]`` is the authority. A
number is read in the spec's declared unit, a ``{value, unit}`` pair is converted to
it, and every other declared kind has one shape: enum and string, boolean, fitting
and name lists, vectors, matrices, record lists and component names.

`kwargs_for` and `model_kwargs` are the same conversion the spec-driven tests use;
they moved here so the harness and the tests cannot disagree about what a case means.
"""

from __future__ import annotations

import importlib
import inspect
import math
from collections.abc import Mapping, Sequence
from typing import Any

import pint

from azoth._dispatch import fluid_inputs
from azoth.core.errors import AzothError
from azoth.core.units import from_si, quantity, to_si, ureg
from azoth.harness.errors import HarnessInputError, UndefinedInput

__all__ = [
    "PARAMETER_RESOLVERS",
    "decode_inputs",
    "function_for",
    "kwargs_for",
    "model_kwargs",
    "parameter_set",
    "parameters_of",
]

_QUANTITY = "quantity"
_LIST_OF_NAMES = frozenset({"fitting_list", "vector_string"})
_MODEL_COMPONENTS_INPUT = "components"
_ASSOCIATING_KEY = "associating"

#: A model's ``params`` argument is annotated with one of these records; the table names
#: the databank function that resolves it and the other declared inputs that function
#: consumes. A record not here is an error rather than a quietly wrong call.
PARAMETER_RESOLVERS: dict[str, tuple[str, tuple[str, ...]]] = {
    "GeNrtlPhaseParameters": ("ge_nrtl_phase_parameters", ()),
    "GeUnifacPhaseParameters": ("ge_unifac_phase_parameters", ()),
    "GeUniquacPhaseParameters": ("ge_uniquac_phase_parameters", ()),
    "GeVanLaarAcidPhaseParameters": ("ge_van_laar_acid_phase_parameters", ()),
    "GeWilsonPhaseParameters": ("ge_wilson_phase_parameters", ()),
    "NrtlParameters": ("nrtl_parameters", ()),
    "UnifacParameters": ("unifac_parameters", ()),
    "UnifacPsrkParameters": ("unifac_psrk_parameters", ()),
    "UnifacUmrpruParameters": ("unifac_umrpru_parameters", ("parameters",)),
    "UniquacParameters": ("uniquac_parameters", ()),
    "VanLaarAcidParameters": ("van_laar_acid_parameters", ()),
}


def _number(value: Any, field: str) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError) as exc:
        raise UndefinedInput(f"{field}: expected a number, got {value!r}", field=field) from exc
    if not math.isfinite(number):
        raise UndefinedInput(f"{field}: {number!r} is not finite", field=field)
    return number


def _quantity_value(declaration: Mapping[str, Any], value: Any, field: str) -> float:
    """One dimensioned value, in the spec's own unit."""
    unit = str(declaration.get("unit", "dimensionless"))
    interval = bool(declaration.get("interval", False))
    if isinstance(value, Mapping):
        if set(value) != {"value", "unit"}:
            raise UndefinedInput(
                f"{field}: a quantity is {{value, unit}}, got keys {sorted(value)}", field=field
            )
        try:
            carried = ureg.Quantity(_number(value["value"], field), str(value["unit"]))
            si = to_si(carried, unit, field, interval=interval)
        except AzothError as exc:
            raise UndefinedInput(str(exc), field=field) from exc
        except (pint.errors.PintError, ValueError) as exc:
            raise UndefinedInput(f"{field}: {exc}", field=field) from exc
        return float(from_si(si, unit).magnitude)
    return _number(value, field)


def _normalise(declaration: Mapping[str, Any], value: Any, field: str) -> Any:
    kind = declaration.get("type", _QUANTITY)
    if kind == _QUANTITY:
        return _quantity_value(declaration, value, field)
    if kind in ("enum", "string"):
        text = str(value)
        allowed = declaration.get("values")
        if allowed is not None and text not in allowed:
            raise UndefinedInput(f"{field}: {text!r} is not one of {list(allowed)}", field=field)
        return text
    if kind == "boolean":
        if not isinstance(value, bool):
            raise UndefinedInput(f"{field}: expected true or false, got {value!r}", field=field)
        return value
    if kind in _LIST_OF_NAMES or kind == "components":
        if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
            raise UndefinedInput(f"{field}: expected a list of names", field=field)
        return [str(item) for item in value]
    if kind == "vector":
        if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
            raise UndefinedInput(f"{field}: expected a vector", field=field)
        return [_quantity_value(declaration, item, field) for item in value]
    if kind == "matrix":
        if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
            raise UndefinedInput(f"{field}: expected a matrix", field=field)
        rows: list[list[float]] = []
        for index, row in enumerate(value):
            if isinstance(row, (str, bytes)) or not isinstance(row, Sequence):
                raise UndefinedInput(f"{field}[{index}]: expected a row", field=field)
            rows.append([_quantity_value(declaration, item, f"{field}[{index}]") for item in row])
        return rows
    if kind == "record_list":
        if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
            raise UndefinedInput(f"{field}: expected a list of records", field=field)
        fields = declaration["fields"]
        records: list[dict[str, Any]] = []
        for index, record in enumerate(value):
            if not isinstance(record, Mapping):
                raise UndefinedInput(f"{field}[{index}]: expected an object", field=field)
            unknown = set(record) - set(fields)
            if unknown:
                raise UndefinedInput(
                    f"{field}[{index}]: fields the spec does not declare: {sorted(unknown)}",
                    field=field,
                )
            normalised: dict[str, Any] = {}
            for name, field_declaration in fields.items():
                if name in record:
                    normalised[name] = _normalise(
                        field_declaration, record[name], f"{field}[{index}].{name}"
                    )
                elif field_declaration.get("optional", False):
                    normalised[name] = None
                else:
                    raise UndefinedInput(
                        f"{field}[{index}]: required field {name!r} is absent", field=field
                    )
            records.append(normalised)
        return records
    raise UndefinedInput(f"{field}: the {kind!r} input kind has no bridge codec", field=field)


def decode_inputs(entry: Mapping[str, Any], document: Mapping[str, Any]) -> dict[str, Any]:
    """A JSON document as the spec-case values the conversion functions take."""
    if not isinstance(document, Mapping):
        raise UndefinedInput("inputs must be a JSON object", field="inputs")
    declared: Mapping[str, Any] = entry["inputs"]
    unknown = set(document) - set(declared)
    if unknown:
        raise UndefinedInput(
            f"{entry['id']}: inputs the spec does not declare: {sorted(unknown)}", field="inputs"
        )
    missing = sorted(
        name
        for name, declaration in declared.items()
        if not declaration.get("optional", False) and name not in document
    )
    if missing:
        raise UndefinedInput(f"{entry['id']}: required inputs absent: {missing}", field="inputs")
    decoded: dict[str, Any] = {}
    for name, value in document.items():
        if value is None and declared[name].get("optional", False):
            decoded[name] = None
            continue
        decoded[name] = _normalise(declared[name], value, name)
    return decoded


def _scalar(unit: str, value: Any) -> Any:
    """One spec number as the argument the implementation takes."""
    return float(value) if unit == "dimensionless" else quantity(float(value), unit)


def _declared(declaration: Mapping[str, Any], value: Any) -> Any:
    """One declared input, at the shape and unit the spec gives it."""
    unit = str(declaration.get("unit", "dimensionless"))
    kind = declaration.get("type", _QUANTITY)
    if kind == "record_list":
        fields = declaration["fields"]
        return [
            {
                name: None if record.get(name) is None else _declared(field, record[name])
                for name, field in fields.items()
            }
            for record in value
        ]
    if kind in _LIST_OF_NAMES:
        return [str(item) for item in value]
    if kind == "vector":
        return [_scalar(unit, item) for item in value]
    if kind == "matrix":
        return [[_scalar(unit, item) for item in row] for row in value]
    if kind in ("enum", "string"):
        return str(value)
    if kind == "boolean":
        return bool(value)
    return _scalar(unit, value)


def kwargs_for(calc: Mapping[str, Any], inputs: Mapping[str, Any]) -> dict[str, Any]:
    """Turn a calc's declared inputs into keyword arguments for the real call."""
    declared = calc["inputs"]
    kwargs: dict[str, Any] = {}
    for name, value in inputs.items():
        declaration = declared[name]
        kind = declaration.get("type", _QUANTITY)
        if kind in _LIST_OF_NAMES or kind == "components":
            kwargs[name] = [str(item) for item in value]
        elif kind in ("enum", "string"):
            kwargs[name] = value
        elif kind == "record_list":
            kwargs[name] = _declared(declaration, value)
        elif kind == "boolean":
            kwargs[name] = bool(value)
        elif kind == "vector" or kind == "matrix":
            kwargs[name] = _declared(declaration, value)
        elif kind == _QUANTITY and declaration.get("unit") != "dimensionless":
            kwargs[name] = quantity(float(value), str(declaration["unit"]))
        else:
            kwargs[name] = float(value)
    return kwargs


def model_kwargs(
    model: Mapping[str, Any],
    inputs: Mapping[str, Any],
    *,
    card: Any = None,
) -> dict[str, Any]:
    """Turn a model's declared inputs into keyword arguments for the real call.

    Distinct from :func:`kwargs_for` because a model's arguments are objects: the spec
    declares `components` and the function takes a `mixture`, and where an enthalpy is
    involved an `ideal_gas` too. A keycard is threaded to `mixture_of` rather than
    applied globally, which is the library's rule.
    """
    from azoth.eos import components as databank

    declared = model["inputs"]
    kwargs: dict[str, Any] = {}
    takes = parameters_of(model)

    for prefix, declared_name in fluid_inputs(model):
        names = inputs.get(declared_name)
        if names is None:
            raise HarnessInputError(
                f"{model['id']} declares {declared_name!r} and the inputs do not state it",
                field=declared_name,
            )
        mixture_key = f"{prefix}mixture"
        if mixture_key in takes:
            fluid, ideal_gas = databank.mixture_of(
                list(names),
                eos=inputs.get(f"{prefix}eos", "pr"),
                associating=bool(inputs.get(f"{prefix}associating", False)),
                card=card,
            )
            kwargs[mixture_key] = fluid
            ideal_gas_key = f"{prefix}ideal_gas"
            if ideal_gas_key in takes:
                kwargs[ideal_gas_key] = ideal_gas
        elif "coeffs" in takes:
            kwargs["coeffs"] = [databank.bwrs_coefficients(name) for name in names]
        elif f"{prefix}components" in takes:
            kwargs[f"{prefix}components"] = list(names)
        elif "params" not in takes:
            if len(names) != 1:
                raise HarnessInputError(
                    f"{model['id']} takes scalar critical constants but the inputs name "
                    f"{len(names)} components",
                    field=declared_name,
                )
            record = databank.entry(names[0])
            kwargs.update(Tc=record.Tc, Pc=record.Pc, omega=record.omega)
        if "params" in takes:
            kwargs["params"] = parameter_set(model, names, inputs)

    fluids = fluid_inputs(model)
    resolved = {name for _, name in fluids}
    for prefix, _ in fluids:
        if f"{prefix}mixture" in takes:
            resolved |= {f"{prefix}{_ASSOCIATING_KEY}", f"{prefix}eos"}
    for name, value in inputs.items():
        if name in resolved or name not in takes:
            continue
        kwargs[name] = _declared(declared[name], value)
    return kwargs


def function_for(entry: Mapping[str, Any]) -> Any:
    """The public function for a registered id, or a refusal naming it."""
    namespace, _, name = str(entry["id"]).partition(".")
    module = importlib.import_module(f"azoth.{namespace}")
    function = getattr(module, name, None)
    if function is None:
        raise HarnessInputError(f"{entry['id']}: azoth.{namespace} has no `{name}`")
    return function


def parameters_of(model: Mapping[str, Any]) -> set[str]:
    """The parameter names a model's function actually takes."""
    function = function_for(model)
    try:
        return set(inspect.signature(function).parameters)
    except (TypeError, ValueError) as exc:
        raise HarnessInputError(
            f"{model['id']}: its callable has no inspectable signature", field=model["id"]
        ) from exc


def parameter_set(model: Mapping[str, Any], names: Sequence[str], inputs: Mapping[str, Any]) -> Any:
    """The resolved parameters a model's ``params`` argument is annotated with."""
    from azoth.eos import components as databank

    function = function_for(model)
    annotation = inspect.signature(function).parameters["params"].annotation
    try:
        resolver, extras = PARAMETER_RESOLVERS[annotation]
        resolve = getattr(databank, resolver)
    except KeyError:
        raise HarnessInputError(
            f"{model['id']}: `params` is annotated {annotation!r}, which is not a "
            f"parameter set this knows how to resolve; known: {sorted(PARAMETER_RESOLVERS)}"
        ) from None
    return resolve(list(names), **{extra: inputs[extra] for extra in extras})
