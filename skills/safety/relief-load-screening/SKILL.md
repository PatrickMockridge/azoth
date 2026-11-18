# Relief load screening

Educational fire-case relief load screening indicators for pressure relief devices. USE WHEN: a task needs public, screening-level fire-case relief load estimates and capacity flags without proprietary relief design methods.

**Ported.** `process.fire_case_relief` computes the fire-case relief rate,
`process.psv_fire_sizing` sizes the device and `process.overpressure_acceptance` tests the
resulting peak against the allowable pressure.

## When to Use

A vessel or a segment exposed to a pool fire whose relief load, device area or peak pressure
is being screened; or the fire case's contribution to a relief disposal network.

## Inputs

- **Relief load** (`process.fire_case_relief`): `has_drainage` and `has_fire_fighting`, and
  the wetted area or the vessel geometry, the latent heat in J/kg (declared dimensionless) and
  the relieving temperature.
- **Fire sizing** (`process.psv_fire_sizing`): the initial mass and pressure, the set
  pressure, the fire heat input, the vessel volume, the gas properties and the blowdown time.
- **Acceptance** (`process.overpressure_acceptance`): the peak pressure, the MAWP, the cause
  and whether multiple devices are credited.

## Outputs

- The relief rate in kg/s and kg/h, and the latent heat used.
- The fire sizing's required area, its API orifice letter, the blowdown pressure and the
  flow coefficient.
- The accumulation factor, the allowable pressure and whether the peak is accepted.

## How a calculation runs

The fire-case load is the class's two heat-input coefficients, 21000 or 34500
Btu/(hr ft²), over the wetted area or the `pi D h` fallback, divided by the latent heat;
drainage lowers the coefficient and the fire-fighting flag is read by no formula. The dynamic
sizing takes `max(peak, average)` over the one-minute heat-up at 21% overpressure. The
acceptance applies the 1.10/1.16/1.21 factors and accepts at equality.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

relief = azoth.process.fire_case_relief(
    has_drainage=True,
    has_fire_fighting=True,
    wetted_area=quantity(100.0, "m**2"),
    latent_heat=300000.0,
    relieving_temperature=quantity(350.0, "K"),
)
print(relief.relief_rate.to("kg/s"))  # 6.2847
print(relief.relief_rate_kg_per_hr.to("kg/h"))  # 22625
```

## The keycard

The fire-case id takes the latent heat directly; no fluid is flashed. The thermo-bound relief
loads (`process.blocked_outlet_relief`, `process.control_valve_failure_relief`,
`process.check_valve_leak_relief`, `process.tube_rupture_relief`) evaluate a fluid and are a
different route.

## Validation checklist

- [ ] State the drainage and fire-fighting basis the area actually has.
- [ ] Use the relieving latent heat, not the normal-boiling-point value.
- [ ] Check the wetted area against the vessel's real geometry and its liquid level.
- [ ] Carry the load into the sizing, then the peak into the acceptance.
- [ ] Remember the fire-fighting flag is read by no formula.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A load twice the expected | Drainage omitted, so the higher coefficient was used | State the drainage basis |
| A tiny relief rate | The latent heat is too high or the wetted area too small | Recheck both |
| A peak accepted at the MAWP | The cause's accumulation factor was not stated | State the cause and the device count |
| The fire-fighting flag changes nothing | It is carried and unread | Expected; the class reads no formula |

## Limitations

The ids are the class's screening forms, not a relief study: they do not model the fire's
growth, the vessel's wall temperature or the disposal network's back pressure. The class's
prose is presentation.

## Related Azoth functionality

- `process.psv_sizing` — the general relief-valve area.
- `process.pipeline_pressure_protection` — the pipeline's overpressure barriers.
- `process.relief_disposal_network` — the simultaneous loads on a disposal system.

## References

- API STD 521. Pressure-relieving and Depressuring Systems.
- API STD 520 Part I. Sizing, Selection, and Installation of Pressure-relieving Devices.
- NeqSim: https://github.com/equinor/neqsim
