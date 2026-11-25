"""The calculation bridge a harness calls: catalogue, spec, run and version.

The bridge is the library's public surface as JSON. It resolves an id from the
registry, decodes the spec-shaped inputs, calls the public function on the selected
backend, and returns the result with its provenance block. It is what
`python -m azoth.harness` prints and what `agents/deepseek/plugin/` invokes; it holds no
DeepSeek Harness dependency.
"""

from __future__ import annotations

from azoth.harness.api import run, version_info
from azoth.harness.catalogue import SCHEMA, catalogue, spec
from azoth.harness.errors import (
    HarnessError,
    HarnessInputError,
    InputTooLarge,
    OutputTooLarge,
    UndefinedInput,
    UnknownId,
)

__all__ = [
    "SCHEMA",
    "HarnessError",
    "HarnessInputError",
    "InputTooLarge",
    "OutputTooLarge",
    "UndefinedInput",
    "UnknownId",
    "catalogue",
    "run",
    "spec",
    "version_info",
]
