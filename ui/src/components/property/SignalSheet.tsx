/**
 * One signal between instruments: which tag sends it, which receives it, and what carries it.
 *
 * **A signal is a view of control, like the instrument it joins.** It is not a process stream —
 * no material moves along it — so it has no conditions, no composition and no result sheet; the
 * five carriers DEXPI keeps are the whole record.
 *
 * The endpoints are the identities of the edge; this sheet states them and edits the carrier.
 * Changing an endpoint is a different signal, which is a different command, not a field write.
 */

import type { EditorCommand } from "../../wire/commands";
import type { GraphEdge } from "../../wire/types";

/** The five carriers DEXPI names, and the empty option for "the drawing does not say". */
const KINDS = ["electric", "pneumatic", "hydraulic", "capillary", "radiation"] as const;

export function SignalSheet({
  edge,
  onCommand,
}: {
  edge: GraphEdge;
  onCommand: (command: EditorCommand) => void;
}) {
  const { from, to } = edge.data;

  return (
    <div className="sheet" id="sheet-signal" role="tabpanel" aria-labelledby="tab-signal">
      <div className="field">
        <div className="head">
          <span className="name">from</span>
        </div>
        <input value={from} readOnly aria-label="from" />
        <div className="hint">the instrument the signal leaves</div>
      </div>
      <div className="field">
        <div className="head">
          <span className="name">to</span>
        </div>
        <input value={to} readOnly aria-label="to" />
        <div className="hint">the instrument it reaches</div>
      </div>
      <div className="field">
        <div className="head">
          <span className="name">carrier</span>
        </div>
        <select
          value={edge.data.signal_kind ?? ""}
          aria-label="carrier"
          onChange={(event) => {
            const raw = event.target.value;
            const carrier = KINDS.find((entry) => entry === raw);
            onCommand({
              command: "set_signal",
              from,
              to,
              ...(carrier === undefined ? {} : { kind: carrier }),
            });
          }}
        >
          <option value="">not stated</option>
          {KINDS.map((carrier) => (
            <option key={carrier} value={carrier}>
              {carrier}
            </option>
          ))}
        </select>
        <div className="hint">
          the five carriers DEXPI keeps: what the line means, not what it carries
        </div>
      </div>
      <p className="note">
        a signal is control, not process state — nothing is solved along it, and a run does not
        change it
      </p>
    </div>
  );
}
