"""The transport structs are generated, and this is the ratchet that says so.

`tools/gen_python_transport.py` emits `crates/azoth-python/src/transport_gen.rs` - the struct,
the `__repr__` and the `From<&Kernel>` impl every registered result crosses the boundary in.
`docs-drift` regenerates and diffs it, so a spec or kernel edited without regenerating is caught
in CI; `test_the_committed_transports_are_current` runs the same `--check` in the suite, and
`test_one_struct_per_result_and_every_field_paired` drives the emitter so a result that lost a
struct or a field fails here rather than only as a byte diff.
"""

from __future__ import annotations

import importlib
import re
import subprocess
import sys
from pathlib import Path
from types import ModuleType

REPO_ROOT = Path(__file__).resolve().parents[2]


def _tools_module(name: str) -> ModuleType:
    sys.path.insert(0, str(REPO_ROOT / "tools"))
    try:
        return importlib.import_module(name)
    finally:
        sys.path.pop(0)


def test_the_committed_transports_are_current() -> None:
    """The drift gate `docs-drift` runs, run here so it fails in the suite too.

    Through `--check` rather than by comparing the emit: the generator passes the Rust it emits
    through `rustfmt` before writing, so a comparison against the raw emitter output fails on
    formatting rather than on drift.
    """
    result = subprocess.run(
        [sys.executable, str(REPO_ROOT / "tools" / "gen_python_transport.py"), "--check"],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, (
        f"the transports are out of date - run `python tools/gen_python_transport.py`\n"
        f"{result.stdout}{result.stderr}"
    )


def test_an_optional_vector_of_plain_numbers_is_copied_not_mapped() -> None:
    """`Option<Vec<f64>>` transports as `to_vec()`.

    The optional-vector branch first met a dimensioned element (the separator's `m/s` load
    factors), so it built a closure per element. A plain `f64` element makes that closure
    `|v| *v`, which `cargo clippy -D warnings` refuses as `map_clone`, and
    `iter().copied().collect()` is refused as `iter_cloned_collect` in turn. Pinned here
    because the generated file's meaning does not depend on which spelling it uses, so the
    suite alone would not notice the lint's next visit.
    """
    field_expr = _tools_module("gen_python_transport").field_expr
    assert (
        field_expr("rate", "number", 1, True, None, "")
        == "r.rate.as_ref().map(|values| values.to_vec())"
    )
    assert (
        field_expr("k_values", "number", 2, True, None, "")
        == "r.k_values.as_ref().map(|rows| rows.iter().map(|row| row.to_vec()).collect())"
    )


def test_one_struct_per_result_and_every_field_paired() -> None:
    """Every indexed result gets a struct, and every field in it is the spec's own name.

    Driving the emitter rather than reading the committed file: the claim is that the
    *generator* pairs the index with the structs, so a stale reader shows up here.
    """
    generator = _tools_module("gen_python_transport")
    rust_index = _tools_module("rust_index")
    # The generator's own id table is built by calling `rust_index.check()`, so a spec id with
    # no result type and a result type with no spec both refuse before anything is emitted.
    rust_index.check()

    source = generator.emit()
    emitted = re.findall(r"^pub struct (Py\w+) \{", source, re.M)
    expected = [f"Py{result.rust_name}" for result in rust_index.result_types()]

    assert emitted == expected, "the emitted structs are not one per registered result"

    # Each struct's fields, in the order the emitter writes them, are the public names.
    declarations = re.findall(r"^pub struct (Py\w+) \{(.*?)^\}", source, re.M | re.S)
    assert len(declarations) == len(expected)
    by_name = dict(declarations)
    for result in rust_index.result_types():
        fields = re.findall(r"^\s+pub (\w+):", by_name[f"Py{result.rust_name}"], re.M)
        assert fields == list(result.public_fields), (
            f"{result.calc_id}: the emitted fields are not the declared outputs"
        )
