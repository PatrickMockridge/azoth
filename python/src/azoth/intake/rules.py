"""The three licensed inputs an assessor may construct without asking.

A rule is a constructor, not a heuristic: it names what it may infer, the
evidence it requires, and the request a use of it raises. The set is closed on
purpose — a fourth rule is a code change reviewed as one, not a string an assessor
can write. No rule guesses a typical value, and none reads a prose default out of
a spec description.
"""

from __future__ import annotations

import math
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any

from azoth.intake.dossier import Claim, Dossier
from azoth.intake.units import UnitGap, normalize

COMPOSITION_REMAINDER = "composition_remainder"
DERIVED_BY_ID = "derived_by_id"
SPEC_DEFAULT = "spec_default"

_MAX_DEPTH = 4
_REMAINDER_LOW = 1.0e-6
_REMAINDER_HIGH = 0.2


class RuleError(Exception):
    """A rule whose preconditions do not hold. A hard gap, never a guess."""

    def __init__(self, code: str, detail: str, request: str) -> None:
        super().__init__(detail)
        self.code = code
        self.detail = detail
        self.request = request


@dataclass(frozen=True, slots=True)
class RuleValue:
    """What a rule constructed, and the request its use raises."""

    value: Any
    detail: str
    request: str
    provenance: Mapping[str, Any] | None = None


@dataclass(frozen=True, slots=True)
class RuleContext:
    """Everything a rule may read. It cannot see the target's other bindings."""

    target_id: str
    input_name: str
    declaration: Mapping[str, Any]
    dossier: Dossier
    components: tuple[str, ...] | None
    depth: int = 0
    derive: Callable[[str, str, Mapping[str, Any], int], tuple[Any, Mapping[str, Any]]] | None = (
        None
    )


@dataclass(frozen=True, slots=True)
class Rule:
    id: str
    licence: str
    citation: str
    apply: Callable[[Mapping[str, Any], RuleContext], RuleValue]


def _claim(dossier: Dossier, basis: Mapping[str, Any], key: str) -> Claim:
    claim_id = basis.get(key)
    if not isinstance(claim_id, str):
        raise RuleError(
            "RULE_PRECONDITION",
            f"the rule's basis needs `{key}` as a claim id",
            f"name the claim that carries `{key}`",
        )
    return dossier.claim(claim_id)


def _numbers(value: object, field: str) -> list[float]:
    if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
        raise RuleError(
            "RULE_PRECONDITION", f"{field} is not a vector", f"provide {field} as a vector"
        )
    out: list[float] = []
    for index, item in enumerate(value):
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise RuleError(
                "RULE_PRECONDITION",
                f"{field}[{index}] is not a number",
                f"provide {field} as numbers",
            )
        number = float(item)
        if not math.isfinite(number) or number < 0.0:
            raise RuleError(
                "RULE_PRECONDITION",
                f"{field}[{index}] is not a finite nonnegative fraction",
                f"provide {field} as nonnegative fractions",
            )
        out.append(number)
    return out


def _remainder(basis: Mapping[str, Any], context: RuleContext) -> RuleValue:
    """The balance of a composition, assigned to the component the source calls unreported."""
    if context.components is None:
        raise RuleError(
            "RULE_PRECONDITION",
            f"{context.target_id} declares no `components` input to order the composition",
            "provide the component list",
        )
    component = basis.get("component")
    if not isinstance(component, str) or not component:
        raise RuleError(
            "RULE_PRECONDITION",
            "the rule's basis needs `component`",
            "name the unreported component",
        )
    if component not in context.components:
        raise RuleError(
            "RULE_PRECONDITION",
            f"{component!r} is not one of the target's components",
            f"add {component} to the component list or name the component the source reports",
        )
    composition = _claim(context.dossier, basis, "claim")
    raw = _numbers(composition.value, "composition")
    try:
        partial = [
            normalize(
                value,
                composition.unit,
                str(context.declaration.get("unit", "dimensionless")),
                "composition",
                expect_mole=True,
            ).value
            for value in raw
        ]
    except UnitGap as exc:
        raise RuleError(exc.code, exc.detail, exc.request) from exc
    unreported = _claim(context.dossier, basis, "unreported")
    if component.lower() not in unreported.quote.lower():
        raise RuleError(
            "RULE_PRECONDITION",
            f"the cited absence does not name {component!r}",
            f"cite the source sentence that says {component} is not reported",
        )
    if len(partial) != len(context.components) - 1:
        raise RuleError(
            "RULE_PRECONDITION",
            f"the reported composition has {len(partial)} entries; the component list has "
            f"{len(context.components)} and one is unreported",
            f"provide one fraction for every component except {component}",
        )
    total = float(basis.get("total", 1.0))
    shortfall = total - sum(partial)
    if not math.isfinite(total) or not _REMAINDER_LOW <= shortfall <= _REMAINDER_HIGH:
        raise RuleError(
            "COMPOSITION_NOT_CLOSED",
            f"the reported fractions sum to {sum(partial):.10g}; the balance is {shortfall:.10g}, "
            f"outside the licensed {_REMAINDER_LOW:g}..{_REMAINDER_HIGH:g} band",
            f"provide the {component} fraction or the full analysis",
        )
    index = context.components.index(component)
    value = [*partial[:index], shortfall, *partial[index:]]
    return RuleValue(
        value=value,
        detail=(
            f"{component} = {shortfall:.10g} assigned from the balance; the source names "
            f"{component} as not reported"
        ),
        request=(
            f"Confirm the {component} fraction ({shortfall:.10g}) or provide the measured value."
        ),
    )


def _derive(basis: Mapping[str, Any], context: RuleContext) -> RuleValue:
    """An input constructed by another registered id, with that result's provenance."""
    nested_id = basis.get("id")
    output = basis.get("output")
    inputs = basis.get("inputs")
    if not isinstance(nested_id, str) or not nested_id:
        raise RuleError("RULE_PRECONDITION", "the rule's basis needs `id`", "name the calculation")
    if not isinstance(output, str) or not output:
        raise RuleError("RULE_PRECONDITION", "the rule's basis needs `output`", "name the output")
    if not isinstance(inputs, Mapping):
        raise RuleError(
            "RULE_PRECONDITION", "the rule's basis needs `inputs`", "bind the derivation's inputs"
        )
    if context.derive is None:
        raise RuleError("RULE_PRECONDITION", "no derivation callback", "internal error")
    if context.depth >= _MAX_DEPTH:
        raise RuleError(
            "RULE_PRECONDITION",
            f"the derivation chain is deeper than {_MAX_DEPTH}",
            "derive the input in fewer steps",
        )
    value, provenance = context.derive(nested_id, output, inputs, context.depth + 1)
    if isinstance(value, Mapping) and set(value) == {"magnitude", "unit"}:
        try:
            converted = normalize(
                value["magnitude"],
                str(value["unit"]),
                str(context.declaration.get("unit", "dimensionless")),
                context.input_name,
            )
        except UnitGap as exc:
            raise RuleError(exc.code, exc.detail, exc.request) from exc
        value = converted.value
    return RuleValue(
        value=value,
        detail=f"constructed by {nested_id}.{output}",
        request=f"Confirm the {nested_id} derivation used for {context.input_name}.",
        provenance=provenance,
    )


def _default(basis: Mapping[str, Any], context: RuleContext) -> RuleValue:
    """The default the spec itself declares. No shipped spec declares one today."""
    del basis
    default = context.declaration.get("default")
    if context.declaration.get("optional", False) is not True:
        raise RuleError(
            "RULE_PRECONDITION",
            f"{context.input_name} is required, so it has no default",
            f"provide {context.input_name}",
        )
    if not isinstance(default, (int, float)) or isinstance(default, bool):
        raise RuleError(
            "SPEC_DEFAULT_ABSENT",
            f"{context.input_name} declares no numeric default; its prose default is not read",
            f"provide {context.input_name}, or confirm the library's fallback",
        )
    return RuleValue(
        value=float(default),
        detail=f"{context.input_name} = {default} from the spec's declared default",
        request=f"Confirm the spec default {default} for {context.input_name}.",
    )


RULES: dict[str, Rule] = {
    COMPOSITION_REMAINDER: Rule(
        id=COMPOSITION_REMAINDER,
        licence=(
            "the balance of a composition may be assigned to one component that the same "
            "source names as unreported"
        ),
        citation="docs/src/calculus/capability.md",
        apply=_remainder,
    ),
    DERIVED_BY_ID: Rule(
        id=DERIVED_BY_ID,
        licence="an input may be the output of another registered azoth id from supplied inputs",
        citation="docs/src/calculus/rho.md",
        apply=_derive,
    ),
    SPEC_DEFAULT: Rule(
        id=SPEC_DEFAULT,
        licence="an optional input may take the numeric default the spec itself declares",
        citation="specs/schema/calc.schema.json",
        apply=_default,
    ),
}

__all__ = [
    "COMPOSITION_REMAINDER",
    "DERIVED_BY_ID",
    "RULES",
    "SPEC_DEFAULT",
    "Rule",
    "RuleContext",
    "RuleError",
    "RuleValue",
]
