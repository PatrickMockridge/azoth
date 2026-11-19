# Safety studies

Choose **Safety study** from the **Case** menu to open a safety study: one TOML document holding
safety scenarios and bow-ties, beside the flowsheet and the calculation book. It is its own
document, with its own commands and its own envelope; switching interfaces never throws it away.

## The study

A study has an id, a name, a list of **scenarios** and a list of **bow-ties**. Results are
derived and never saved: a scenario's answer is the last calculate's, and a scenario whose
inputs changed says its answer is out of date.

The **Overview** lists the study's bow-ties and scenarios, every diagnostic the study carries,
and the whole safety surface grouped by family. Thirty-one scenario models across ten
families — fire, dispersion, equipment safety, firewater, overpressure, process safety systems,
risk, safety assessment, vibration and depressurization — and the bow-tie run inside this
interface. The rest — the models that take a fluid or a table of rows, and the rupture,
release and QRA families — are listed with links to their manual pages; they are not runnable
here yet. The four thermo-bound relief loads and the overpressure protection study are in that
list: they take a fluid or a scenario table, and the library reaches them.

## Bow-ties

A bow-tie names the **threats** that can cause a top event, the **barriers** between them and
the **consequences** on the other side. The diagram draws threats on the left, a shared
prevention wall, the top event, a mitigation wall and the consequences on the right; select a
plate to edit it.

**What the arithmetic does is stated on the drawing.** `process.bow_tie` sums the active
threats and multiplies every functional `prevention` or `both` barrier's PFD once. A
`mitigation` barrier is recorded and read by no formula, so the plate says so; a non-functional
plate says inactive. The model reads no threat-to-barrier link, so the prevention wall is drawn
against the aggregate rather than attributing a barrier to one cause.

A consequence may name a scenario that models it. That is a presentation link: the severity is
still the number the model reads, and a link to a scenario the study does not carry is a
warning that does not stop the calculation.

The **RRF** is the unmitigated frequency over the mitigated one, which is the model's answer.
Drag a plate and the position is a document edit; a position does not make a result out of
date.

## Scenario models

A scenario is one registered model and the inputs it was given. Every safety model whose
inputs and outputs are scalar can run here; a model that takes or returns a table of rows is
listed with its refusal. The decks group them by family.

### Fire and explosion

| model | what it answers |
|---|---|
| `process.jet_fire` | radiant flux at a distance, and the distance to a target flux |
| `process.pool_fire` | pool-fire flame height, surface emissive power and the distance to a threshold |
| `process.bleve` | fireball size and duration, and the distance to a threshold |
| `process.vce` | the TNO multi-energy overpressure and the distance to a target overpressure |
| `process.pfp_demand` | the time a steel member survives unprotected, and the protection a survival time demands |

### Dispersion

| model | what it answers |
|---|---|
| `process.gaussian_plume` | ground-level concentration at a downwind point, and the distance to a target concentration |
| `process.hazardous_area` | the notional source diameter, the mass-fraction LFL, the de-rated distance and the zone |
| `process.heavy_gas_dispersion` | the reduced gravity, the Britter parameter and the distance to a concentration ratio |
| `process.probit` | the probit from a dose, or the probability at a stated probit |
| `process.toxic_library` | the IDLH, ERPG-2 and AEGL-2 thresholds, and a ppm concentration converted to kg/m³ |

### Firewater

| model | what it answers |
|---|---|
| `process.active_fire_protection_screening` | whether water extinguishes, the blowdown margin and the selective-protection saving |
| `process.deluge_nozzle_layout` | the nozzle count from the flow and coverage criteria, the spacing and the delivered density |
| `process.fire_monitor_coverage` | the monitor flow, swept area, wind drift and effective density |
| `process.fire_water_coverage_assessment` | the coverage gap, flow deficit and the supply verdict |

### Overpressure and relief

| model | what it answers |
|---|---|
| `process.fire_case_relief` | the fire-case relief rate from wetted area or geometry, the heat input and the latent heat |
| `process.overpressure_acceptance` | the accumulation factor, the allowable pressure and whether the peak is accepted |
| `process.pipeline_pressure_protection` | the incidental pressure and whether the segment is fully rated or needs two barriers |
| `process.psv_sizing` | the gas, liquid or two-phase relief area and the selected orifice |
| `process.psv_fire_sizing` | the fire-case relief area, the selected orifice and the blowdown pressure |

### Risk and SIL

| model | what it answers |
|---|---|
| `process.nog070_sil_determination` | the achieved SIL against the NOG 070 minimum, and the PFD reduction a non-compliant SIF needs |
| `process.safety_instrumented_function` | the SIF's RRF, required PFD and SIL, architecture PFDs and spurious-trip availability |

Each form states what the model is for, what every input means, the checked limits and the
conditions the model does not check. The **Results** are the kernel's; a scenario's
**distance-to-threshold** table, where the model derives one, is computed by the same kernel —
one inverse solve per threshold — so the table cannot disagree with the number beside it. A
scenario whose inputs changed says its answer is out of date.

### The models that stay listed

A scenario stores scalar inputs and draws scalar outputs, so a table-shaped model is listed
with the refusal its shape earns rather than given a field it cannot express.
`process.gas_dispersion` and the eleven release models take a composition;
`process.fire_water_demand`, `process.relief_disposal_network`, `process.tr3001_compliance`,
`process.consequence_analysis`, `process.risk_matrix`, `process.fault_tree`, `process.lopa` and
`process.event_tree` take and/or return record lists; `process.sil_verification` returns one;
`process.api537_flare_flame_model` takes a flux table; the four self-heating ids publish the
class's warning strings as a vector output, and the basket test takes a record list. The four
rupture models take records too, and their manual pages state the inputs, the range and the
worked example. The four HAZID worksheets are record-shaped as well: the template returns its
deviation rows, the FMEA returns the ranked rows, the consequence population returns the
populated rows, and the MAH skeleton returns its three catalogues. The three safety-review
worksheets are record-shaped as well: the API 14C SAFE chart returns its rows and missing
devices, the barrier register returns its findings, and the Clause 10 review returns its
assessments, verdicts and coverage. The seven record-shaped depressurization models take a
fluid, a stored trajectory or the long-form source samples; `process.psv_valve` is the
family's one scalar scenario, and it runs on the Depressurization deck.

## Units

The document stores SI magnitudes, as the calculation book does. A field shows the unit set at
the top of the page.

- **Frequency** is 1/year. The unit vocabulary has no year, so the value crosses unconverted;
  the SIF's failure rate is 1/hour and its proof-test interval is hours.
- **Heat of combustion and latent heat** are J/kg, declared dimensionless for the same reason:
  no vocabulary unit expresses a mass-specific energy.
- **Probability of failure on demand** is dimensionless in `[0, 1]`; a relief area in in² and a
  SIL are dimensionless too.
- **Flux** is W/m², **pressure** is Pa or bar as the field states, and **severity** is a whole
  number the bow-tie model reads.
- **Flows** are L/min and **relief rates** kg/h where the model's spec states them, so a
  demand or a density reads the way the standard writes it.

## Saving and the served door

Every envelope carries the study's TOML, so **Save writes a file and is no call**. On the
shipped demo the study round-trips through the document; on `azoth serve` the study is the one
the process was started with (`--safety-study`), and New and Open are disabled because the
route has no call that replaces it.

## What is not here yet

The record-shaped models in the ten decks stay listed and refused; rupture, release and QRA
have no native view, and their manual pages state the inputs, the range and the worked
example. The HAZOP worksheet, a printable safety report and a flux-versus-distance chart are
not built. A scenario runs one model, not a bundle of models for one release.
