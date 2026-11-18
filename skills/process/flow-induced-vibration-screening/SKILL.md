# Flow induced vibration screening

Educational flow-induced vibration (FIV) screening using a public fluid kinetic-energy (rho v^2) likelihood-of-failure index. USE WHEN: a task needs a public, screening-level check of whether a main-line flow velocity and density produce a kinetic-energy level that warrants a detailed Energy Institute style FIV assessment before piping vibration design.

**Ported.** `process.piping_fiv_gas` and `process.piping_fiv_liquid` evaluate the AVIFF
likelihood with the D/t, branch, pulsation and support factors; the gas and liquid methods
are split because they take different inputs and different energy terms.

## When to Use

A main-line piping circuit whose flow energy may drive vibration: a gas line's momentum
flux, a liquid line's velocity, a high branch count or a poor support arrangement. The
screen answers whether the circuit warrants a detailed dynamic assessment.

## Inputs

Gas (`process.piping_fiv_gas`):

- `density` and `velocity`: at line conditions, kg/m³ and m/s.
- `outside_diameter` and `wall_thickness`: m.
- `branch_count`: the number of small-bore connections, a whole number.
- `pulsation_factor`: 1 for steady service to 4 for a reciprocating compressor's near
  field; **clamped to [1, 4]**.
- `support_quality_factor`: 1 for good supports to 3 for poor ones; **clamped to [1, 3]**.

Liquid (`process.piping_fiv_liquid`): `velocity`, the same geometry and branch count, and
the support factor. There is no density or pulsation term.

## Outputs

- `lof`: the normalised likelihood of failure.
- `likelihood`: `low`, `medium`, `high` or `very_high`; the bands are exclusive upward, so
  0.3 is medium, 0.5 high and 1.0 very high.
- `rho_v2` (gas), `d_overt`, `d_overt_factor`, `branch_factor`, `energy_term` and the
  applied factors. The D/t factor is floored at one for D/t at or below 40.

## How a calculation runs

Gas: `rho v^2 / 50000 * D/t factor * (1 + 0.15 n) * pulsation * support`. Liquid:
`v / 6 * D/t factor * (1 + 0.10 n) * support`. The class clamps the pulsation and support
factors silently and the id publishes the applied values; a value outside the ranges also
raises a warning, so the clamp is visible.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.piping_fiv_gas(
    density=quantity(1.0, "kg/m**3"),
    velocity=quantity(10.0, "m/s"),
    outside_diameter=quantity(0.2, "m"),
    wall_thickness=quantity(0.01, "m"),
    branch_count=0,
    pulsation_factor=1.0,
    support_quality_factor=1.0,
)
print(result.lof)  # 0.002
print(result.likelihood)  # low
```

## The keycard

No fluid is flashed: the density or velocity is the caller's line-condition value. The
screens are arithmetic over that state, not a property lookup.

## Validation checklist

- [ ] Choose the gas or liquid id by the service, not by the stream's majority phase.
- [ ] Count the small-bore connections; each one raises the branch factor.
- [ ] State the real pulsation and support factors and read the applied values back.
- [ ] Use the line conditions at the worst operating case, not the design maximum only.
- [ ] For a side branch, run `process.flow_induced_pulsation_screening` as well: it is a
      different mechanism.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A low band on a circuit known to vibrate | The pulsation or support factor was left at 1 | State the real factors; the clamps are visible |
| The gas id is used for a liquid line | The service was assumed from the fluid name | Use `process.piping_fiv_liquid` |
| The D/t factor is one on a thick pipe | D/t at or below 40 is the floor | Expected; the factor is not meant to fall below one |
| A branch rings but the main-line band is low | Main-line FIV and side-branch pulsation are different mechanisms | Run the pulsation screen |

## Limitations

The score is the Energy Institute AVIFF screening index, not a vibration amplitude or a
fatigue life. The class's recommendation text is not published.

## Related Azoth functionality

- `process.acoustic_induced_vibration_screening` — the downstream-of-a-let-down device
  screen.
- `process.flow_induced_pulsation_screening` — the side-branch acoustic resonance screen.
- `process.pipe` — the line's steady hydraulics, where the velocity and density come from.

## References

- Energy Institute. Guidelines for the Avoidance of Vibration Induced Fatigue Failure in
  Process Pipework (AVIFF), 2nd edition.
- NeqSim: https://github.com/equinor/neqsim
