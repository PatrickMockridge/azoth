# Jet fire radiation screening

Educational jet-fire thermal-radiation screening that estimates the radiative flux at a distance, or the distance to a target flux, using the public single-point-source flame radiation model. USE WHEN: a task needs a public, screening-level jet-fire radiation flux or a distance to a personnel/equipment thermal limit before detailed fire-consequence analysis.

**Ported.** `process.jet_fire` evaluates the total radiative power, Wayne's transmissivity,
the incident flux at a distance and the distance to a target flux.

## When to Use

A pressurised gas jet that may ignite: a flange leak, a relief tailpipe or a compressor seal
failure. The screen answers the flux at a receptor or the distance to a thermal limit.

## Inputs

- `mass_flow` in kg/s, `heat_of_combustion` in J/kg (declared dimensionless), and
  `radiative_fraction`.
- Optional `relative_humidity`; the class's transmissivity correction uses it.
- Optional `distance` in m, for the flux at a receptor, and optional `target_flux` in W/m²,
  for the distance to that flux.

## Outputs

- `total_radiative_power` in W.
- `transmissivity` when the humidity is stated.
- `incident_heat_flux` at the stated distance.
- `distance_to_flux` for the stated target flux.

## How a calculation runs

The total radiative power is the mass flow times the heat of combustion times the radiative
fraction. Wayne's transmissivity reduces the flux over the path, and the point-source flux is
`power * transmissivity / (4 pi r^2)`. The distance to a target flux is the same relation
solved for the radius.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.jet_fire(
    mass_flow=quantity(2.0, "kg/s"),
    heat_of_combustion=50.0e6,
    radiative_fraction=0.2,
    distance=quantity(20.0, "m"),
    target_flux=quantity(12500.0, "W/m**2"),
)
print(result.total_radiative_power.to("W"))  # 2.0e7
print(result.incident_heat_flux.to("W/m**2"))  # 2119.4
print(result.distance_to_flux.to("m"))  # 8.78
```

## The keycard

The id is the point-source arithmetic over the caller's release state; no fluid is flashed and
no component is resolved.

## Validation checklist

- [ ] Confirm the release is a jet, not a pool fire or a flash fire.
- [ ] State the radiative fraction; a hydrogen or high-soot gas is not the same as methane.
- [ ] Use the receptor's real distance and the governing target flux.
- [ ] Read the distance to the personnel limit and the equipment limit separately.
- [ ] Carry a high consequence to a detailed flame model.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A flux with no distance to it | Only `distance` was stated | State `target_flux` for the inverse |
| A transmissivity missing | The humidity was omitted | State `relative_humidity` |
| The flux falls as a square law | The point source is an approximation | Expected at screening distances |

## Limitations

The single-point-source model treats the flame as a point; it does not resolve the flame
shape, tilt or lift-off, and it is a screen rather than a consequence calculation. The class's
own clamps are mirrored.

## Related Azoth functionality

- `process.pool_fire` and `process.bleve` — the other fire consequences.
- `process.pfp_demand` — the protection a structure needs against the flux.
- `process.api537_flare_flame_model` — the flare's own radiation form.

## References

- API STD 521. Pressure-relieving and Depressuring Systems.
- Wayne, F.D. (1991). Fire radiation through the atmosphere.
- NeqSim: https://github.com/equinor/neqsim
