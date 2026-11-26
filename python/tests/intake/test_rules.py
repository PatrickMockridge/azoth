"""The closed set of licensed rules, and the dormant spec-default rule."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from azoth.intake import RULES, RuleContext, RuleError, load_path

ROOT = Path(__file__).resolve().parents[3]
EXAMPLE = ROOT / "agents" / "intake" / "examples" / "gas-analysis.toml"


def test_the_licensed_set_is_exactly_three() -> None:
    assert set(RULES) == {"composition_remainder", "derived_by_id", "spec_default"}


def test_no_rule_is_a_typical_value_or_a_judgement() -> None:
    for rule_id in RULES:
        assert not any(word in rule_id for word in ("typical", "standard", "analogue", "judgement"))


def test_every_rule_cites_a_repo_path_that_exists() -> None:
    for rule in RULES.values():
        assert (ROOT / rule.citation).exists(), rule.citation
        assert rule.licence


def _context(declaration: dict[str, Any]) -> RuleContext:
    return RuleContext(
        target_id="eos.pt_flash",
        input_name="efficiency",
        declaration=declaration,
        dossier=load_path(EXAMPLE),
        components=None,
    )


def test_spec_default_reads_only_a_numeric_declared_default() -> None:
    value = RULES["spec_default"].apply(
        {}, _context({"optional": True, "default": 0.75, "unit": "dimensionless"})
    )
    assert value.value == 0.75
    with pytest.raises(RuleError, match="required"):
        RULES["spec_default"].apply({}, _context({"default": 0.75}))
    with pytest.raises(RuleError, match="no numeric default"):
        RULES["spec_default"].apply({}, _context({"optional": True}))


def test_the_remainder_rule_names_the_component_it_assigns() -> None:
    value = RULES["composition_remainder"].apply(
        {"claim": "C-1", "unreported": "C-2", "component": "n-heptane"},
        RuleContext(
            target_id="eos.pt_flash",
            input_name="z",
            declaration={"type": "vector", "unit": "dimensionless"},
            dossier=load_path(EXAMPLE),
            components=(
                "methane",
                "ethane",
                "propane",
                "n-butane",
                "n-pentane",
                "n-hexane",
                "n-heptane",
            ),
        ),
    )
    assert value.value[-1] == pytest.approx(0.01)
    assert "n-heptane" in value.request
