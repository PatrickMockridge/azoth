/**
 * One scenario family: the models it can add, the study's scenarios in it, and the selected
 * scenario's card.
 *
 * **The deck filters the study by the scenario's own family**, so the Fire deck draws the fire
 * scenarios and nothing else even when the study holds six families. The models come from the
 * catalogue's `sub_interface`; a model the workspace cannot run stays listed and disabled with
 * the refusal its shape earned.
 *
 * **The copy is the family registry's.** The tab, the picker title, the starter heading and the
 * first step are read from the served row, so a deck and its overview card cannot disagree.
 */

import { useState } from "react";

import { DockTabs } from "../property/DockTabs";
import { WindowChrome } from "../shell/WindowChrome";
import { ScenarioCard } from "./ScenarioCard";
import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyDeck,
  SafetyEnvelope,
  SafetyFamily,
} from "../../wire/types";

export function ScenarioDeck({
  family,
  deck,
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
  family: SafetyFamily;
  deck: SafetyDeck;
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
  const scenarios = envelope.study.scenarios.filter((scenario) => scenario.family === family.id);
  const current = scenarios.find((scenario) => scenario.id === active) ?? scenarios[0];
  const form = catalogue?.safety?.find(
    (entry) => entry.id === current?.calculation,
  )?.form;
  // The deck's model list is the catalogue's `sub_interface`, not `family`: the bow-tie is in
  // the risk family and is its own view, so it must not be offered as a scenario here.
  const models = (catalogue?.safety ?? []).filter(
    (entry) => entry.sub_interface === family.id,
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
        <WindowChrome title={deck.title}>
          <input
            className="calc-search"
            value={query}
            aria-label={deck.search}
            placeholder={deck.search}
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
        <WindowChrome title={current?.name ?? deck.heading}>
          {current === undefined ? (
            <div className="calc-starter">
              <h2>{deck.heading}</h2>
              <p>{family.blurb}</p>
              <ol>
                <li>{deck.step}</li>
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
                label={`Safety ${deck.heading.toLowerCase()}`}
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
