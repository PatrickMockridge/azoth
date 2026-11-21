"""PVT simulation.

NeqSim's ``pvtsimulation/`` without its ``flowassurance/``: the black-oil and gas-water
correlation tables, the decline and material-balance arithmetic, and the laboratory
experiments a reservoir fluid is described by. The namespace is its own rather than a family
under :mod:`azoth.eos` or :mod:`azoth.process`, mirroring NeqSim's package split: a PVT
experiment is a sequence of flashes and property reads, not an equation of state and not a
unit operation.

The correlation ids are scalar record lists; the laboratory ids take a fluid as
:class:`azoth.characterization.FluidComponent` rows and publish one row per pressure,
temperature or separator stage. Both languages build the fluid through
:func:`azoth.characterization.mixture_from_characterization`, so a pseudo-component crosses
the same way an assay cut does.
"""

from __future__ import annotations

from azoth.pvt import layers as layers

__all__ = ["layers"]
