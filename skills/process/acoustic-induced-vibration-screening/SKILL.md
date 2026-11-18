# Acoustic induced vibration screening

Educational acoustic-induced-vibration (AIV) screening using the public Energy Institute likelihood-of-failure form for gas pressure-reducing devices. USE WHEN: a task needs a public, screening-level estimate of sound power level, pipe diameter-to-thickness ratio, allowable sound power level, and an AIV likelihood-of-failure index for a control valve, relief valve, or restriction orifice before detailed vibration assessment.

**Ported.** `process.acoustic_induced_vibration_screening` evaluates the Carucci-Mueller
sound power level, the design-line allowable and the margin's likelihood band from SI inputs.
The class's recommendation text is presentation.

## When to Use

A control valve, relief valve, restriction orifice or any let-down device whose downstream
pipe may be driven by the acoustic field it creates. The screen answers whether the pipe's
D/t is enough for the sound power the device produces.

## Inputs

- `mass_flow`: the gas mass flow through the device, kg/s, above zero.
- `upstream_pressure` and `downstream_pressure`: absolute pressures in Pa; the downstream
  must be at least zero and strictly below the upstream.
- `temperature`: the downstream temperature, K.
- `molecular_weight`: the gas molar mass, **kg/mol**; the correlation's g/mol form is an
  internal conversion.
- `outside_diameter` and `wall_thickness`: the downstream pipe, m.

## Outputs

- `pwl` and `allowable_pwl`: the sound power level and the Carucci-Mueller design-line
  allowable, in dB.
- `margin`: `pwl - allowable_pwl`, dB; positive is an exceedance.
- `pressure_drop_ratio`: `(P1 - P2)/P1`.
- `d_overt`: the diameter-to-thickness ratio.
- `likelihood`: `low`, `medium`, `high` or `very_high`; the bands are exclusive upward, so
  -10 dB is medium, 0 high and +10 very high.

## How a calculation runs

The class evaluates `10 log10[(dP/P1)^3.6 W^2 (T/MW)^1.2] + 126.1` in its imperial form,
with W in lb/s and T in degrees Rankine; the id converts the SI inputs first. The allowable
is `173.6 - 0.125 D/t`, and the margin's band is the class's -10/0/+10 dB ladder. A
downstream pressure at or above the upstream one is refused; zero is a full let-down.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.acoustic_induced_vibration_screening(
    mass_flow=quantity(1.0, "kg/s"),
    upstream_pressure=quantity(10.0, "bar"),
    downstream_pressure=quantity(9.0, "bar"),
    temperature=quantity(300.0, "K"),
    molecular_weight=quantity(0.020, "kg/mol"),
    outside_diameter=quantity(0.2, "m"),
    wall_thickness=quantity(0.01, "m"),
)
print(f"{result.pwl:.2f} dB against {result.allowable_pwl:.2f} dB")
print(result.likelihood)  # low
```

## The keycard

The id takes the molar mass directly; no component resolves against the databank and no
fluid is flashed. A name-keyed fluid would be a different calculation.

## Validation checklist

- [ ] Confirm the device is a gas let-down; a liquid device is not this mechanism.
- [ ] State the downstream pipe's real wall thickness: the allowable moves 0.125 dB per unit
      D/t.
- [ ] Read the margin, not only the band: a medium band can still be a negative margin.
- [ ] Carry the result to an acoustic-fatigue assessment when the band is high or very high.
- [ ] Check the upstream pressure is absolute and the downstream is below it.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A very high band on a modest flow | The molar mass was given in kg/mol where g/mol was meant, or the downstream pressure is nearly the upstream | State kg/mol; check both pressures |
| The screen refuses the state | The downstream pressure equals or exceeds the upstream | State a real let-down |
| The margin is positive but the band is medium | The band is the class's ladder, not the sign of the margin | Read the margin for severity |

## Limitations

The screen is a screening category, not a fatigue calculation: it does not predict the
vibration amplitude, the small-bore connection stresses or the pipe's fatigue life. The
class's recommendation text is not published.

## Related Azoth functionality

- `process.piping_fiv_gas` and `process.piping_fiv_liquid` — the main-line flow-induced
  vibration screens.
- `process.flow_induced_pulsation_screening` — the side-branch acoustic resonance screen.
- `process.psv_sizing` — the relief area when the device is a relief valve.

## References

- Carucci, V.A. and Mueller, R.T. (1982). Acoustically induced piping vibration in
  high-capacity compressors and control valves.
- Energy Institute. Guidelines for the Avoidance of Vibration Induced Fatigue Failure in
  Process Pipework (AVIFF), 2nd edition.
- NeqSim: https://github.com/equinor/neqsim
