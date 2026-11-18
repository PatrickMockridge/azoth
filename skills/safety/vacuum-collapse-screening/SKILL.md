# Vacuum collapse screening

Educational vacuum-collapse (implosion) screening for a blocked-in vessel that cools down, with vacuum-depth and external-rating flags. USE WHEN: a task needs a public, screening-level check of whether a closed vessel cooldown or steam/vapour condensation can pull a vacuum below the external pressure rating, without proprietary vacuum-collapse design methods.

This is a `screening` skill and **ported**. azoth's `process.vacuum_collapse` flashes the
blocked-in inventory at its initial state, holds the molar volume and runs a constant-volume
flash at every cooling step, then reports the minimum pressure against the external rating
and the make-up gas a setpoint would need. The class's cooling curve and its
unit-string setters are not carried: the caller states the mixture and the SI conditions.

## When to Use

Educational vacuum-collapse (implosion) screening for a blocked-in vessel that cools down, with vacuum-depth and external-rating flags. USE WHEN: a task needs a public, screening-level check of whether a closed vessel cooldown or steam/vapour condensation can pull a vacuum below the external pressure rating, without proprietary vacuum-collapse design methods.

## Inputs

- `components` and `mole_fractions`: the blocked-in inventory's substances and composition.
- `vessel_volume`, `initial_pressure`, `initial_temperature`: the rigid volume and the
  blocked-in state.
- `cold_end_temperature`: the coldest temperature reached, below the initial one.
- `atmospheric_pressure`: optional; absent is 101325 Pa.
- `external_pressure_rating`: optional; **absent is 0 Pa**, a full-vacuum-rated vessel.
- `makeup_setpoint`: optional; absent is the atmospheric pressure.
- `makeup_gas_molar_mass`: optional; absent is 0.0280134 kg/mol (nitrogen).
- `cooling_steps`: optional positive integer; absent is 20.

## Outputs

- `final_pressure`: the pressure at the cold end.
- `minimum_pressure` and `minimum_pressure_temperature`: the smallest pressure on the curve
  and where it is reached.
- `vacuum_depth`: `max(0, atmospheric_pressure - minimum_pressure)`.
- `margin_to_rating`: `minimum_pressure - external_pressure_rating`.
- `vacuum_present` and `within_rating`.
- `makeup_gas_moles` and `makeup_gas_mass`: the make-up a setpoint would need.
- `verdict`: `no_vacuum`, `vacuum_within_rating` or `vacuum_exceeds_rating`.

## How a calculation runs

A TP flash at the initial pressure and temperature fixes the mixture's molar volume, and the
sweep holds it while a constant-volume flash runs at `T_i = T0 + (Tc - T0) i/steps` for
`i = 1..steps`. The final pressure is the last step's; the minimum is the smallest on the
curve, which can be interior when condensation is not monotonic. A vacuum is present when the
minimum is below atmosphere; it exceeds the rating when the minimum is below the external
rating. Make-up is `V (setpoint - pminimum)/(R Tminimum)` moles when the minimum is below
the setpoint.

The cubic is the databank PR. **An absent rating is not a missing verdict**: the class assumes
a full-vacuum-rated 0 bara vessel and still answers. A cold end at or above the initial
temperature is refused, as the class refuses it.

## Python usage pattern

```python
import azoth

result = azoth.process.vacuum_collapse(
    components=["methane", "propane"],
    mole_fractions=[0.2, 0.8],
    vessel_volume=10.0,
    initial_pressure=5.0e5,
    initial_temperature=300.0,
    cold_end_temperature=180.0,
    external_pressure_rating=2.0e4,
)
print(result.minimum_pressure.to("bara"))  # 0.652 bara
print(result.verdict)                      # vacuum_within_rating
```

## The keycard

The components resolve against the databank, so a name the databank does not carry is refused
rather than modelled. The interaction parameters are the databank PR pair.

## Validation checklist

- [ ] Confirm the task actually needs a cooldown sweep rather than a thermal in-breathing
      screen (API 2000) or a design check (UG-28).
- [ ] State the composition, not a pseudo-fluid: the sweep is a flash of the mixture.
- [ ] Give the real external-pressure rating. An absent rating assumes 0 bara and will report
      a vacuum within rating that the vessel may not have.
- [ ] Check the cold end is the coldest state the operation can reach, not the ambient.
- [ ] Read the verdict with the make-up gas: a within-rating vacuum can still need a vacuum
      breaker or pad gas.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A vacuum is within rating on an unrated vessel | The rating was omitted, so 0 bara was assumed | State the vessel's real external-pressure rating |
| The sweep answers no vacuum for a condensable stream | The cold end is above the mixture's condensation range | Lower the cold end or check the composition |
| A cold end above the initial temperature is refused | The sweep is a cooling case | State a colder end, or run a heating case elsewhere |
| The pressure rises during the sweep | The mixture is non-condensing and the ideal-gas relation dominates | Expected for a dry gas |

## Limitations

The id carries the class's cooling sweep, not a transient heat balance: the cooling schedule
is the stated linear interpolation, and the vessel's thermal mass and heat-transfer path are
the caller's. The cooling curve is presentation and not carried.

## Related Azoth functionality

- `process.settle_out_pressure` — the connected-volume settle-out pressure after a blocked-in
  release.
- `process.mdmt` — the cold-service metal-temperature screen for the vessel the sweep cools.
- `process.gas_blowby` — the pressure-driven path into a low-pressure system.

## References

- API STD 521. Pressure-relieving and Depressuring Systems.
- API STD 2000. Venting Atmospheric and Low-pressure Storage Tanks.
- ASME Boiler and Pressure Vessel Code, Section VIII Division 1, UG-28.
- NeqSim: https://github.com/equinor/neqsim
