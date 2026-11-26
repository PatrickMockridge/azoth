"""The deterministic gate over an intake dossier.

The checker answers one question per target: can every required input be
**constructed** from a claim or a licensed rule? A hard gap means no: the
target's term does not exist. A soft gap means it exists but rests on something
declared — an inference, a lower source rung, a warning-severity range — and the
report must say so. The register and the requests are derived here, never
authored, so a dossier cannot omit the request its own gap raises.
"""

from __future__ import annotations

import math
import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal

from azoth.core.range import RangeCheck, Severity
from azoth.intake.dossier import (
    Binding,
    Claim,
    Dossier,
    DossierInvalid,
    Target,
    from_document,
)
from azoth.intake.rules import RULES, RuleContext, RuleError, RuleValue
from azoth.intake.units import UnitGap, normalize

SCHEMA = "azoth.intake/1"

_NUMBER = re.compile(r"[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][-+]?\d+)?")
_COMPOSITION_NAMES = frozenset(
    {"z", "composition", "overall_composition", "mole_fractions", "feed_z", "feed_composition"}
)
_COMPOSITION_TOLERANCE = 1.0e-9
_MAX_FILE_BYTES = 2_000_000
_MAX_DEPTH = 4

_HARD_ORDER = {
    "UNKNOWN_TARGET": 0,
    "UNKNOWN_INPUT": 1,
    "REQUIRED_INPUT_ABSENT": 2,
    "CONFLICT_UNRESOLVED": 3,
    "SOURCE_BLOCKED": 4,
    "QUOTE_NOT_FOUND": 5,
    "QUOTE_DOES_NOT_SUPPORT_VALUE": 6,
    "UNIT_MISSING": 7,
    "UNIT_AMBIGUOUS": 8,
    "UNIT_DIMENSION_MISMATCH": 9,
    "COMPOSITION_BASIS_MISMATCH": 10,
    "SHAPE_MISMATCH": 11,
    "UNSUPPORTED_BINDING": 12,
    "RULE_UNLICENSED": 13,
    "RULE_PRECONDITION": 14,
    "COMPOSITION_NOT_CLOSED": 15,
    "COMPOSITION_NEGATIVE": 16,
    "RANGE_HARD": 17,
    "INPUTS_MISMATCH": 18,
}
_SOFT_ORDER = {
    "INFERRED_COMPOSITION_REMAINDER": 0,
    "INFERRED_DERIVED_BY_ID": 1,
    "SPEC_DEFAULT_USED": 2,
    "LOWER_RUNG_USED": 3,
    "RANGE_SOFT": 4,
    "RANGE_UNVERIFIED": 5,
}


@dataclass(frozen=True, slots=True)
class Origin:
    input: str
    state: str
    value: Any = None
    source: str | None = None
    quote: str | None = None
    rule: str | None = None
    rung: str | None = None
    rungs_above: tuple[str, ...] = ()
    detail: str | None = None
    provenance: Mapping[str, Any] | None = None


@dataclass(frozen=True, slots=True)
class Gap:
    gap: str
    target: str
    input: str
    class_: Literal["hard", "soft"]
    code: str
    detail: str
    consequence: str
    ask: str
    why: str
    accepts: str
    blocking: bool


@dataclass(frozen=True, slots=True)
class Request:
    request: str
    gap: str
    input: str
    ask: str
    why: str
    accepts: str
    blocking: bool
    priority: int


@dataclass(frozen=True, slots=True)
class TargetReview:
    id: str
    verdict: str
    origins: tuple[Origin, ...]
    inputs: Mapping[str, Any]
    gaps: tuple[Gap, ...]


@dataclass(frozen=True, slots=True)
class Review:
    schema: str
    dossier: str
    targets: tuple[TargetReview, ...]
    register: tuple[Gap, ...]
    requests: tuple[Request, ...]

    @property
    def verdict(self) -> str:
        if any(gap.class_ == "hard" for gap in self.register):
            return "refused"
        if self.register:
            return "passed_with_flags"
        return "passed"


@dataclass
class _TargetState:
    target: Target
    entry: Mapping[str, Any]
    declared: Mapping[str, Any]
    components: tuple[str, ...] | None = None
    origins: list[Origin] = field(default_factory=list)
    gaps: list[tuple[str, str, str, str, str, str, str, str]] = field(default_factory=list)
    values: dict[str, Any] = field(default_factory=dict)


def _collapse(text: str) -> str:
    return " ".join(text.split())


def _kind(declaration: Mapping[str, Any]) -> str:
    return str(declaration.get("type", "quantity"))


def _unit(declaration: Mapping[str, Any]) -> str:
    return str(declaration.get("unit", "dimensionless"))


def _is_composition(name: str, declaration: Mapping[str, Any]) -> bool:
    if _kind(declaration) != "vector" or _unit(declaration) != "dimensionless":
        return False
    description = str(declaration.get("description", "")).lower()
    return (
        name in _COMPOSITION_NAMES or "mole fraction" in description or "composition" in description
    )


def _quote_has_number(quote: str, value: float) -> bool:
    for token in _NUMBER.findall(quote):
        try:
            number = float(token)
        except ValueError:
            continue
        if math.isclose(number, value, rel_tol=1.0e-12, abs_tol=1.0e-15):
            return True
    return False


def _number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


class _Reviewer:
    def __init__(self, dossier: Dossier, *, keycard: str | Path | None = None) -> None:
        self.dossier = dossier
        self.keycard = keycard

    # -- entry and components ---------------------------------------------------------

    def _entry(self, target_id: str) -> tuple[str, Mapping[str, Any]] | None:
        from azoth.harness.catalogue import entry_for

        try:
            return entry_for(target_id)
        except Exception:
            return None

    def _components(self, state: _TargetState) -> tuple[str, ...] | None:
        if state.components is not None:
            return state.components
        for name in ("components", "component_names"):
            declaration = state.declared.get(name)
            if declaration is None or name not in state.values:
                continue
            value = state.values[name]
            if isinstance(value, list) and all(isinstance(item, str) for item in value):
                state.components = tuple(value)
                return state.components
        return None

    # -- source and claim checks ------------------------------------------------------

    def _file_text(self, source_id: str, locator: str) -> str | None:
        source = self.dossier.source(source_id)
        if source.kind != "file":
            return None
        path_text = locator.split(":", 1)[0] if re.search(r":\d+-\d+$", locator) else locator
        path = Path(path_text).expanduser()
        try:
            resolved = path.resolve()
        except OSError:
            return None
        if not resolved.is_relative_to(Path.cwd().resolve()):
            return None
        try:
            with resolved.open("rb") as handle:
                raw = handle.read(_MAX_FILE_BYTES + 1)
        except OSError:
            return None
        if len(raw) > _MAX_FILE_BYTES:
            return None
        try:
            return raw.decode("utf-8")
        except UnicodeDecodeError:
            return None

    def _check_claim(
        self, state: _TargetState, name: str, claim: Claim, *, literal: bool = True
    ) -> None:
        source = self.dossier.source(claim.source)
        if source.status != "used":
            self._gap(
                state,
                name,
                "hard",
                "SOURCE_BLOCKED",
                f"the claim cites {source.id}, whose status is {source.status}",
                f"obtain {source.locator}",
                "a used source for this input",
            )
            return
        if _collapse(claim.quote) not in _collapse(source.excerpt):
            self._gap(
                state,
                name,
                "hard",
                "QUOTE_NOT_FOUND",
                f"the quoted words are not in {source.id}'s excerpt",
                f"cite the sentence in {source.locator} that states {name}",
                "a quote the source contains",
            )
            return
        if source.kind == "file":
            text = self._file_text(claim.source, source.locator)
            if text is None:
                self._gap(
                    state,
                    name,
                    "hard",
                    "SOURCE_BLOCKED",
                    f"{source.locator} cannot be read as a UTF-8 file under the working directory",
                    f"supply {source.locator} in the workspace",
                    "a readable file",
                )
                return
            if _collapse(source.excerpt) not in _collapse(text):
                self._gap(
                    state,
                    name,
                    "hard",
                    "QUOTE_NOT_FOUND",
                    f"{source.id}'s excerpt is not in {source.locator}",
                    f"copy the excerpt from {source.locator}",
                    "an excerpt the file contains",
                )
                return
        if not literal:
            return
        if _number(claim.value) and not _quote_has_number(claim.quote, float(claim.value)):
            self._gap(
                state,
                name,
                "hard",
                "QUOTE_DOES_NOT_SUPPORT_VALUE",
                f"the quote does not contain the value {claim.value!r}",
                f"cite the sentence that states the value of {name}",
                "a quote containing the value",
            )
        elif isinstance(claim.value, str) and claim.value.lower() not in claim.quote.lower():
            self._gap(
                state,
                name,
                "hard",
                "QUOTE_DOES_NOT_SUPPORT_VALUE",
                f"the quote does not contain {claim.value!r}",
                f"cite the words that state {name}",
                "a quote containing the value",
            )

    # -- binding resolution ------------------------------------------------------------

    def _resolve_claim(
        self, state: _TargetState, name: str, declaration: Mapping[str, Any], claim: Claim
    ) -> Any:
        self._check_claim(state, name, claim)
        kind = _kind(declaration)
        unit = _unit(declaration)
        expect_mole = _is_composition(name, declaration)
        interval = declaration.get("interval", False) is True
        if kind in ("enum", "string", "boolean", "components", "vector_string", "fitting_list"):
            return claim.value
        if kind == "record_list":
            self._gap(
                state,
                name,
                "hard",
                "UNSUPPORTED_BINDING",
                "record-list claims must carry a unit per field and are not read in this tranche",
                f"bind {name} as a scalar or vector, or provide field units",
                "a supported binding",
            )
            return None
        try:
            if kind == "quantity":
                return normalize(
                    claim.value,
                    claim.unit,
                    unit,
                    name,
                    expect_mole=expect_mole,
                    interval=interval,
                ).value
            if kind == "vector":
                values = claim.value
                if isinstance(values, (str, bytes)) or not isinstance(values, Sequence):
                    raise UnitGap("SHAPE_MISMATCH", f"{name}: expected a vector", f"provide {name}")
                return [
                    normalize(
                        item,
                        claim.unit,
                        unit,
                        f"{name}[{index}]",
                        expect_mole=expect_mole,
                        interval=interval,
                    ).value
                    for index, item in enumerate(values)
                ]
            if kind == "matrix":
                rows = claim.value
                if isinstance(rows, (str, bytes)) or not isinstance(rows, Sequence):
                    raise UnitGap("SHAPE_MISMATCH", f"{name}: expected a matrix", f"provide {name}")
                matrix: list[list[float]] = []
                for row, items in enumerate(rows):
                    if isinstance(items, (str, bytes)) or not isinstance(items, Sequence):
                        raise UnitGap(
                            "SHAPE_MISMATCH",
                            f"{name}[{row}]: expected a row",
                            f"provide {name} as a matrix",
                        )
                    matrix.append(
                        [
                            normalize(
                                item,
                                claim.unit,
                                unit,
                                f"{name}[{row}][{column}]",
                                expect_mole=expect_mole,
                                interval=interval,
                            ).value
                            for column, item in enumerate(items)
                        ]
                    )
                return matrix
        except UnitGap as exc:
            self._gap(state, name, "hard", exc.code, exc.detail, exc.request, "a convertible value")
            return None
        self._gap(
            state,
            name,
            "hard",
            "UNSUPPORTED_BINDING",
            f"the {kind!r} input kind has no intake codec",
            f"bind {name} in a supported shape",
            "a supported binding",
        )
        return None

    def _resolve_rule(
        self,
        state: _TargetState,
        name: str,
        declaration: Mapping[str, Any],
        binding: Binding,
        depth: int,
    ) -> RuleValue | None:
        assert binding.rule is not None and binding.basis is not None
        rule = RULES.get(binding.rule)
        if rule is None:
            self._gap(
                state,
                name,
                "hard",
                "RULE_UNLICENSED",
                f"{binding.rule!r} is not a licensed rule",
                f"provide {name} directly or cite one of {sorted(RULES)}",
                "a licensed rule",
            )
            return None
        if binding.rule == "composition_remainder":
            for key, literal in (("claim", True), ("unreported", False)):
                claim_id = binding.basis.get(key)
                if not isinstance(claim_id, str):
                    continue
                try:
                    claim = self.dossier.claim(claim_id)
                except DossierInvalid:
                    self._gap(
                        state,
                        name,
                        "hard",
                        "RULE_PRECONDITION",
                        f"the rule's basis cites unknown claim {claim_id!r}",
                        f"name the claim that carries `{key}`",
                        "a claim the dossier contains",
                    )
                    return None
                self._check_claim(state, name, claim, literal=literal)

        def derive(
            nested_id: str, output: str, inputs: Mapping[str, Any], nested_depth: int
        ) -> tuple[Any, Mapping[str, Any]]:
            return self._derive(state, name, nested_id, output, inputs, nested_depth)

        context = RuleContext(
            target_id=state.target.id,
            input_name=name,
            declaration=declaration,
            dossier=self.dossier,
            components=self._components(state),
            depth=depth,
            derive=derive,
        )
        try:
            return rule.apply(binding.basis, context)
        except RuleError as exc:
            self._gap(
                state, name, "hard", exc.code, exc.detail, exc.request, "a constructible input"
            )
            return None

    def _derive(
        self,
        state: _TargetState,
        outer_name: str,
        nested_id: str,
        output: str,
        inputs: Mapping[str, Any],
        depth: int,
    ) -> tuple[Any, Mapping[str, Any]]:
        if depth > _MAX_DEPTH:
            raise RuleError(
                "RULE_PRECONDITION",
                f"the derivation chain is deeper than {_MAX_DEPTH}",
                "derive the input in fewer steps",
            )
        resolved = self._entry(nested_id)
        if resolved is None:
            raise RuleError(
                "RULE_UNLICENSED",
                f"{nested_id!r} is not a registered id",
                "name a registered calculation",
            )
        _, entry = resolved
        declared = entry["inputs"]
        document: dict[str, Any] = {}
        nested_state = _TargetState(
            target=Target(id=nested_id, inputs=()), entry=entry, declared=declared
        )
        for name, raw in inputs.items():
            if name not in declared:
                raise RuleError(
                    "RULE_PRECONDITION",
                    f"{nested_id} has no input {name!r}",
                    f"bind one of {sorted(declared)}",
                )
            binding = _binding_from_mapping(raw, f"derivation.{nested_id}.{name}")
            value = self._resolve_binding(nested_state, name, declared[name], binding, depth)
            if value is None:
                raise RuleError(
                    "RULE_PRECONDITION",
                    f"the derivation of {nested_id} rests on an unconstructible {name}",
                    f"provide {name} for {nested_id}",
                )
            document[name] = value
        try:
            from azoth.harness.api import run as run_calc

            envelope = run_calc(nested_id, document, keycard=self.keycard)
        except Exception as exc:
            raise RuleError(
                "RULE_PRECONDITION",
                f"{nested_id} could not be run: {exc}",
                f"check the derivation inputs for {nested_id}",
            ) from exc
        result = envelope.get("result", {})
        if output not in result:
            raise RuleError(
                "RULE_PRECONDITION",
                f"{nested_id} has no output {output!r}",
                f"name one of {sorted(result)}",
            )
        value = result[output]
        provenance = result.get("provenance")
        if isinstance(value, Mapping) and set(value) == {"magnitude", "unit"}:
            try:
                value = normalize(
                    value["magnitude"],
                    str(value["unit"]),
                    _unit(state.declared[outer_name]),
                    outer_name,
                ).value
            except UnitGap as exc:
                raise RuleError(exc.code, exc.detail, exc.request) from exc
        return value, provenance if isinstance(provenance, Mapping) else {}

    def _resolve_binding(
        self,
        state: _TargetState,
        name: str,
        declaration: Mapping[str, Any],
        binding: Binding,
        depth: int,
    ) -> Any:
        if binding.claim is not None:
            claim = self.dossier.claim(binding.claim)
            value = self._resolve_claim(state, name, declaration, claim)
            if value is None:
                return None
            state.origins.append(
                Origin(
                    input=name,
                    state="supplied",
                    value=value,
                    source=claim.source,
                    quote=claim.quote,
                    rung=claim.rung,
                    rungs_above=tuple(f"{rung.rung}:{rung.outcome}" for rung in claim.rungs_above),
                    detail=claim.rung,
                )
            )
            self._lower_rung(state, name, claim)
            return value
        if binding.rule is not None:
            rule_value = self._resolve_rule(state, name, declaration, binding, depth)
            if rule_value is None:
                return None
            assumed = binding.rule == "spec_default"
            state.origins.append(
                Origin(
                    input=name,
                    state="assumed" if assumed else "inferred",
                    value=rule_value.value,
                    rule=binding.rule,
                    detail=rule_value.detail,
                    provenance=rule_value.provenance,
                )
            )
            self._gap(
                state,
                name,
                "soft",
                "SPEC_DEFAULT_USED" if assumed else f"INFERRED_{binding.rule.upper()}",
                rule_value.detail,
                rule_value.request,
                "a soft inference; confirm it",
            )
            return rule_value.value
        if binding.absent:
            if declaration.get("optional", False) is True:
                state.origins.append(Origin(input=name, state="absent", detail=binding.note))
                return None
            self._gap(
                state,
                name,
                "hard",
                "REQUIRED_INPUT_ABSENT",
                binding.note or "the dossier records no value for a required input",
                f"provide {name}",
                f"a value or a licensed construction in {_unit(declaration)}",
            )
            return None
        self._gap(
            state,
            name,
            "hard",
            "CONFLICT_UNRESOLVED",
            binding.note or "two sources disagree and no resolution is recorded",
            f"reconcile the values for {name}",
            "one value, with the resolution cited",
        )
        return None

    def _lower_rung(self, state: _TargetState, name: str, claim: Claim) -> None:
        source = self.dossier.source(claim.source)
        if source.rank is None:
            return
        for other in self.dossier.sources:
            if (
                other.status == "blocked"
                and other.rank is not None
                and other.rank < source.rank
                and (not other.covers or name in other.covers)
            ):
                self._gap(
                    state,
                    name,
                    "soft",
                    "LOWER_RUNG_USED",
                    f"{source.id} (rung {source.rank}) is used; {other.id} (rung "
                    f"{other.rank}) was blocked",
                    f"obtain {other.locator}",
                    "the higher-rung source",
                )
                return

    # -- gaps and the target ----------------------------------------------------------

    def _gap(
        self,
        state: _TargetState,
        name: str,
        class_: Literal["hard", "soft"],
        code: str,
        detail: str,
        ask: str,
        accepts: str,
    ) -> None:
        consequence = "target refused" if class_ == "hard" else "runs; flagged"
        why = (
            f"{state.target.id}.{name} cannot be constructed from the dossier"
            if class_ == "hard"
            else f"{state.target.id}.{name} was constructed but the evidence is declared"
        )
        state.gaps.append((name, class_, code, detail, consequence, ask, why, accepts))

    def _target(self, target: Target) -> TargetReview:
        resolved = self._entry(target.id)
        if resolved is None:
            state = _TargetState(target=target, entry={}, declared={})
            self._gap(
                state,
                "target",
                "hard",
                "UNKNOWN_TARGET",
                f"{target.id!r} is not a registered id",
                "choose a registered id with azoth_catalogue",
                "a registered id",
            )
            gaps = self._finish_gaps(target, state.gaps)
            return TargetReview(target.id, "refused", (), {}, gaps)
        _, entry = resolved
        declared = entry["inputs"]
        state = _TargetState(target=target, entry=entry, declared=declared)
        ordered = sorted(
            target.inputs,
            key=lambda item: 0 if item.name in ("components", "component_names") else 1,
        )
        for item in ordered:
            if item.name not in declared:
                self._gap(
                    state,
                    item.name,
                    "hard",
                    "UNKNOWN_INPUT",
                    f"{target.id} declares no input {item.name!r}",
                    f"bind one of {sorted(declared)}",
                    "a declared input",
                )
                continue
            value = self._resolve_binding(state, item.name, declared[item.name], item.binding, 0)
            if value is not None:
                state.values[item.name] = value
        for name, declaration in declared.items():
            if declaration.get("optional", False) is True:
                continue
            if name not in state.values and not any(gap[0] == name for gap in state.gaps):
                self._gap(
                    state,
                    name,
                    "hard",
                    "REQUIRED_INPUT_ABSENT",
                    "the dossier does not bind this required input",
                    f"provide {name}",
                    f"a value in {_unit(declaration)}",
                )
        self._decode(state)
        self._ranges(state)
        self._composition(state)
        gaps = self._finish_gaps(target, state.gaps)
        verdict = (
            "refused"
            if any(gap.class_ == "hard" for gap in gaps)
            else ("passed_with_flags" if gaps else "passed")
        )
        return TargetReview(target.id, verdict, tuple(state.origins), dict(state.values), gaps)

    def _decode(self, state: _TargetState) -> None:
        missing = [
            name
            for name, declaration in state.declared.items()
            if declaration.get("optional", False) is not True and name not in state.values
        ]
        if missing:
            return
        try:
            from azoth.harness.inputs import decode_inputs

            decoded = decode_inputs(state.entry, state.values)
        except Exception as exc:
            field = getattr(exc, "field", None) or "inputs"
            if any(gap[0] == field for gap in state.gaps):
                return
            self._gap(
                state,
                str(field),
                "hard",
                "SHAPE_MISMATCH",
                str(exc),
                f"correct {field}",
                "a value that decodes against the spec",
            )
            return
        state.values.clear()
        state.values.update(decoded)

    def _ranges(self, state: _TargetState) -> None:
        for raw in state.entry.get("valid_range", ()):
            try:
                check = RangeCheck.from_spec(raw)
            except Exception:
                continue
            if check.quantity not in state.declared:
                continue
            if check.quantity not in state.values:
                if any(gap[0] == check.quantity and gap[1] == "hard" for gap in state.gaps):
                    continue
                self._gap(
                    state,
                    check.quantity,
                    "soft",
                    "RANGE_UNVERIFIED",
                    f"the declared check on {check.quantity} could not run because the "
                    "input is absent",
                    f"provide {check.quantity} to evaluate: {check.rationale}",
                    f"a value in {_unit(state.declared[check.quantity])}",
                )
                continue
            value = state.values[check.quantity]
            if not _number(value):
                continue
            if not check.violated(float(value)):
                continue
            if check.severity == Severity.ERROR:
                self._gap(
                    state,
                    check.quantity,
                    "hard",
                    "RANGE_HARD",
                    f"{check.quantity} = {value} is outside the spec's valid range: "
                    f"{check.rationale}",
                    f"provide {check.quantity} within the declared range",
                    check.rationale,
                )
            else:
                self._gap(
                    state,
                    check.quantity,
                    "soft",
                    "RANGE_SOFT",
                    f"{check.quantity} = {value} is outside the validated range; the kernel "
                    f"returns a value with {check.code.value}",
                    f"confirm {check.quantity} is intended: {check.rationale}",
                    check.rationale,
                )

    def _composition(self, state: _TargetState) -> None:
        components = self._components(state)
        if components is None:
            return
        candidates = [
            name
            for name, declaration in state.declared.items()
            if _is_composition(name, declaration)
            and name in state.values
            and isinstance(state.values[name], list)
            and len(state.values[name]) == len(components)
        ]
        if not candidates:
            return
        name = "z" if "z" in candidates else candidates[0]
        values = state.values[name]
        if any(_number(value) and float(value) < 0.0 for value in values):
            self._gap(
                state,
                name,
                "hard",
                "COMPOSITION_NEGATIVE",
                f"{name} has a negative fraction",
                f"provide nonnegative fractions for {name}",
                "a nonnegative composition",
            )
            return
        total = sum(float(value) for value in values if _number(value))
        if abs(total - 1.0) > _COMPOSITION_TOLERANCE:
            self._gap(
                state,
                name,
                "hard",
                "COMPOSITION_NOT_CLOSED",
                f"{name} sums to {total:.12g}, not 1 within {_COMPOSITION_TOLERANCE:g}; "
                "the spec does not renormalise",
                "provide the missing fraction or a composition that closes",
                "a composition summing to one",
            )

    def _finish_gaps(
        self, target: Target, raw: list[tuple[str, str, str, str, str, str, str, str]]
    ) -> tuple[Gap, ...]:
        del target
        ordered = sorted(
            raw,
            key=lambda item: (
                0 if item[1] == "hard" else 1,
                _HARD_ORDER.get(item[2], 99) if item[1] == "hard" else _SOFT_ORDER.get(item[2], 99),
                item[0],
                item[2],
            ),
        )
        gaps: list[Gap] = []
        for index, (name, class_, code, detail, consequence, ask, why, accepts) in enumerate(
            ordered, 1
        ):
            gaps.append(
                Gap(
                    gap=f"G-{index:02d}",
                    target="",
                    input=name,
                    class_="hard" if class_ == "hard" else "soft",
                    code=code,
                    detail=detail,
                    consequence=consequence,
                    ask=ask,
                    why=why,
                    accepts=accepts,
                    blocking=class_ == "hard",
                )
            )
        return tuple(gaps)

    def run(self) -> Review:
        reviewed = [self._target(target) for target in self.dossier.targets]
        pairs = [(target.id, gap) for target in reviewed for gap in target.gaps]
        pairs.sort(
            key=lambda pair: (
                0 if pair[1].class_ == "hard" else 1,
                _HARD_ORDER.get(pair[1].code, 99)
                if pair[1].class_ == "hard"
                else _SOFT_ORDER.get(pair[1].code, 99),
                pair[0],
                pair[1].input,
            )
        )
        register = [
            Gap(
                gap=f"G-{index:02d}",
                target=target_id,
                input=gap.input,
                class_=gap.class_,
                code=gap.code,
                detail=gap.detail,
                consequence=gap.consequence,
                ask=gap.ask,
                why=gap.why,
                accepts=gap.accepts,
                blocking=gap.blocking,
            )
            for index, (target_id, gap) in enumerate(pairs, 1)
        ]
        requests = [
            Request(
                request=f"R-{index:02d}",
                gap=gap.gap,
                input=gap.input,
                ask=gap.ask,
                why=gap.why,
                accepts=gap.accepts,
                blocking=gap.blocking,
                priority=index,
            )
            for index, gap in enumerate(register, 1)
        ]
        targets = [
            TargetReview(
                id=target.id,
                verdict=target.verdict,
                origins=target.origins,
                inputs=target.inputs,
                gaps=tuple(gap for gap in register if gap.target == target.id),
            )
            for target in reviewed
        ]
        return Review(
            schema=SCHEMA,
            dossier=self.dossier.id,
            targets=tuple(targets),
            register=tuple(register),
            requests=tuple(requests),
        )


def _binding_from_mapping(raw: Any, field: str) -> Binding:
    if not isinstance(raw, Mapping):
        raise RuleError(
            "RULE_PRECONDITION", f"{field}: a nested binding must be an object", f"correct {field}"
        )
    claim = raw.get("claim")
    rule = raw.get("rule")
    absent = raw.get("absent", False) is True
    conflicted = raw.get("conflicted", False) is True
    basis = raw.get("basis")
    note = raw.get("note")
    chosen = (
        int(isinstance(claim, str)) + int(isinstance(rule, str)) + int(absent) + int(conflicted)
    )
    if chosen != 1 or (basis is not None and not isinstance(basis, Mapping)):
        raise RuleError(
            "RULE_PRECONDITION",
            f"{field}: a nested binding is one of claim, rule, absent or conflicted",
            f"correct {field}",
        )
    if isinstance(rule, str) and basis is None:
        raise RuleError(
            "RULE_PRECONDITION", f"{field}: a rule binding needs its basis", f"correct {field}"
        )
    return Binding(
        claim=claim if isinstance(claim, str) else None,
        rule=rule if isinstance(rule, str) else None,
        basis=basis if isinstance(basis, Mapping) else None,
        absent=absent,
        conflicted=conflicted,
        note=str(note) if isinstance(note, str) else None,
    )


def review(document: Mapping[str, Any] | Dossier, *, keycard: str | Path | None = None) -> Review:
    """Check every target in a dossier and return its verdict, register and requests."""
    dossier = document if isinstance(document, Dossier) else from_document(document)
    return _Reviewer(dossier, keycard=keycard).run()


def _origin_document(origin: Origin) -> dict[str, Any]:
    document: dict[str, Any] = {
        "input": origin.input,
        "state": origin.state,
        "value": origin.value,
        "source": origin.source,
        "quote": origin.quote,
        "rule": origin.rule,
        "rung": origin.rung,
        "rungs_above": list(origin.rungs_above),
        "detail": origin.detail,
    }
    if origin.provenance:
        document["provenance"] = dict(origin.provenance)
    return document


def document(checked: Review, *, target: str | None = None) -> dict[str, Any]:
    """The review as the envelope block, optionally for one target only."""
    targets = [item for item in checked.targets if target is None or item.id == target]
    register = [gap for gap in checked.register if target is None or gap.target == target]
    gap_ids = {gap.gap for gap in register}
    requests = [request for request in checked.requests if request.gap in gap_ids]
    verdict = targets[0].verdict if target is not None and len(targets) == 1 else checked.verdict
    return {
        "verdict": verdict,
        "targets": [
            {
                "id": item.id,
                "verdict": item.verdict,
                "origins": [_origin_document(origin) for origin in item.origins],
                "gaps": [gap.gap for gap in item.gaps],
            }
            for item in targets
        ],
        "register": [
            {
                "gap": gap.gap,
                "target": gap.target,
                "input": gap.input,
                "class": gap.class_,
                "code": gap.code,
                "detail": gap.detail,
                "consequence": gap.consequence,
                "request": gap.gap,
                "ask": gap.ask,
                "why": gap.why,
                "accepts": gap.accepts,
                "blocking": gap.blocking,
            }
            for gap in register
        ],
        "requests": [
            {
                "request": request.request,
                "gap": request.gap,
                "input": request.input,
                "ask": request.ask,
                "why": request.why,
                "accepts": request.accepts,
                "blocking": request.blocking,
                "priority": request.priority,
            }
            for request in requests
        ],
    }


__all__ = ["Gap", "Origin", "Request", "Review", "TargetReview", "document", "review"]
