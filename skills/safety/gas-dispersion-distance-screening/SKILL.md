# Gas dispersion distance screening

Educational Gaussian point-source gas-dispersion screening that estimates the downwind distance to a target concentration (LFL or toxic ppm) using the public Pasquill-Gifford / Briggs rural model. USE WHEN: a task needs a public, screening-level distance-to-LFL or distance-to-ppm estimate for a continuous gas release before detailed dispersion or consequence analysis.

**Ported.** `process.gaussian_plume` evaluates the sigma curves, the two-image concentration
and the log-scan distance to a threshold. `process.gas_dispersion` is the full analyzer: a
released mixture's ambient flash, its Le Chatelier LFL, the model selection and the flammable
and toxic endpoints over the passive plume or the dense Britter-McQuaid fit.

## When to Use

A continuous release whose downwind distance to a flammable or toxic threshold is being
screened; a dispersion contour for a consequence study; or the choice between a passive plume
and a dense-gas fit.

## Inputs

`process.gaussian_plume`: `emission_rate` in kg/s, `effective_stack_height` in m, `wind_speed`
in m/s, the `stability` class (`A` to `F`), the `terrain` (`rural` or `urban`), the receptor
`distance`, `crosswind` and `height` in m, and the `threshold` concentration in kg/m³.

`process.gas_dispersion`: the release `components` and `composition`, the `mass_release_rate`,
and optional ambient conditions, stability, roughness, release height, model selection and the
flammable or toxic target.

## Outputs

- `gaussian_plume`: `sigma_y`, `sigma_z`, the concentration at the receptor, the centreline
  ground concentration and the distance to the threshold when it exists on the scan.
- `gas_dispersion`: the selected model, the source and air densities, the LFL and its
  endpoint fraction, the distances to the flammable endpoint and to half the LFL, the cloud
  volume, the toxic distance and the two endpoint flags.

## How a calculation runs

The plume's sigma curves are the Pasquill-Gifford rural correlations, with the urban forms
where the terrain says so; the concentration is the two-image sum, and the distance to a
threshold is a log scan from the source. The analyzer flashes the released composition at the
ambient state, forms the gas phase's Le Chatelier LFL, selects the passive or dense model by
its own density-ratio test, and reports the endpoint distances.

## Python usage pattern

```python
import azoth
from azoth.core.units import quantity

result = azoth.process.gaussian_plume(
    emission_rate=quantity(1.0, "kg/s"),
    effective_stack_height=quantity(0.0, "m"),
    wind_speed=quantity(5.0, "m/s"),
    stability="D",
    terrain="rural",
    distance=quantity(100.0, "m"),
    crosswind=quantity(0.0, "m"),
    height=quantity(0.0, "m"),
    threshold=quantity(0.05, "kg/m**3"),
)
print(result.concentration.to("kg/m**3"))
print(result.distance_to_threshold)  # absent when the scan never reaches it
```

## The keycard

`process.gaussian_plume` is arithmetic over the caller's release state; no fluid is flashed.
`process.gas_dispersion` resolves its `components` against the databank and flashes the
mixture, so a name the databank does not carry is refused.

## Validation checklist

- [ ] State the stability class and the terrain the site actually has.
- [ ] Use the release's real stack or release height, not zero by default.
- [ ] For a mixture, state the composition rather than a pseudo-gas.
- [ ] Read the endpoint with its basis: LFL fraction, half-LFL and toxic threshold are three
      different distances.
- [ ] Check whether the release is dense enough for the dense-gas fit.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| The distance to threshold is absent | The scan never reaches the threshold from the source | Check the emission rate and the threshold |
| A passive plume on a dense release | The stability or the model selection ignored the density | Let the analyzer select the model, or state it |
| A toxic distance from the wrong component | The toxic component and threshold were not both stated | State both |
| An LFL distance shorter than the half-LFL distance | The two endpoints were confused | Read each with its fraction |

## Limitations

Both ids are screening models. The plume is a steady Gaussian form; the analyzer's dense fit is
Britter-McQuaid. Neither resolves buildings, terrain or a time-varying release. The DTO's
scenario name and JSON are session surface.

## Related Azoth functionality

- `process.probit` — the consequence of the concentration at the receptor.
- `process.consequence_analysis` — the QRA engine that composes the models.
- `process.toxic_library` — the IDLH and ERPG thresholds a toxic target may come from.

## References

- Pasquill, F. and Smith, F.B. Atmospheric Diffusion.
- Briggs, G.A. Diffusion Estimation for Small Emissions.
- Britter, R.E. and McQuaid, J. Workbook on the Dispersion of Dense Gases.
- NeqSim: https://github.com/equinor/neqsim
