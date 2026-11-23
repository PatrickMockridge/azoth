# The PVT regression

A measured PVT report is a set of experiments, and an equation of state reproduces it only
if its heavy end and its interaction parameters are tuned. This skill is that fit: a
weighted multi-target regression of a characterization factor or a binary interaction
parameter against CCE, CVD, DLE and viscosity data, with the per-target residuals and the
class's simplified uncertainty.

## When to Use

- When a heavy-end characterization factor (critical-temperature, pressure, acentric or
  volume-shift multiplier) must reproduce measured CCE, CVD, DLE or viscosity data.
- When a binary interaction parameter (methane–C7+, C2–C6–C7+, CO2–hydrocarbon,
  N2–hydrocarbon) must be fitted to several measurements at once.
- When the residual ledger and a standard error are needed before a rigorous EOS regression.

## Inputs

- Component rows: the base components and the characterized pseudo rows, in mixture order.
- Parameter rows: one of the twelve carried constants, each with its bounds and initial
  guess.
- Experiment rows grouped by an `experiment` index: CCE relative volumes, CVD liquid dropout
  and gas Z, DLE Rs/Bo/oil density, or one phase's viscosity at a state.
- The four experiment weights, the cubic and mixing rule, and the iteration cap.

## Outputs

- One fitted row per parameter, with its bounds; the standard errors and 95 % half-widths;
  one unweighted objective row per experiment; the weighted chi-square, degrees of freedom,
  residual variance, AAD percent, iteration count and convergence reason.

## How a calculation runs

Each parameter is applied to the fluid before assembly: the multipliers scale every pseudo
row's critical constants, the BIPs overwrite named pairs, and the volume-shift multiplier
scales the Peneloux constant through the Rackett compressibility. The experiment schedules
are flashed through the laboratory kernels, and Levenberg–Marquardt with Ridders
derivatives minimises the weighted chi-square: a five-iteration floor, two `1e-6` stops,
damping 10, and the initial guess evaluated without clamping.

**The weights divide the sample standard deviations, not the objectives.** The objective
rows are a second, unweighted ledger; a user reading both sees two different numbers. The
uncertainty is the class's simplified form - the chi-square over the degrees of freedom
times a tenth of the bound width - not the optimizer's covariance.

## Python usage pattern

```python
import azoth

# `rows` is the base and characterized pseudo-component row list.
result = azoth.pvt.regression(
    components=rows,
    parameters=[
        {"parameter": "tc_multiplier_c7plus", "initial": 1.0, "lower": 0.95, "upper": 1.05}
    ],
    cce=[{"experiment": 0.0, "temperature": 373.15, "pressure": 2.5e7, "relative_volume": 0.998}],
    max_iterations=50,
)
print(result.fitted_parameters[0].value, result.final_chi_square)
```

## The keycard

The base components' interaction coefficients come from the databank; a fitted BIP
overwrites the named pair for the fit only. A keycard can supply a substance the databank
lacks, but the fitted parameters are a result, not a card.

## Validation checklist

- [ ] Read the convergence reason: `max_iterations_reached` and `singular_matrix` are not
      convergence.
- [ ] Read the objective rows and the weighted chi-square separately.
- [ ] Confirm the fluid has pseudo rows before fitting a C7+ parameter.
- [ ] Compare a standard error with the bound width it was derived from.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| The objective row and the chi-square disagree | Two ledgers: one weighted, one not | Read each for its own purpose |
| A C7+ parameter does not move | The fluid has no pseudo rows | Characterize the heavy end first |
| A tiny "uncertainty" read as a covariance | The class's simplified form | Compare it with the bound width |
| No saturation-pressure or separator target | Neither is carried | Use another experiment or the class itself |

## Limitations

Five of the class's seventeen parameters are not carried: the plus-molar-mass multiplier,
the gamma shape and minimum molecular weight, and the LBC and Pedersen viscosity
parameters. The separator experiment is owed; `SATURATION_PRESSURE` and `SWELLING` are not
regression targets. The plus name must start with a carbon number for the split.

## Related Azoth functionality

`pvt.regression`, `pvt.constant_mass_expansion`, `pvt.constant_volume_depletion`,
`pvt.differential_liberation`, `eos.viscosity`, `characterization.characterise_plus_fraction`
- pages under `docs/src/pvt/`.

## References

- NeqSim `pvtsimulation/regression/PVTRegression.java`.
- Marquardt, D. W. (1963), *SIAM Journal on Applied Mathematics* 11(2), 431–441.
- `validation/neqsim/captures/pvt_regression.tsv`.
