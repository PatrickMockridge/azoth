# Depressurization screening

Educational blowdown and depressurization screening with time-to-target and low-temperature flags. USE WHEN: a task needs a public, screening-level blowdown time indicator and auto-refrigeration low-temperature triage without proprietary blowdown design methods.

**Ported.** `process.depressurization` marches the vessel, `process.depressurization_acceptance`
tests a stored trajectory, and `process.non_equilibrium_blowdown` gives the two-zone
temperature split; the rest of the family covers filling, the PSV latch, blocked-outlet demand,
the shared header and the steady-versus-dynamic sizing.

## When to Use

A vessel or a segment whose blowdown time, low-temperature margin or relief demand is being
screened; a fire-exposed inventory whose vapour and liquid temperatures are expected to
diverge; or a relief header whose simultaneous load is being checked.

## Inputs

- **The equilibrium blowdown** (`process.depressurization`): the fluid's components and
  composition, the initial temperature and pressure, the vessel volume, the orifice diameter,
  the coefficient and the back pressure; optionally the constant fire duty or the
  temperature-dependent fire (a preset or the explicit emissivity, flame temperature and
  convection over the exposed area), the lumped wall, the step and horizon, the stop pressure
  and the withdrawal mode (`auto`, `bulk` or `vapour`).
- **The acceptance** (`process.depressurization_acceptance`): the stored trajectory rows
  (time, pressure, mass, mass flow), the optional escape and rupture times, and the optional
  pressure, mass and fire-rate limits.
- **The valve** (`process.psv_valve`): the set pressure, blowdown fraction, coefficient,
  orifice area, back pressure, the upstream state, the prior open state and pop count.
- **The rest**: `process.vessel_filling` (the charge), `process.blocked_outlet_overpressure`
  (the feed), `process.non_equilibrium_blowdown` (the two-zone fire case),
  `process.multi_vessel_blowdown_study` (the source samples) and
  `process.dynamic_psv_sizing` (the fire duty and the set pressure).

## Outputs

- The pressure, temperature, mass, wall temperature and discharge-rate history; the
  half-pressure and 7 barg crossing times; the minimum and maximum temperatures; the
  monotonic flags; the vapour-withdrawal and fallback counts.
- The acceptance's limiting time, interpolated pressure and mass, peak rate and verdict.
- The valve's open state, pop count and relieving flow.
- The filling's summary and liner flags; the blocked-outlet time to demand; the two-zone
  temperatures and their maximum bifurcation; the header peak, contributions and Mach; the
  steady and dynamic areas and the oversizing ratio.

## How a calculation runs

The equilibrium march aligns the inventory to the vessel volume, then each step discharges
through the orifice — the **lightest** phase when the flash is multiphase and its density is
at most 300 kg/m3, taking at most a quarter of that phase — and updates the state with a mass
and energy balance, the lumped wall and the fire boundary, and a VU flash at the vessel
volume. The class's single-component VU route, its ideal-gas fallback and its silent
per-step substitutions are mirrored. The API 521 summary interpolates the half-pressure and
8 bara crossings and applies the 900 s horizon; the acceptance evaluates a stored trajectory
against the escape and rupture times and the three limits. The non-equilibrium model splits
the feed into vapour and liquid zones, sets the pressure by a vapour TV flash at the
remaining volume, and couples them through the wetted wall (Rohsenow boiling) and the
interface.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

blowdown = azoth.process.depressurization(
    components=["methane", "ethane", "propane", "n-butane"],
    initial_z=[0.85, 0.08, 0.04, 0.03],
    initial_t=quantity(313.15, "K"),
    initial_p=quantity(9.0e6, "Pa"),
    vessel_volume=quantity(30.0, "m**3"),
    orifice_diameter=quantity(0.03, "m"),
    discharge_coefficient=0.85,
    back_pressure=quantity(2.0e5, "Pa"),
    max_time=quantity(900.0, "s"),
)
print(blowdown.time_to_half_pressure.to("s"))  # about 119 s
print(blowdown.seven_barg_criterion_met)  # True
print(blowdown.min_wall_temperature.to("K"))  # the MDMT check
```

## The keycard

The gas's state is the model's own: the initial feed is PR with the classic mixing rule, and
the march flashes each step. **The vessel is one homogeneous control volume**, and a
multiphase inventory discharges through a top-mounted valve, so the vapour leaves and the
liquid stays — a bulk withdrawal understates the flare load and overstates the cooling
without failing. The wall is a lump, and a zero wall mass disables it rather than defaulting.

## Validation checklist

- [ ] State the withdrawal basis: a top-mounted valve passes vapour, not the bulk.
- [ ] Check the minimum wall temperature against the MDMT, not the final temperature.
- [ ] Use the half-pressure and 7 barg criteria for the API 521 screen, and the STS-0131
      acceptance for a stored trajectory against the escape and rupture times.
- [ ] Carry the relieving state into the PSV sizing and the multi-vessel header.
- [ ] Remember the class's fire defaults: a non-equilibrium run applies a fire even when none
      is configured.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A flare load far below expectation | Bulk withdrawal on a two-phase inventory | Use `auto` or `vapour` |
| A blowdown that misses 7 barg | The fire duty or the orifice is the wrong basis | Restate both and rerun |
| A low-temperature flag that does not move | The minimum wall temperature, not the fluid's | Enable the wall model |
| A non-equilibrium run collapsing at once | The default 1 kg wall and derived fire area | Set the wall and the exposed area |

## Limitations

The ids are the class's screening forms: the equilibrium march is one homogeneous control
volume with the class's per-step fallbacks, and the two-zone model is the class's
screening-grade split, not a rigorous coupled VLE march. The default fire and wall parameters
are the class's and can be unstable. The class's reports, recommendation strings and unit
setters are presentation.

## Related Azoth functionality

- `process.depressurization` — the equilibrium trajectory and API 521 summary.
- `process.depressurization_acceptance` — the STS-0131 acceptance of a stored trajectory.
- `process.psv_valve` — one step of the PSV latch and flow.
- `process.vessel_filling` and `process.blocked_outlet_overpressure` — the charging and the
  blocked-outlet demand.
- `process.non_equilibrium_blowdown` — the two-zone vapour/liquid split.
- `process.multi_vessel_blowdown_study` — the shared-header superposition.
- `process.dynamic_psv_sizing` — the steady-versus-dynamic relief area.
- `process.psv_sizing`, `process.psv_fire_sizing`, `process.mdmt` — the relief and metal
  temperature screens.

## References

- API STD 521. Pressure-relieving and Depressurizing Systems.
- NORSOK S-001; STS 0131.
- NeqSim: https://github.com/equinor/neqsim
