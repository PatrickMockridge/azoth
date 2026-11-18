# Flare radiation screening

Educational flare thermal-radiation screening using the public API 521 / API 537 point-source equation. USE WHEN: a task needs a public, screening-level estimate of radiant heat flux at a distance from a flare and a check against allowable radiation limits before detailed flare and radiation design.

**Ported.** `process.api537_flare_flame_model` evaluates the flame geometry, the ground flux at
a distance, the sterile radii at target fluxes and the two sound levels. `process.jet_fire` is
the same point-source form for an ignited jet.

## When to Use

A flare stack or a jet release whose thermal hazard at a receptor distance is being screened:
a sterile radius, a personnel limit, or the flux on an adjacent structure.

## Inputs

- `mass_flow` in kg/s, `heat_of_combustion` in J/kg (declared dimensionless), and
  `radiative_fraction`.
- `exit_velocity` in m/s, and the receptor `ground_distance` and `sound_distance` in m.
- `target_fluxes`: a record list of the flux values whose sterile radii are wanted, W/m².
- Optional `stack_height` and `wind_speed`, and the class's optional transmissivity and
  acoustic efficiency.

## Outputs

- `heat_release` and `radiative_power`.
- The flame length, tilt, centre and tip positions.
- `heat_flux_at_ground_distance`, and one sterile radius per target flux.
- `sound_power_level`, `sound_pressure_level` and the equivalent flame temperature.

## How a calculation runs

The class forms the heat release, its radiative part, Kent's flame length and the `atan2`
tilt, then the point-source flux at the receptor. Each target flux has a sterile radius: the
distance at which that flux is reached. The two sound levels and the equivalent flame
temperature are the class's own forms. A negative stack height, which its setter accepts, is
refused.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.api537_flare_flame_model(
    mass_flow=quantity(10.0, "kg/s"),
    heat_of_combustion=50.0e6,
    radiative_fraction=0.2,
    exit_velocity=quantity(30.0, "m/s"),
    ground_distance=quantity(100.0, "m"),
    sound_distance=quantity(100.0, "m"),
    target_fluxes=[{"flux": 6250.0}, {"flux": 12500.0}],
    stack_height=quantity(80.0, "m"),
)
print(result.heat_flux_at_ground_distance.to("W/m**2"))
print(result.sterile_radii)
```

## The keycard

The id is arithmetic over the release's heat and geometry; no fluid is flashed. The heat of
combustion, the radiative fraction and the exit velocity are the caller's.

## Validation checklist

- [ ] Confirm the release is a flare or an ignited jet, not a pool fire or a BLEVE.
- [ ] State the radiative fraction from the gas, not a default.
- [ ] Use the real stack height and the receptor's ground distance.
- [ ] Read the sterile radius at the governing limit, not the area-average flux.
- [ ] Check the sound level if the receptor is also a noise-sensitive location.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| A sterile radius much smaller than expected | The radiative fraction is too low or the stack too tall | Recheck both against the flare basis |
| A negative stack height is refused | The class accepts it silently; the port refuses it | State a positive height |
| The point source and a detailed model disagree | The point-source form ignores the flame's shape | Use the radius as a screen, then a detailed model |

## Limitations

The point-source model is a screening form: it does not resolve the flame's finite size,
shielding, or the wind's effect on the flame shape beyond the class's own correction. The
class's fit is its own approximation.

## Related Azoth functionality

- `process.jet_fire` — the same radiation form for a jet.
- `process.pool_fire` and `process.bleve` — the other fire consequences.
- `process.pfp_demand` — the protection a structure needs against the flux.

## References

- API STD 521. Pressure-relieving and Depressuring Systems.
- API STD 537. Flare Details for General Refinery and Petrochemical Service.
- NeqSim: https://github.com/equinor/neqsim
