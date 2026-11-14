/**
 * The control record behind an instrument: what the tag is, what it measures, and what it does.
 *
 * **This layer is exchange evidence, not physics.** `PSHH-101` is a view of control — the run
 * never reads it — but a P&ID carries it, and a round trip that loses a range or a SIL loses the
 * one thing the drawing was for. The sheet reads the node's record and sends the whole record back
 * as `set_instrument`, so the command model needs no per-field setters and a cleared field is an
 * unset rather than a value that could not be cleared.
 *
 * **The tag is not editable here.** Renaming would move every signal that names it, which is a
 * document edit and not a field edit; the tag is shown and left alone.
 *
 * **A range and a safety function are records, not fields**, so they are added and cleared whole.
 * A per-field write would have to invent the part the reader has not typed yet — a range with no
 * unit, a SIL with no function — and the first keystroke would land something the drawing never
 * said.
 */

import { DraftField } from "../DraftField";
import type { EditorCommand } from "../../wire/commands";
import type { GraphNode, Instrument, InstrumentRange, SafetyFunction } from "../../wire/types";

/** The three locations DEXPI names, and the empty option for "the drawing does not say". */
const LOCATIONS = ["field", "panel", "central"] as const;

/** A patch where a present-but-undefined key clears the field. */
type Patch<T> = { [K in keyof T]?: T[K] | undefined };

/** The patch's value where it names the key, and the record's otherwise — a key `in` the patch. */
function pick<T, K extends keyof T>(base: T, patch: Patch<T>, key: K): T[K] | undefined {
  return key in patch ? patch[key] : base[key];
}

/** The free text fields, in the order the record declares them. */
const TEXT: readonly {
  field: "function" | "service" | "loop_number" | "measures" | "actuates";
  hint: string;
  patch: (value: string | undefined) => Patch<Instrument>;
}[] = [
  {
    field: "function",
    hint: "ISA function letters — PSHH, TIC, FV, LS — as the tag states them",
    patch: (value) => ({ function: value ?? "" }),
  },
  {
    field: "service",
    hint: "what it measures or commands, in words",
    patch: (value) => ({ service: value }),
  },
  {
    field: "loop_number",
    hint: "the loop the tag's number belongs to",
    patch: (value) => ({ loop_number: value }),
  },
  {
    field: "measures",
    hint: "a stream endpoint or an instance id",
    patch: (value) => ({ measures: value }),
  },
  {
    field: "actuates",
    hint: "an instance id or an endpoint",
    patch: (value) => ({ actuates: value }),
  },
];

export function InstrumentSheet({
  node,
  onCommand,
}: {
  node: GraphNode;
  onCommand: (command: EditorCommand) => void;
}) {
  const instrument = node.data.instrument;

  if (instrument === undefined) {
    return (
      <div className="sheet" id="sheet-control" role="tabpanel" aria-labelledby="tab-control">
        <p className="note">this node carries no control record</p>
      </div>
    );
  }

  /** The whole record with a patch applied; a patched-away optional is an unset. */
  const send = (patch: Patch<Instrument>) => {
    const service = pick(instrument, patch, "service");
    const loop_number = pick(instrument, patch, "loop_number");
    const measures = pick(instrument, patch, "measures");
    const actuates = pick(instrument, patch, "actuates");
    const location = pick(instrument, patch, "location");
    const range = pick(instrument, patch, "range");
    const safety = pick(instrument, patch, "safety");
    onCommand({
      command: "set_instrument",
      id: instrument.id,
      function: pick(instrument, patch, "function") ?? instrument.function,
      ...(service === undefined ? {} : { service }),
      ...(loop_number === undefined ? {} : { loop_number }),
      ...(measures === undefined ? {} : { measures }),
      ...(actuates === undefined ? {} : { actuates }),
      ...(location === undefined ? {} : { location }),
      ...(range === undefined ? {} : { range }),
      ...(safety === undefined ? {} : { safety }),
    });
  };

  const range = instrument.range;
  const writeRange = (patch: Patch<InstrumentRange>) => {
    if (range === undefined) {
      return;
    }
    send({
      range: {
        min: pick(range, patch, "min") ?? range.min,
        max: pick(range, patch, "max") ?? range.max,
        unit: pick(range, patch, "unit") ?? range.unit,
      },
    });
  };

  const safety = instrument.safety;
  const writeSafety = (patch: Patch<SafetyFunction>) => {
    if (safety === undefined) {
      return;
    }
    const voting = pick(safety, patch, "voting");
    const safe_state = pick(safety, patch, "safe_state");
    const proof_test_hours = pick(safety, patch, "proof_test_hours");
    const closure_seconds = pick(safety, patch, "closure_seconds");
    send({
      safety: {
        function: pick(safety, patch, "function") ?? safety.function,
        sil: pick(safety, patch, "sil") ?? safety.sil,
        ...(voting === undefined ? {} : { voting }),
        ...(safe_state === undefined ? {} : { safe_state }),
        ...(proof_test_hours === undefined ? {} : { proof_test_hours }),
        ...(closure_seconds === undefined ? {} : { closure_seconds }),
      },
    });
  };

  return (
    <div className="sheet" id="sheet-control" role="tabpanel" aria-labelledby="tab-control">
      <div className="field">
        <div className="head">
          <span className="name">tag</span>
        </div>
        <input value={instrument.id} readOnly aria-label="tag" />
        <div className="hint">
          renaming a tag is a document edit — it moves every signal that names it — so it is not
          offered here
        </div>
      </div>

      {TEXT.map(({ field, hint, patch }) => (
        <div className="field" key={field}>
          <div className="head">
            <span className="name">{field}</span>
          </div>
          <DraftField
            value={readField(instrument, field) ?? ""}
            kind="text"
            placeholder={field === "function" ? "required" : "not stated"}
            label={field}
            onCommit={(next) => {
              const value = String(next);
              if (field === "function" && value.trim() === "") {
                // The one required field: an emptied function is not a command, and the
                // placeholder says so rather than sending an empty ISA tag.
                return;
              }
              send(patch(value === "" ? undefined : value));
            }}
          />
          <div className="hint">{hint}</div>
        </div>
      ))}

      <div className="field">
        <div className="head">
          <span className="name">location</span>
        </div>
        <select
          value={instrument.location ?? ""}
          aria-label="location"
          onChange={(event) => {
            const raw = event.target.value;
            const location = LOCATIONS.find((entry) => entry === raw);
            send({ location });
          }}
        >
          <option value="">not stated</option>
          {LOCATIONS.map((location) => (
            <option key={location} value={location}>
              {location}
            </option>
          ))}
        </select>
        <div className="hint">where the instrument sits, in the three names DEXPI keeps</div>
      </div>

      <h2>Calibrated range</h2>
      {range === undefined ? (
        <div className="field">
          <button
            type="button"
            className="button"
            onClick={() => send({ range: { min: 0, max: 0, unit: "" } })}
            title="state the range the drawing carries, then fill its three fields"
          >
            State a range
          </button>
          <div className="hint">the drawing does not state one</div>
        </div>
      ) : (
        <>
          <div className="field">
            <div className="head">
              <span className="name">min</span>
            </div>
            <DraftField
              value={String(range.min)}
              kind="number"
              label="range min"
              onCommit={(next) => {
                if (typeof next === "number") {
                  writeRange({ min: next });
                }
              }}
            />
          </div>
          <div className="field">
            <div className="head">
              <span className="name">max</span>
            </div>
            <DraftField
              value={String(range.max)}
              kind="number"
              label="range max"
              onCommit={(next) => {
                if (typeof next === "number") {
                  writeRange({ max: next });
                }
              }}
            />
          </div>
          <div className="field">
            <div className="head">
              <span className="name">unit</span>
            </div>
            <DraftField
              value={range.unit}
              kind="text"
              label="range unit"
              onCommit={(next) => {
                writeRange({ unit: String(next) });
              }}
            />
          </div>
          <div className="field">
            <button
              type="button"
              className="button"
              onClick={() => send({ range: undefined })}
              title="the drawing states no range"
            >
              Clear range
            </button>
          </div>
        </>
      )}

      <h2>Safety function</h2>
      {safety === undefined ? (
        <div className="field">
          <button
            type="button"
            className="button"
            onClick={() => send({ safety: { function: "", sil: 1 } })}
            title="state the safety function the drawing names, then fill its fields"
          >
            State a safety function
          </button>
          <div className="hint">
            exchange evidence, not SIL verification — what the drawing says, kept so a round trip
            does not lose it
          </div>
        </div>
      ) : (
        <>
          <div className="field">
            <div className="head">
              <span className="name">SIF tag</span>
            </div>
            <DraftField
              value={safety.function}
              kind="text"
              placeholder="the safety function's own tag"
              label="safety function"
              onCommit={(next) => {
                writeSafety({ function: String(next) });
              }}
            />
          </div>
          <div className="field">
            <div className="head">
              <span className="name">SIL</span>
            </div>
            <DraftField
              value={String(safety.sil)}
              kind="number"
              label="safety integrity level"
              onCommit={(next) => {
                if (typeof next === "number") {
                  writeSafety({ sil: next });
                }
              }}
            />
            <div className="hint">the declared integrity level, 1 to 4</div>
          </div>
          <div className="field">
            <div className="head">
              <span className="name">voting</span>
            </div>
            <DraftField
              value={safety.voting ?? ""}
              kind="text"
              placeholder="not stated"
              label="voting"
              onCommit={(next) => {
                writeSafety({ voting: next === "" ? undefined : String(next) });
              }}
            />
            <div className="hint">the voting architecture, e.g. 2oo3</div>
          </div>
          <div className="field">
            <div className="head">
              <span className="name">safe_state</span>
            </div>
            <DraftField
              value={safety.safe_state ?? ""}
              kind="text"
              placeholder="not stated"
              label="safe state"
              onCommit={(next) => {
                writeSafety({ safe_state: next === "" ? undefined : String(next) });
              }}
            />
            <div className="hint">the state the final element moves to: open, closed or trip</div>
          </div>
          <div className="field">
            <div className="head">
              <span className="name">proof_test_hours</span>
            </div>
            <DraftField
              value={safety.proof_test_hours === undefined ? "" : String(safety.proof_test_hours)}
              kind="number"
              placeholder="not stated"
              label="proof test interval"
              onCommit={(next) => {
                writeSafety({ proof_test_hours: typeof next === "number" ? next : undefined });
              }}
            />
            <div className="hint">the proof-test interval, hours</div>
          </div>
          <div className="field">
            <div className="head">
              <span className="name">closure_seconds</span>
            </div>
            <DraftField
              value={safety.closure_seconds === undefined ? "" : String(safety.closure_seconds)}
              kind="number"
              placeholder="not stated"
              label="closure time"
              onCommit={(next) => {
                writeSafety({ closure_seconds: typeof next === "number" ? next : undefined });
              }}
            />
            <div className="hint">the final element&apos;s closure time, seconds</div>
          </div>
          <div className="field">
            <button
              type="button"
              className="button"
              onClick={() => send({ safety: undefined })}
              title="the drawing names no safety function for this instrument"
            >
              Clear safety function
            </button>
          </div>
        </>
      )}
    </div>
  );
}

/** The text field's stored value, by name, without a computed-key write. */
function readField(
  instrument: Instrument,
  field: (typeof TEXT)[number]["field"],
): string | undefined {
  switch (field) {
    case "function":
      return instrument.function;
    case "service":
      return instrument.service;
    case "loop_number":
      return instrument.loop_number;
    case "measures":
      return instrument.measures;
    case "actuates":
      return instrument.actuates;
  }
}
