# Safety function coverage screening

Educational process safety-function coverage screening using the public API RP 14C / ISO 10418 SAFE-chart concept. USE WHEN: a task needs a public, screening-level check of whether a process component has the typically required protective functions (PSH, PSL, PSV, LSH, LSL, etc.) before a formal SAFE-chart and safety analysis review.

This is a `screening` skill and **ported**. azoth's `process.api14c_safe_chart` classifies
each equipment row by its class name, looks up the category's required device set, and reports
the declared-present and missing devices. The class's flowsheet walk, its markdown and JSON
exports and its gap sentences are not carried: the caller states the equipment rows.

## When to Use

Educational process safety-function coverage screening using the public API RP 14C / ISO 10418 SAFE-chart concept. USE WHEN: a task needs a public, screening-level check of whether a process component has the typically required protective functions (PSH, PSL, PSV, LSH, LSL, etc.) before a formal SAFE-chart and safety analysis review.

## Inputs

- `equipment`: rows of `{equipment, unit_class}`. `unit_class` is the unit's simple class name
  or category, matched by lower-case substring.
- `present_devices`: optional rows of `{equipment, device}`, one per declared device. Device
  values are `psh`, `psl`, `lsh`, `lsl`, `tsh`, `tsl`, `usv`, `sdv`, `bdv`, `psv`, `fsv`,
  `fire` and `gas`.
- `assume_complete`: optional boolean. True sets every required device present and ignores the
  declarations.

## Outputs

- `items`: one row per classified equipment with `equipment`, `category`, `required_count`,
  `present_count`, `missing_count` and `complete`.
- `missing_devices`: one row per missing device with `equipment` and `device`.
- `equipment_count`, `missing_count` and `complete` for the whole chart.

## How a calculation runs

The category is a lower-case substring test on `unit_class`, first match wins: scrubber,
separator, flashdrum, knockout, or both flash and drum; tank; heater, furnace or reformer;
pipe, flowline or riser; compressor or expander; pump; heatexchanger, cooler, condenser,
reboiler or aircooler; well; else the row is skipped silently. `&&` binds before `||`, so
`Flash` alone and `Drum` alone do not classify.

The category's required set is the API RP 14C table's: pressure vessel `PSH PSL LSH LSL PSV
SDV`; atmospheric vessel `LSH PSV FSV`; fired vessel `PSH TSH LSL PSV SDV BDV FIRE`; pipeline
segment `PSH PSL SDV FSV`; compressor `PSH PSL TSH SDV BDV PSV`; pump `PSH SDV PSV FSV`; heat
exchanger `PSH PSV TSH`; wellhead `PSH PSL USV SDV PSV`. The class stores each set in a
`HashSet`, which fixes no order; the rows are published in the device declaration order.

An undeclared equipment has an empty present set, so every required device is missing. Extra
declared devices count in `present_count` and are not required rows.

## Python usage pattern

```python
import azoth

result = azoth.process.api14c_safe_chart(
    equipment=[{"equipment": "V-101", "unit_class": "Separator"}],
    present_devices=[
        {"equipment": "V-101", "device": "psh"},
        {"equipment": "V-101", "device": "psv"},
    ],
)
print(result.missing_count)  # 4
print([row.device for row in result.missing_devices])  # ['psl', 'lsh', 'lsl', 'sdv']
```

## The keycard

Not applicable — the chart reads class names and device declarations, not a fluid.

## Validation checklist

- [ ] Confirm the task actually needs this screening rather than a full SAFE-chart review.
- [ ] State the unit class the classifier reads, not a free tag: `Separator` classifies, a
      made-up class name may not.
- [ ] Declare every installed device, including the ones the table does not require.
- [ ] Treat the missing set as a documentation gap to resolve, not as a design finding.

## Common mistakes

| Symptom | Cause | Fix |
|---|---|---|
| An equipment row is absent from the chart | Its class name matched no classifier branch, so the row was skipped | Use the unit's simple class name or a category the classifier reads |
| Every device is missing on a unit that has them | No `present_devices` row declares them | Add one row per installed device |
| `present_count` exceeds `required_count` | Extra declared devices are counted | Expected: the gap is required minus present |
| The device order differs from the table's prose | The class's sets are `HashSet`s | The rows are in the device declaration order |

## Limitations

The chart is the API RP 14C Appendix A required-device table and the class-name classifier;
it is not a safety analysis, a SIL determination or a substitute for the C&E. The flowsheet
walk is caller state: the caller supplies the equipment rows.

## Related Azoth functionality

- `process.s001_clause10_review` — the NORSOK S-001 Clause 10 review over normalized evidence.
- `process.barrier_register_validation` — the barrier register and performance-standard rules.

## References

- API RP 14C. Analysis, Design, Installation, and Testing of Safety Systems for Offshore
  Production Facilities.
- ISO 10418. Offshore production installations — Analysis, design, installation and testing of
  basic surface process safety systems.
- NeqSim: https://github.com/equinor/neqsim
