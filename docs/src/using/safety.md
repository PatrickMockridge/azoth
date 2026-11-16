# Safety studies

The **Safety** switch opens a safety study: one TOML document holding fire scenarios and
bow-ties, beside the flowsheet and the calculation book. It is its own document, with its own
commands and its own envelope; switching workspaces never throws it away.

## The study

A study has an id, a name, a list of **scenarios** and a list of **bow-ties**. Results are
derived and never saved: a scenario's answer is the last calculate's, and a scenario whose
inputs changed says its answer is out of date.

The **Overview** lists the study's bow-ties and scenarios, every diagnostic the study carries,
and the whole safety surface grouped by family. Five fire models and the bow-tie run inside
this workspace. The rest — dispersion, firewater, overpressure, rupture, release, risk and
QRA — are listed with links to their manual pages; they are not runnable here yet.

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

## Fire scenarios

A fire scenario is one registered model and the inputs it was given. Five can be run here:

| model | what it answers |
|---|---|
| `process.jet_fire` | radiant flux at a distance, and the distance to a target flux |
| `process.pool_fire` | pool-fire flame height, surface emissive power and the distance to a threshold |
| `process.bleve` | fireball size and duration, and the distance to a threshold |
| `process.vce` | the TNO multi-energy overpressure and the distance to a target overpressure |
| `process.pfp_demand` | the time a steel member survives unprotected, and the protection a survival time demands |

Each form states what the model is for, what every input means, the checked limits and the
conditions the model does not check. The **Results** are the kernel's, and the
**distance-to-threshold** table is derived by the same kernel — one inverse solve per threshold,
so the table cannot disagree with the number beside it.

The API 537 flare flame model is listed and refused here: its target fluxes are a table of
rows, which a scenario does not yet store.

## Units

The document stores SI magnitudes, as the calculation book does. A field shows the unit set at
the top of the page.

- **Frequency** is 1/year. The unit vocabulary has no year, so the value crosses unconverted.
- **Heat of combustion** is J/kg, declared dimensionless for the same reason: no vocabulary
  unit expresses a mass-specific energy.
- **Flux** is W/m², **pressure** is Pa or bar as the field states, and **severity** is a whole
  number the bow-tie model reads.

## Saving and the served door

Every envelope carries the study's TOML, so **Save writes a file and is no call**. On the
shipped demo the study round-trips through the document; on `azoth serve` the study is the one
the process was started with (`--safety-study`), and New and Open are disabled because the
route has no call that replaces it.

## What is not here yet

The other safety families have no native view; their manual pages state the inputs, range and
worked example. The HAZOP worksheet, a printable safety report and a flux-versus-distance chart
are not built. A scenario runs one model, not a bundle of models for one release.
