/**
 * One bow-tie plate, as a form.
 *
 * **A field edit sends the whole bow-tie**, which is the command model's shape: the inspector
 * holds no record of its own, it derives one from the envelope, changes one value and sends it.
 * The summary says what the arithmetic does, because the diagram cannot.
 */

import { DraftField } from "../DraftField";
import type { BowtieRecord, SafetyCommand } from "../../wire/safetyCommands";
import type { SafetyBowtieView } from "../../wire/types";
import { barrierType, nextRowId, recordOf } from "./records";

export function BowtieInspector({
  bowtie,
  selected,
  scenarios,
  onCommand,
  onScenario,
}: {
  bowtie: SafetyBowtieView;
  selected: string | null;
  scenarios: { id: string; name: string }[];
  onCommand: (command: SafetyCommand) => void;
  onScenario: (id: string) => void;
}) {
  const edit = (change: (record: BowtieRecord) => void) => {
    const record = recordOf(bowtie);
    change(record);
    onCommand({ command: "set_bowtie", bowtie: record });
  };

  if (selected === null) {
    return (
      <div className="safety-inspector">
        <h2>{bowtie.name}</h2>
        <p>{bowtie.top_event}</p>
        <Summary bowtie={bowtie} />
        <p className="note">
          Select a plate to edit it. Every functional prevention barrier multiplies the frequency
          once; a mitigation barrier is recorded and read by no formula, because that is what the
          model computes.
        </p>
        <div className="safety-actions">
          <button type="button" onClick={() => add("threat")}>
            Add a threat
          </button>
          <button type="button" onClick={() => add("barrier")}>
            Add a barrier
          </button>
          <button type="button" onClick={() => add("consequence")}>
            Add a consequence
          </button>
        </div>
      </div>
    );
  }

  function add(role: "threat" | "barrier" | "consequence") {
    edit((record) => {
      if (role === "threat") {
        record.threats.push({
          id: nextRowId(
            "threat",
            record.threats.map((row) => row.id),
          ),
          label: "New threat",
          frequency: 0.1,
          active: true,
        });
      } else if (role === "barrier") {
        record.barriers.push({
          id: nextRowId(
            "barrier",
            record.barriers.map((row) => row.id),
          ),
          label: "New barrier",
          pfd: 0.1,
          barrier_type: "prevention",
          functional: true,
        });
      } else {
        record.consequences.push({
          id: nextRowId(
            "consequence",
            record.consequences.map((row) => row.id),
          ),
          label: "New consequence",
          severity: 1,
        });
      }
    });
  }

  if (selected === "top_event") {
    return (
      <div className="safety-inspector">
        <h2>Top event</h2>
        <label className="safety-name">
          <span>Name</span>
          <DraftField
            value={bowtie.name}
            kind="text"
            label="Bow-tie name"
            onCommit={(value) => {
              if (typeof value === "string" && value.trim() !== "") {
                edit((record) => {
                  record.name = value.trim();
                });
              }
            }}
          />
        </label>
        <label className="safety-name">
          <span>Top event</span>
          <DraftField
            value={bowtie.top_event}
            kind="text"
            label="Top event"
            onCommit={(value) => {
              if (typeof value === "string" && value.trim() !== "") {
                edit((record) => {
                  record.top_event = value.trim();
                });
              }
            }}
          />
        </label>
        <Summary bowtie={bowtie} />
      </div>
    );
  }

  if (selected.startsWith("threat:")) {
    const id = selected.slice("threat:".length);
    const threat = bowtie.threats.find((row) => row.id === id);
    if (threat === undefined) {
      return <Missing />;
    }
    return (
      <div className="safety-inspector">
        <h2>Threat</h2>
        <Field
          label="Label"
          value={threat.label}
          onCommit={(value) => {
            if (typeof value === "string") {
              edit((record) => {
                const row = record.threats.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.label = value;
                }
              });
            }
          }}
        />
        <Field
          label="Frequency (1/year)"
          number
          value={threat.frequency}
          onCommit={(value) => {
            const number = Number(value);
            if (Number.isFinite(number)) {
              edit((record) => {
                const row = record.threats.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.frequency = number;
                }
              });
            }
          }}
        />
        <label className="safety-check">
          <input
            type="checkbox"
            checked={threat.active}
            onChange={(event) =>
              edit((record) => {
                const row = record.threats.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.active = event.target.checked;
                }
              })
            }
          />
          <span>Contributes to the unmitigated frequency</span>
        </label>
        <Remove
          onRemove={() =>
            edit((record) => {
              record.threats = record.threats.filter((entry) => entry.id !== id);
            })
          }
        />
      </div>
    );
  }

  if (selected.startsWith("barrier:")) {
    const id = selected.slice("barrier:".length);
    const barrier = bowtie.barriers.find((row) => row.id === id);
    if (barrier === undefined) {
      return <Missing />;
    }
    return (
      <div className="safety-inspector">
        <h2>Barrier</h2>
        <Field
          label="Label"
          value={barrier.label}
          onCommit={(value) => {
            if (typeof value === "string") {
              edit((record) => {
                const row = record.barriers.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.label = value;
                }
              });
            }
          }}
        />
        <label className="safety-name">
          <span>Type</span>
          <select
            value={barrier.barrier_type}
            aria-label="Barrier type"
            onChange={(event) =>
              edit((record) => {
                const row = record.barriers.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.barrier_type = barrierType(event.target.value);
                }
              })
            }
          >
            <option value="prevention">prevention</option>
            <option value="mitigation">mitigation</option>
            <option value="both">both</option>
          </select>
        </label>
        <Field
          label="PFD (0 to 1)"
          number
          value={barrier.pfd}
          onCommit={(value) => {
            const number = Number(value);
            if (Number.isFinite(number)) {
              edit((record) => {
                const row = record.barriers.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.pfd = number;
                }
              });
            }
          }}
        />
        <label className="safety-check">
          <input
            type="checkbox"
            checked={barrier.functional}
            onChange={(event) =>
              edit((record) => {
                const row = record.barriers.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.functional = event.target.checked;
                }
              })
            }
          />
          <span>Functional</span>
        </label>
        {barrier.barrier_type === "mitigation" ? (
          <p className="note">
            A mitigation barrier is recorded and changes no frequency: the model reads only the
            prevention barriers' PFDs.
          </p>
        ) : null}
        <Remove
          onRemove={() =>
            edit((record) => {
              record.barriers = record.barriers.filter((entry) => entry.id !== id);
            })
          }
        />
      </div>
    );
  }

  if (selected.startsWith("consequence:")) {
    const id = selected.slice("consequence:".length);
    const consequence = bowtie.consequences.find((row) => row.id === id);
    if (consequence === undefined) {
      return <Missing />;
    }
    return (
      <div className="safety-inspector">
        <h2>Consequence</h2>
        <Field
          label="Label"
          value={consequence.label}
          onCommit={(value) => {
            if (typeof value === "string") {
              edit((record) => {
                const row = record.consequences.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.label = value;
                }
              });
            }
          }}
        />
        <Field
          label="Severity (whole number)"
          number
          value={consequence.severity}
          onCommit={(value) => {
            const number = Number(value);
            if (Number.isFinite(number)) {
              edit((record) => {
                const row = record.consequences.find((entry) => entry.id === id);
                if (row !== undefined) {
                  row.severity = number;
                }
              });
            }
          }}
        />
        <label className="safety-name">
          <span>Modelled by scenario</span>
          <select
            value={consequence.scenario ?? ""}
            aria-label="Modelled by scenario"
            onChange={(event) => {
              const wanted = event.target.value;
              edit((record) => {
                const row = record.consequences.find((entry) => entry.id === id);
                if (row === undefined) {
                  return;
                }
                if (wanted === "") {
                  delete row.scenario;
                } else {
                  row.scenario = wanted;
                }
              });
            }}
          >
            <option value="">—</option>
            {scenarios.map((scenario) => (
              <option key={scenario.id} value={scenario.id}>
                {scenario.name}
              </option>
            ))}
          </select>
        </label>
        {consequence.scenario === null ? null : (
          <button
            type="button"
            onClick={() => {
              if (consequence.scenario !== null) {
                onScenario(consequence.scenario);
              }
            }}
          >
            Open {consequence.scenario_name ?? consequence.scenario} in Fire
          </button>
        )}
        <Remove
          onRemove={() =>
            edit((record) => {
              record.consequences = record.consequences.filter((entry) => entry.id !== id);
            })
          }
        />
      </div>
    );
  }

  return <Missing />;
}

function Field({
  label,
  value,
  number = false,
  onCommit,
}: {
  label: string;
  value: string | number;
  number?: boolean;
  onCommit: (value: string | number) => void;
}) {
  return (
    <label className="safety-name">
      <span>{label}</span>
      <DraftField
        value={String(value)}
        kind={number ? "number" : "text"}
        label={label}
        onCommit={onCommit}
      />
    </label>
  );
}

function Remove({ onRemove }: { onRemove: () => void }) {
  return (
    <button type="button" className="safety-remove" onClick={onRemove}>
      Remove this plate
    </button>
  );
}

function Missing() {
  return (
    <div className="safety-inspector">
      <p className="note">That plate is no longer in the bow-tie. Select another.</p>
    </div>
  );
}

function Summary({ bowtie }: { bowtie: SafetyBowtieView }) {
  const derived = bowtie.derived;
  return (
    <dl className="safety-summary">
      <dt>Threats</dt>
      <dd>{bowtie.counts.threats}</dd>
      <dt>Prevention barriers</dt>
      <dd>{bowtie.counts.prevention_barriers}</dd>
      <dt>Mitigation barriers</dt>
      <dd>{bowtie.counts.mitigation_barriers}</dd>
      <dt>Consequences</dt>
      <dd>{bowtie.counts.consequences}</dd>
      <dt>Unmitigated</dt>
      <dd>{derived?.unmitigated_frequency_per_year ?? "—"} /year</dd>
      <dt>Mitigated</dt>
      <dd>{derived?.mitigated_frequency_per_year ?? "—"} /year</dd>
      <dt>RRF</dt>
      <dd>{derived?.total_rrf ?? "—"}</dd>
      <dt>Maximum severity</dt>
      <dd>{derived?.max_severity ?? "—"}</dd>
    </dl>
  );
}
