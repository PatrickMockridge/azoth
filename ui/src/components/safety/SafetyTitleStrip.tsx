/**
 * The safety study's title strip.
 *
 * The identity/tools split is the other workspaces' strip, so the fourth workspace wears the same
 * chrome. Its state is a `.chip` rather than a `.pill`, for the reason the calculation book's is:
 * the canvas test reads the first and last `.pill` and this surface has neither.
 */

import { AppearanceMenu } from "../AppearanceMenu";
import { DraftField } from "../DraftField";
import { MenuBar } from "../MenuBar";
import { manualHome } from "../../manual";
import type { MenuSpec } from "../../commands/model";
import type { Appearance } from "../../state/appearance";
import type { Units } from "../../state/units";
import type { SafetyEnvelope } from "../../wire/types";

export function SafetyTitleStrip({
  envelope,
  menus,
  appearance,
  units,
  unsaved,
  version,
  onName,
  onAppearance,
  onUnits,
}: {
  envelope: SafetyEnvelope | null;
  menus: readonly MenuSpec[];
  appearance: Appearance;
  units: Units;
  unsaved: boolean;
  /** The library version, from the flowsheet's provenance record. */
  version: string;
  onName: (name: string) => void;
  onAppearance: (appearance: Appearance) => void;
  onUnits: (id: string) => void;
}) {
  const build = (import.meta.env.VITE_AZOTH_BUILD ?? "").trim();
  const state = status(envelope);
  return (
    <header className="bar">
      <div className="bar-identity">
        <h1>azoth</h1>
        <a
          className="screening"
          href={`${manualHome()}index.html#not-for-design-work-yet`}
          target="_blank"
          rel="noreferrer"
          title="the port is incomplete, so these are screening values and not design deliverables"
        >
          not for design work yet
        </a>
        <MenuBar menus={menus} />
        {envelope === null ? null : (
          <div className="case-name">
            <DraftField
              value={envelope.study.name}
              kind="text"
              label="Safety study name"
              onCommit={(value) => {
                if (typeof value === "string" && value.trim() !== "") {
                  onName(value.trim());
                }
              }}
            />
            {unsaved ? (
              <span className="unsaved" title="the study has edits that are not saved">
                unsaved
              </span>
            ) : null}
          </div>
        )}
        <span className="spacer" />
        {envelope === null ? null : (
          <>
            <span className="chip">{envelope.study.id}</span>
            <span role="status" aria-live="polite" className={`chip ${state.className}`}>
              {state.word}
            </span>
          </>
        )}
      </div>
      <div className="bar-tools">
        <span className="spacer" />
        {units.sets.length === 0 ? null : (
          <label className="units-choice">
            <span className="units-label">Units</span>
            <select
              className="units"
              value={units.active}
              aria-label="Unit set"
              title="the unit a value is read in; the study stores SI and the field shows the set"
              onChange={(event) => onUnits(event.target.value)}
            >
              {units.sets.map((set) => (
                <option key={set.id} value={set.id}>
                  {set.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <span className="build" title="the library version, and the commit this page was built from">
          v{version}
          {build === "" ? "" : ` · ${build.slice(0, 7)}`}
        </span>
        <AppearanceMenu appearance={appearance} onChange={onAppearance} />
      </div>
    </header>
  );
}

/** The study's state, in the chip vocabulary. */
function status(envelope: SafetyEnvelope | null): {
  word: string;
  className: "ok" | "stale" | "warn" | "bad";
} {
  if (envelope === null) {
    return { word: "loading", className: "warn" };
  }
  if (!envelope.ok) {
    return { word: "incomplete", className: "warn" };
  }
  const failed =
    envelope.study.scenarios.some((scenario) => scenario.error !== null) ||
    envelope.study.bowties.some((bowtie) => bowtie.error !== null);
  if (failed) {
    return { word: "failed", className: "bad" };
  }
  if (envelope.dirty) {
    return { word: "stale", className: "stale" };
  }
  return { word: "calculated", className: "ok" };
}
