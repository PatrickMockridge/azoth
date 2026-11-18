# Firewater deluge design

Educational fire-water and deluge coverage screening: area versus dedicated object demand, deluge nozzle-net sizing from both the flow and the spacing criterion, and fire-monitor screening with wind drift. USE WHEN: a task asks whether a process area has adequate fire-water coverage, how many deluge nozzles are needed, or whether monitors or passive fire protection can substitute for a fixed system.

**Ported.** Five ids carry the arithmetic: `process.fire_water_demand`,
`process.deluge_nozzle_layout`, `process.fire_monitor_coverage`,
`process.fire_water_coverage_assessment` and `process.active_fire_protection_screening`.

## When to Use

A process area or a dedicated object whose fire-water demand, deluge layout, monitor coverage
or supply adequacy is being screened; or a fire where the choice between fixed water, monitors
and passive protection is being made.

## Inputs

- **Demand** (`process.fire_water_demand`): `protected_area` in m², `application_rate` in
  L/(min m²) (declared dimensionless), `duration` in s, and optional foam concentrate,
  simultaneous-area factor and dedicated object rows.
- **Deluge layout** (`process.deluge_nozzle_layout`): the protected area, the required
  density, the nozzle K, its minimum pressure in bar, the maximum spacing and the two
  clearances.
- **Monitor coverage** (`process.fire_monitor_coverage`): the target area, the required
  density, and optional monitor count, flow, throw, sweep, wind and fall height.
- **Supply assessment** (`process.fire_water_coverage_assessment`): the protected area, the
  required and spare flows, the worst-case pressure margin, and optional covered area,
  delivered flow and required margin.
- **Protection screening** (`process.active_fire_protection_screening`): the scenario enum,
  the blowdown and bare-steel times, and the dedicated-object and general-area demands.

## Outputs

- Demand in L/min and m³/h, the water volume and the foam concentrate.
- The deluge flow per nozzle, both nozzle counts, the governing criterion, the grid spacing,
  the delivered density and the pressure, density and clearance flags.
- The monitor flow, swept area, wind drift, effective density and its verdict.
- The supply coverage gap, flow deficit and feasibility flags.
- The protection screen's extinguishing flag, blowdown margin and selective-protection saving.

## How a calculation runs

The demand is the application rate over the protected area, plus the dedicated objects, times
the duration for the volume. The deluge net is `max(count from flow, count from coverage)`,
with the grid spacing checked against the maximum and the delivered density against the
requirement. The monitor's swept area is reduced by the wind drift before its effective
density is formed. The supply assessment compares the covered area and delivered flow with
what is required. The protection screen tests whether the blowdown reaches the target faster
than the bare steel fails.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

demand = azoth.process.fire_water_demand(
    protected_area=quantity(510.0, "m**2"),
    application_rate=10.0,
    duration=quantity(1800.0, "s"),
)
print(demand.total_demand_m3_per_hour.to("m**3/h"))  # 306
print(demand.water_volume.to("m**3"))  # 153
```

## The keycard

The ids are arithmetic over the caller's demands and geometry; no fluid is flashed and no
component is resolved. The application rate and the required density are the standard's
values, and the caller states them.

## Validation checklist

- [ ] State the application rate from the governing standard, not a default.
- [ ] Add the dedicated objects; an area-only demand can understate the total.
- [ ] Check both the flow and the spacing criterion for the nozzle count.
- [ ] Apply the wind drift to the monitor's swept area.
- [ ] Compare the supply with the worst-case pressure margin, not only the flow.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| The demand is an area-only number | Dedicated objects were omitted | Add the object rows |
| The nozzle count is flow-governed but the spacing fails | Only one criterion was read | Read the governing criterion and the grid spacing |
| The monitor density passes in still air but fails in wind | The drift was not applied | Read the wind coverage fraction |
| A supply passes on flow but not pressure | The pressure margin was not stated | State the worst-case margin |

## Limitations

The ids are screening arithmetic, not hydraulic network design: pipe friction, pump curves and
the simultaneous operation of several systems are the caller's. The classes' prose and their
report strings are presentation.

## Related Azoth functionality

- `process.active_fire_protection_screening` — the water-versus-passive choice.
- `process.pfp_demand` — the passive protection demand when water is not the answer.
- `process.api537_flare_flame_model` — the radiation a flare adds to the area.

## References

- NFPA 15. Standard for Water Spray Fixed Systems for Fire Protection.
- API RP 2030. Application of Fixed Water Spray Systems for Fire Protection.
- NeqSim: https://github.com/equinor/neqsim
