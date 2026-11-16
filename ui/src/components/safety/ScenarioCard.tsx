/**
 * One scenario: a registered model, its inputs and its last answer.
 *
 * **Generic over the form.** The card draws whatever `CalcForm` the catalogue holds for the
 * scenario's calculation, so a new family needs a form and a deck entry, not a new card. The
 * browser converts a unit and computes no physics; the distance-to-threshold table is derived
 * by the same model and is drawn only where the model produced one.
 */

import { DraftField } from "../DraftField";
import { ParameterField } from "../ParameterField";
import { formatQuantity } from "../../wire/field";
import { outputRowsOf } from "../../state/results";
import type { Units } from "../../state/units";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type { CalcForm, SafetyScenarioView } from "../../wire/types";

export function ScenarioCard({
  scenario,
  form,
  units,
  onCommand,
  onCalculate,
  onRemove,
  onName,
}: {
  scenario: SafetyScenarioView;
  form: CalcForm | undefined;
  units: Units;
  onCommand: (command: SafetyCommand) => void;
  onCalculate: () => void;
  onRemove: () => void;
  onName: (name: string) => void;
}) {
  const staleResult = scenario.stale && scenario.result !== null;
  return (
    <article className="safety-scenario">
      <header className="safety-scenario-head">
        <div>
          <h2>{scenario.name}</h2>
          <p className="calc-entry-summary">{scenario.purpose ?? scenario.calculation}</p>
        </div>
        <div className="safety-actions">
          <button
            type="button"
            className="primary"
            onClick={onCalculate}
            disabled={!scenario.ready}
            title={scenario.ready ? "run this scenario" : "fill the required inputs first"}
          >
            Calculate
          </button>
          <button type="button" onClick={onRemove} title="remove this scenario from the study">
            Remove
          </button>
        </div>
      </header>

      <label className="safety-name">
        <span>Name</span>
        <DraftField
          value={scenario.name}
          kind="text"
          label="Scenario name"
          onCommit={(value) => {
            if (typeof value === "string" && value.trim() !== "") {
              onName(value.trim());
            }
          }}
        />
      </label>

      {form === undefined ? null : (
        <section className="calc-section">
          <h3>Inputs</h3>
          <div className="calc-inputs">
            {form.inputs.map((input) => (
              <ParameterField
                key={input.name}
                layout="row"
                parameter={input}
                value={scenario.inputs[input.name] ?? ""}
                units={units}
                onChange={(value) =>
                  onCommand({
                    command: "set_scenario_input",
                    scenario: scenario.id,
                    input: input.name,
                    value: value as number | boolean | string,
                  })
                }
                action={
                  input.required ? null : (
                    <button
                      type="button"
                      className="clear"
                      aria-label={`Clear ${input.name}`}
                      title="leave this input unset; the model uses its declared default or omits what needs it"
                      onClick={() =>
                        onCommand({
                          command: "unset_scenario_input",
                          scenario: scenario.id,
                          input: input.name,
                        })
                      }
                    >
                      clear
                    </button>
                  )
                }
              />
            ))}
          </div>
        </section>
      )}

      {scenario.diagnostics.length === 0 ? null : (
        <div className="calc-diagnostics">
          {scenario.diagnostics.map((diagnostic) => (
            <p key={`${diagnostic.code}:${diagnostic.field ?? ""}`}>{diagnostic.message}</p>
          ))}
        </div>
      )}

      {scenario.error === null ? null : (
        <div className="fault">
          {scenario.error.message}
          {scenario.error.field === null ? null : (
            <span className="where"> · {scenario.error.field}</span>
          )}
        </div>
      )}

      {staleResult ? (
        <p className="calc-stale" role="status">
          These are the last calculated values; the inputs changed after the last Calculate.
        </p>
      ) : null}

      {scenario.result === null || form === undefined ? (
        <p className="note">
          {scenario.error === null ? "not calculated yet; press Calculate" : "not calculated"}
        </p>
      ) : (
        <section className="calc-section">
          <h3>Results</h3>
          <div className="calc-outputs">
            {outputRowsOf(scenario.result.outputs, units, form.outputs).map((row) => (
              <div className="calc-output" key={row.key}>
                <div className="calc-output-text">
                  <div className="head">
                    <span className="name">{row.label}</span>
                  </div>
                  {row.description === null || row.description === "" ? null : (
                    <p className="description">{row.description}</p>
                  )}
                </div>
                <div className="calc-output-value">
                  <span className="value">{row.text}</span>
                  <span className="unit">{row.unit ?? ""}</span>
                </div>
              </div>
            ))}
          </div>
          {scenario.result.warnings.length === 0 ? null : (
            <div className="calc-warnings">
              <h4>Warnings</h4>
              {scenario.result.warnings.map((warning, index) => (
                <p key={`${warning.code}:${index}`}>
                  <strong>{warning.code}</strong> {warning.message}
                </p>
              ))}
            </div>
          )}
        </section>
      )}

      {scenario.profile.length === 0 ? null : (
        <section className="calc-section">
          <h3>Distance to threshold</h3>
          <p className="calc-section-note">
            Derived by this scenario's own model, one inverse solve per threshold. A threshold the
            model cannot reach at any distance is shown as not reached.
          </p>
          <table className="safety-profile">
            <thead>
              <tr>
                <th scope="col">Threshold</th>
                <th scope="col">Distance</th>
              </tr>
            </thead>
            <tbody>
              {scenario.profile.map((row) => (
                <tr key={`${row.target.magnitude_si}:${row.target.unit}`}>
                  <td>{formatQuantity(row.target.magnitude_si, row.target.unit, 3, units)}</td>
                  <td>
                    {row.distance === null
                      ? "not reached"
                      : formatQuantity(row.distance.magnitude_si, row.distance.unit, 3, units)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </article>
  );
}
