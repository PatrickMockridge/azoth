/**
 * One scenario family: the models it can add, the study's scenarios in it, and the selected
 * scenario's card.
 *
 * **The deck filters the study by the scenario's own family**, so the Fire deck draws the fire
 * scenarios and nothing else even when the study holds six families. The models come from the
 * catalogue's `sub_interface`; a model the workspace cannot run stays listed and disabled with
 * the refusal its shape earned.
 */

import { useState } from "react";

import { DockTabs } from "../property/DockTabs";
import { WindowChrome } from "../shell/WindowChrome";
import { ScenarioCard } from "./ScenarioCard";
import type { ScenarioView } from "./views";
import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyEnvelope,
} from "../../wire/types";

const COPY: Record<
  ScenarioView,
  { title: string; heading: string; search: string; blurb: string; step: string }
> = {
  fire: {
    title: "Fire models",
    heading: "Fire scenarios",
    search: "Search fire models",
    blurb:
      "Each scenario holds one fire model's inputs and its last answer; the distance-to-threshold table beside the answer is derived by the same model.",
    step: "Add a jet fire, a pool fire, a BLEVE, a vapour-cloud explosion or a protection demand.",
  },
  dispersion: {
    title: "Dispersion models",
    heading: "Dispersion scenarios",
    search: "Search dispersion models",
    blurb:
      "Each scenario holds one dispersion model's inputs and its last answer: a concentration, a distance, a probit or a threshold lookup.",
    step: "Add a plume, a hazardous-area screen, a dense-gas distance, a probit or a toxic-threshold lookup.",
  },
  equipmentsafety: {
    title: "Equipment safety models",
    heading: "Equipment safety scenarios",
    search: "Search equipment safety models",
    blurb:
      "Each scenario holds one equipment safety screen's inputs and its last answer: a blowby rate, a deadhead pressure, an adiabatic rise or a metal temperature.",
    step: "Add a gas blowby, a pump deadhead, a runaway reaction or a minimum design metal temperature.",
  },
  firewater: {
    title: "Firewater models",
    heading: "Firewater scenarios",
    search: "Search firewater models",
    blurb:
      "Each scenario holds one firewater model's inputs and its last answer: the demand, the layout, the coverage or the supply verdict.",
    step: "Add a protection screen, a deluge layout, a monitor coverage or a supply assessment.",
  },
  overpressure: {
    title: "Relief models",
    heading: "Relief scenarios",
    search: "Search relief models",
    blurb:
      "Each scenario holds one relief or overpressure model's inputs and its last answer: an area, an acceptance, a margin or a verdict.",
    step: "Add a fire-case relief, a relief-valve sizing, an acceptance check or a pipeline protection screen.",
  },
  processsafetysystem: {
    title: "Process safety system models",
    heading: "Process safety system scenarios",
    search: "Search process safety system models",
    blurb:
      "Each scenario holds one process safety system model's inputs and its last answer: a resolved target frequency, the pressure checks and the acceptability verdict.",
    step: "Add a NORSOK S-001 secondary-pressure protection screen.",
  },
  risk: {
    title: "Risk and SIL models",
    heading: "Risk scenarios",
    search: "Search risk models",
    blurb:
      "Each scenario holds one risk or SIL model's inputs and its last answer: a SIL, a PFD arithmetic or a compliance verdict.",
    step: "Add a NOG 070 determination or an instrumented-function assessment.",
  },
  safetyassessment: {
    title: "Safety assessment models",
    heading: "Safety assessment scenarios",
    search: "Search safety assessment models",
    blurb:
      "Each scenario holds one safety assessment's inputs and its last answer: the four Kletz pillar scores and their mean index.",
    step: "Add an inherent-safety evaluation.",
  },
  vibration: {
    title: "Vibration models",
    heading: "Vibration scenarios",
    search: "Search vibration models",
    blurb:
      "Each scenario holds one vibration screen's inputs and its last answer: the likelihood of failure, the sound power level and its margin, and every contributing factor.",
    step: "Add a gas or liquid flow-induced vibration screen or an acoustic-induced vibration screen.",
  },
};

export function ScenarioDeck({
  view,
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
  view: ScenarioView;
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
  const copy = COPY[view];
  const scenarios = envelope.study.scenarios.filter((scenario) => scenario.family === view);
  const current = scenarios.find((scenario) => scenario.id === active) ?? scenarios[0];
  const form = catalogue?.safety?.find(
    (entry) => entry.id === current?.calculation,
  )?.form;
  const models = (catalogue?.safety ?? []).filter(
    (entry) => entry.sub_interface === view,
  );
  const needle = query.trim().toLowerCase();
  const matches = models.filter(
    (entry) =>
      needle === "" ||
      entry.name.toLowerCase().includes(needle) ||
      entry.id.toLowerCase().includes(needle) ||
      entry.purpose.toLowerCase().includes(needle),
  );

  return (
    <div className="safety-deck">
      <aside className="safety-side" data-window="picker">
        <WindowChrome title={copy.title}>
          <input
            className="calc-search"
            value={query}
            aria-label={copy.search}
            placeholder={copy.search}
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
        <WindowChrome title={current?.name ?? copy.heading}>
          {current === undefined ? (
            <div className="calc-starter">
              <h2>{copy.heading}</h2>
              <p>{copy.blurb}</p>
              <ol>
                <li>{copy.step}</li>
                <li>Fill the inputs in the unit set at the top; the model's own bounds are shown beside each field.</li>
                <li>Press Calculate. Every output is the model's; a distance-to-threshold table appears where the model derives one.</li>
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
                label={`Safety ${copy.heading.toLowerCase()}`}
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
