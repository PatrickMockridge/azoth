/**
 * The fire sub-interface.
 *
 * **One scenario is one registered fire model and the inputs it was given.** A scenario card
 * draws the model's own form, runs it, and shows the answer with a distance-to-threshold table
 * the kernel derived - the browser converts a unit and computes no physics.
 */

import { useState } from "react";

import { DraftField } from "../DraftField";
import { ParameterField } from "../ParameterField";
import { DockTabs } from "../property/DockTabs";
import { WindowChrome } from "../shell/WindowChrome";
import { formatQuantity } from "../../wire/field";
import { outputRowsOf } from "../../state/results";
import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type {
  CalcForm,
  Catalogue,
  SafetyCalculation,
  SafetyEnvelope,
  SafetyScenarioView,
} from "../../wire/types";

export function FireView({
  envelope,
  catalogue,
  units,
  active,
  onActive,
  onAdd,
  onRemove,
  onCommand,
  onCalculate,
}: {
  envelope: SafetyEnvelope;
  catalogue: Catalogue | null;
  units: Units;
  active: string | null;
  onActive: (id: string | null) => void;
  onAdd: (entry: SafetyCalculation) => void;
  onRemove: (id: string) => void;
  onCommand: (command: SafetyCommand) => void;
  onCalculate: (target: SafetyTarget | null) => void;
}) {
  const [query, setQuery] = useState("");
  const scenarios = envelope.study.scenarios;
  const current = scenarios.find((scenario) => scenario.id === active) ?? scenarios[0];
  const form = catalogue?.safety?.find(
    (entry) => entry.id === current?.calculation,
  )?.form;
  const fireEntries = (catalogue?.safety ?? []).filter(
    (entry) => entry.sub_interface === "fire",
  );
  const needle = query.trim().toLowerCase();
  const matches = fireEntries.filter(
    (entry) =>
      needle === "" ||
      entry.name.toLowerCase().includes(needle) ||
      entry.id.toLowerCase().includes(needle) ||
      entry.purpose.toLowerCase().includes(needle),
  );

  return (
    <div className="safety-fire">
      <aside className="safety-side" data-window="picker">
        <WindowChrome title="Fire models">
          <input
            className="calc-search"
            value={query}
            aria-label="Search fire models"
            placeholder="Search fire models"
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="calc-list">
            {matches.map((entry) => (
              <div className="calc-entry" key={entry.id}>
                <button
                  type="button"
                  disabled={!entry.native}
                  title={
                    entry.native
                      ? `add ${entry.name} as a scenario`
                      : (entry.refusal ?? "not runnable in this workspace yet")
                  }
                  onClick={() => onAdd(entry)}
                >
                  {entry.name}
                </button>
                <p className="calc-entry-summary">{entry.purpose}</p>
                {entry.native ? null : (
                  <p className="refusal">
                    {entry.refusal ?? "not runnable in this workspace yet"}
                  </p>
                )}
              </div>
            ))}
            {matches.length === 0 ? <p className="note">nothing matches “{query}”</p> : null}
          </div>
        </WindowChrome>
      </aside>
      <section className="safety-main" data-window="worksheet">
        <WindowChrome title={current?.name ?? "Fire scenarios"}>
          {current === undefined ? (
            <div className="calc-starter">
              <h2>Fire scenarios</h2>
              <p>
                Pick a fire model on the left to add a scenario. Each scenario holds one model's
                inputs and its last answer; the distance-to-threshold table beside the answer is
                derived by the same model.
              </p>
              <ol>
                <li>Add a jet fire, a pool fire, a BLEVE, a vapour-cloud explosion or a protection demand.</li>
                <li>Fill the inputs in the unit set at the top; the model's own bounds are shown beside each field.</li>
                <li>Press Calculate. The table shows how far each threat threshold reaches.</li>
              </ol>
            </div>
          ) : (
            <>
              <DockTabs
                tabs={scenarios.map((scenario) => ({
                  id: scenario.id,
                  label: scenario.name,
                }))}
                active={current.id}
                onTab={onActive}
                label="Safety scenarios"
              />
              <div
                className="safety-scenario-panel"
                role="tabpanel"
                id={`scenario-${current.id}`}
                aria-labelledby={`tab-${current.id}`}
              >
                <ScenarioCard
                  scenario={current}
                  form={form}
                  units={units}
                  onCommand={onCommand}
                  onCalculate={() => onCalculate({ scenario: current.id })}
                  onRemove={() => onRemove(current.id)}
                  onName={(name) =>
                    onCommand({ command: "rename_scenario", id: current.id, name })
                  }
                />
              </div>
            </>
          )}
        </WindowChrome>
      </section>
    </div>
  );
}

function ScenarioCard({
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
