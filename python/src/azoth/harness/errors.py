"""The bridge's refusals, each with the exit code the CLI gives it."""

from __future__ import annotations


class HarnessError(Exception):
    """A call the bridge refused, named by kind and optionally by field."""

    exit_code = 5
    kind = "HarnessError"

    def __init__(self, message: str, *, field: str | None = None) -> None:
        super().__init__(message)
        self.field = field


class UnknownId(HarnessError):
    """An id that is neither a registered calculation nor a model."""

    exit_code = 2
    kind = "UnknownId"


class UndefinedInput(HarnessError):
    """An input, unit or value the spec does not admit."""

    exit_code = 3
    kind = "UndefinedInput"


class HarnessInputError(UndefinedInput):
    """The conversion layer's name for an input it cannot place."""

    kind = "HarnessInputError"


class InputTooLarge(HarnessError):
    """A JSON document above the bridge's input cap."""

    exit_code = 3
    kind = "InputTooLarge"


class DossierInvalid(HarnessInputError):
    """A dossier that cannot be read as one, so there is nothing to review."""

    kind = "DossierInvalid"


class DossierRequired(HarnessError):
    """A run that requires a dossier and was given none."""

    exit_code = 3
    kind = "DossierRequired"


class TargetMismatch(UndefinedInput):
    """A dossier with no target, or more than one, for the id being run."""

    kind = "TargetMismatch"


class InputsMismatch(UndefinedInput):
    """Run-time inputs that are not the values the dossier constructed."""

    kind = "InputsMismatch"


class IntakeRefused(UndefinedInput):
    """A refused dossier under the strict mode, stopped before the kernel."""

    kind = "IntakeRefused"


class OutputTooLarge(HarnessError):
    """A result above the bridge's output cap."""

    exit_code = 5
    kind = "OutputTooLarge"
