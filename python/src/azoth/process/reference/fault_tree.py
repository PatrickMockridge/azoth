"""``process.fault_tree`` - the fault-tree probability and cut-set enumeration.

As NeqSim's ``FaultTreeAnalyzer`` computes it. Spec: ``specs/models/process/fault_tree.toml``

The Python twin of ``crates/azoth-process/src/models/fault_tree.rs``: the flat node list,
the recursive gate probability, the beta correction on OR, the voting masks and the
minimal cut sets.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FaultTreeResult, MinimalCutSet
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.fault_tree"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


@dataclass
class _Node:
    name: str
    gate: str
    probability: float
    k: int
    beta: float
    children: list[int]


def _whole(name: str, value: float) -> int:
    if not math.isfinite(value) or value != math.floor(value):
        raise InvalidInputError(name, f"a whole number is required, and {value} is not")
    return int(value)


def _parse_nodes(rows: list[dict[str, Any]], cap: int) -> tuple[list[_Node], int]:
    count = len(rows)
    if count == 0:
        raise InvalidInputError("nodes", "a fault tree has a root, and the node list is empty")

    nodes: list[_Node] = []
    parents: list[int | None] = []
    roots = 0
    basics = 0
    for index, row in enumerate(rows):
        name = f"nodes[{index}]"
        label = str(row.get("name") or "")
        if not label:
            raise InvalidInputError(f"{name}.name", "a node label is not empty")
        raw_parent = float(row["parent_index"])
        _whole(f"{name}.parent_index", raw_parent)
        if raw_parent == -1.0:
            roots += 1
            parent = None
        elif 0.0 <= raw_parent < count:
            parent = int(raw_parent)
        else:
            raise InvalidInputError(
                f"{name}.parent_index",
                f"a parent index is -1 or a row index below {count}, and {raw_parent} is not",
            )

        gate = str(row.get("gate"))
        if gate not in ("basic", "and", "or", "voting"):
            raise InvalidInputError(
                f"{name}.gate", f"a gate is basic, and, or or voting, and {gate!r} is not"
            )

        raw_probability = row.get("probability")
        if gate == "basic":
            if raw_probability is None:
                raise InvalidInputError(
                    f"{name}.probability", "a basic event carries its probability"
                )
            probability = float(raw_probability)
            if not math.isfinite(probability) or not 0.0 <= probability <= 1.0:
                raise InvalidInputError(
                    f"{name}.probability",
                    f"a basic probability is in [0, 1], and {probability} is not",
                )
            basics += 1
        else:
            if raw_probability is not None:
                raise InvalidInputError(
                    f"{name}.probability", "only a basic event carries a probability"
                )
            probability = 0.0

        raw_k = row.get("k_of_n")
        if gate == "voting":
            if raw_k is None:
                raise InvalidInputError(f"{name}.k_of_n", "a voting gate carries its k")
            k = _whole(f"{name}.k_of_n", float(raw_k))
            if k < 1:
                raise InvalidInputError(
                    f"{name}.k_of_n", f"a voting gate's k is at least one, and {k} is not"
                )
        else:
            if raw_k is not None:
                raise InvalidInputError(f"{name}.k_of_n", "only a voting gate carries a k")
            k = 0

        raw_beta = row.get("beta_ccf")
        if gate == "or":
            beta = 0.0 if raw_beta is None else float(raw_beta)
            if not math.isfinite(beta) or not 0.0 <= beta <= 1.0:
                raise InvalidInputError(
                    f"{name}.beta_ccf",
                    f"a beta factor is in [0, 1], and {beta} is not",
                )
        else:
            if raw_beta is not None:
                raise InvalidInputError(
                    f"{name}.beta_ccf",
                    "the beta factor is read on an or gate only, which is the class's own code",
                )
            beta = 0.0

        nodes.append(
            _Node(
                name=label,
                gate=gate,
                probability=probability,
                k=k,
                beta=beta,
                children=[],
            )
        )
        parents.append(parent)

    if roots != 1:
        raise InvalidInputError(
            "nodes", f"a fault tree has one root, and the node list has {roots}"
        )

    for index, parent in enumerate(parents):
        if parent is not None:
            if parent == index:
                raise InvalidInputError(
                    f"nodes[{index}].parent_index", "a node is not its own parent"
                )
            nodes[parent].children.append(index)

    root = parents.index(None)
    reached = [False] * count
    stack = [root]
    while stack:
        index = stack.pop()
        if reached[index]:
            raise InvalidInputError(
                "nodes", "a node is reached twice, so the parent indices carry a cycle"
            )
        reached[index] = True
        stack.extend(nodes[index].children)
    if not all(reached):
        raise InvalidInputError(
            "nodes",
            "a node is unreachable from the root, so its parent chain does not end there",
        )

    for index, node in enumerate(nodes):
        if node.gate == "basic" and node.children:
            raise InvalidInputError(
                f"nodes[{index}]", "a basic event is a leaf and carries no children"
            )
        if node.gate != "basic" and not node.children:
            raise InvalidInputError(
                f"nodes[{index}]",
                "a gate carries at least one child; the class answers an empty gate but no "
                "tree states one",
            )

    names: set[str] = set()
    for index, node in enumerate(nodes):
        if node.gate == "basic":
            if node.name in names:
                raise InvalidInputError(
                    f"nodes[{index}].name",
                    f"a basic-event name is unique, and {node.name!r} repeats",
                )
            names.add(node.name)

    if basics > cap:
        raise InvalidInputError(
            "nodes",
            f"the cut-set enumeration is 2^n, and {basics} basic events exceeds the {cap} "
            "the procedure carries",
        )
    for index, node in enumerate(nodes):
        if node.gate == "voting":
            n = len(node.children)
            if n > cap:
                raise InvalidInputError(
                    f"nodes[{index}].k_of_n",
                    f"the voting masks are 2^n, and {n} children exceed the {cap} the "
                    "procedure carries",
                )
            if node.k > n:
                raise InvalidInputError(
                    f"nodes[{index}].k_of_n",
                    f"a voting gate's k is in [1, {n}], and {node.k} is not",
                )

    return nodes, root


def _voting_probability(probabilities: list[float], k: int) -> float:
    n = len(probabilities)
    total = 0.0
    for mask in range(1 << n):
        if mask.bit_count() < k:
            continue
        probability = 1.0
        for bit, p in enumerate(probabilities):
            probability *= p if (mask >> bit) & 1 == 1 else 1.0 - p
        total += probability
    return total


def _top_event_probability(index: int, nodes: list[_Node]) -> float:
    node = nodes[index]
    if node.gate == "basic":
        return node.probability
    child_probs = [_top_event_probability(child, nodes) for child in node.children]
    if node.gate == "and":
        probability = 1.0
        for child in child_probs:
            probability *= child
        return probability
    if node.gate == "or":
        complement = 1.0
        for child in child_probs:
            complement *= 1.0 - child
        independent = 1.0 - complement
        if node.beta > 0.0:
            maximum = 0.0
            for child in child_probs:
                maximum = max(maximum, child)
            return (1.0 - node.beta) * independent + node.beta * maximum
        return independent
    return _voting_probability(child_probs, node.k)


def _evaluates_true(index: int, nodes: list[_Node], failed: set[str]) -> bool:
    node = nodes[index]
    if node.gate == "basic":
        return node.name in failed
    true_count = sum(1 for child in node.children if _evaluates_true(child, nodes, failed))
    if node.gate == "and":
        return true_count == len(node.children)
    if node.gate == "or":
        return true_count >= 1
    return true_count >= node.k


def _minimal_cut_sets(nodes: list[_Node], root: int, max_cardinality: int) -> list[list[str]]:
    basics: list[str] = []
    for node in nodes:
        if node.gate == "basic" and node.name not in basics:
            basics.append(node.name)

    cut_sets: list[list[str]] = []
    for mask in range(1, 1 << len(basics)):
        if mask.bit_count() > max_cardinality:
            continue
        selected = {name for bit, name in enumerate(basics) if (mask >> bit) & 1 == 1}
        if not _evaluates_true(root, nodes, selected):
            continue
        sorted_members = sorted(selected)
        if any(all(name in sorted_members for name in existing) for existing in cut_sets):
            continue
        cut_sets.append(sorted_members)
    cut_sets.sort(key=lambda members: (len(members), ", ".join(members)))
    return cut_sets


def fault_tree(
    nodes: list[dict[str, object]],
    max_cardinality: float,
) -> FaultTreeResult:
    """The recursive gate probability and the minimal cut sets.

    Args:
        nodes: the flat node list; a row's position is its index and ``parent_index`` names
            its parent, ``-1`` on the single root.
        max_cardinality: the largest cut set enumerated.

    Returns:
        The top-event probability and the minimal cut sets, ordered by cardinality and then
        member name.

    Raises:
        InvalidInputError: for a malformed tree, a probability or beta outside ``[0, 1]``,
            a ``k`` outside ``[1, n]``, a field on a gate that does not read it, or more
            than the procedure's 20 basics or voting children.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    cardinality = float(max_cardinality)
    apply_checks(checks.on_input, {"max_cardinality": cardinality}.get, warnings)
    _whole("max_cardinality", cardinality)
    if cardinality < 1.0:
        raise InvalidInputError(
            "max_cardinality",
            f"a cut set has at least one member, and {cardinality} is not",
        )

    rows: list[dict[str, Any]] = input_records_to_si(spec, "nodes", nodes)
    cap = int(spec["algorithm"]["max_iterations"])
    parsed, root = _parse_nodes(rows, cap)
    top = _top_event_probability(root, parsed)
    cut_sets = _minimal_cut_sets(parsed, root, int(cardinality))

    return FaultTreeResult(
        top_event_probability=top,
        cut_set_count=float(len(cut_sets)),
        minimal_cut_sets=tuple(
            MinimalCutSet(members=", ".join(members), cardinality=float(len(members)))
            for members in cut_sets
        ),
        warnings=tuple(warnings),
    )
