# Psv orifice screening

Educational pressure-safety-valve orifice screening using the public API 520 Part I critical gas-flow equation. USE WHEN: a task needs a public, screening-level required PSV orifice area and a mapped API orifice letter before detailed relief-valve sizing and selection.

**Ported.** `process.psv_sizing` evaluates the API 520 gas, liquid and two-phase areas, the
corrections the class applies and the API orifice letter ladder.

## When to Use

A pressure-relief valve's required orifice area from a stated relieving duty; a first-pass
orifice letter before vendor selection; or a screen of an existing valve's capacity.

## Inputs

- `service`: `gas`, `liquid` or `two_phase`.
- `set_pressure` in Pa, `overpressure_fraction`, and `back_pressure` in Pa.
- For gas: `mass_flow`, `temperature`, `molar_mass`, `compressibility` and
  `specific_heat_ratio`, with optional balanced-bellows and rupture-disk flags.
- For liquid: `volume_flow`, `liquid_density`, `viscosity` and `liquid_heat_capacity`.
- For two-phase: `gas_fraction`, `gas_density`, `latent_heat` and the phase properties.

## Outputs

- `relieving_pressure` and `back_pressure_fraction`.
- `discharge_coefficient`, `required_area` in m² and `required_area_in2`.
- `recommended_orifice`: the API letter, and `selected_area_in2` from the ladder.
- The back-pressure, combination and viscosity corrections.
- The mass-flow capacity when the caller states an existing orifice.

## How a calculation runs

The gas area is the critical-flow API 520 form when the back pressure is below the critical
ratio and the subcritical form above it; the liquid form uses the required flow and the
`Kw`, `Kv` and `Kp` corrections; the two-phase form is the omega method. The selected area is
the first API letter at or above the required area in².

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.psv_sizing(
    service="gas",
    set_pressure=quantity(10.0, "bar"),
    overpressure_fraction=0.10,
    back_pressure=quantity(1.01325, "bar"),
    mass_flow=quantity(1.0, "kg/s"),
    temperature=quantity(300.0, "K"),
    molar_mass=quantity(0.020, "kg/mol"),
    compressibility=0.9,
    specific_heat_ratio=1.3,
)
print(result.required_area_in2)  # 0.7249
print(result.recommended_orifice)  # H
```

## The keycard

The gas branch takes the molar mass directly; no component is resolved and no fluid is
flashed. For a fluid-bound relief load, use the thermo-bound ids.

## Validation checklist

- [ ] Confirm the service and the phase at the relieving state.
- [ ] State the real back pressure, and whether the valve is balanced-bellows.
- [ ] Use the relieving temperature and the compressibility at that state.
- [ ] Check the correction factors; a high back pressure moves the flow subcritical.
- [ ] Treat the orifice letter as a screening selection, not a vendor datasheet.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A gas area far below the vendor's | A liquid correlation was used, or the compressibility was omitted | Use the gas service and state Z |
| The recommended letter is above the selected | The ladder starts at the required area | Expected; the selected letter is the class's |
| A large back-pressure correction | The valve was assumed conventional at a high back pressure | State balanced bellows if it is |

## Limitations

The id is the class's API 520 implementation, not a certified sizing: it does not select a
vendor, a body size, an inlet/outlet line or a certified coefficient. The eight other sizing
standards the class names are refused by the port.

## Related Azoth functionality

- `process.psv_fire_sizing` — the fire-case dynamic sizing.
- `process.fire_case_relief` — the relief load a fire case produces.
- `process.overpressure_acceptance` — whether the resulting peak is accepted.

## References

- API STD 520 Part I. Sizing, Selection, and Installation of Pressure-relieving Devices.
- NeqSim: https://github.com/equinor/neqsim
