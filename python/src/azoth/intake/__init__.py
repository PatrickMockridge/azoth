"""The intake review: a dossier's sources, claims and bindings, checked."""

from __future__ import annotations

from azoth.intake.check import Gap, Origin, Request, Review, TargetReview, document, review
from azoth.intake.dossier import (
    SCHEMA,
    Audit,
    Binding,
    Claim,
    Dossier,
    DossierInvalid,
    InputBinding,
    Premise,
    RungOutcome,
    Source,
    Target,
    canonical,
    from_document,
    load_path,
)
from azoth.intake.rules import RULES, Rule, RuleContext, RuleError, RuleValue

__all__ = [
    "RULES",
    "SCHEMA",
    "Audit",
    "Binding",
    "Claim",
    "Dossier",
    "DossierInvalid",
    "Gap",
    "InputBinding",
    "Origin",
    "Premise",
    "Request",
    "Review",
    "Rule",
    "RuleContext",
    "RuleError",
    "RuleValue",
    "RungOutcome",
    "Source",
    "Target",
    "TargetReview",
    "canonical",
    "document",
    "from_document",
    "load_path",
    "review",
]
