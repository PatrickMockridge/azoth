/**
 * The Safety workspace's sub-interfaces.
 *
 * **One document, the fixed readings and every deck the registry serves.** Overview lists what
 * the study holds, Catalogue browses all 96 calculations across the 23 families, Bow-tie draws
 * the causes and outcomes, and each deck runs its family's scenarios. A tab is a view and never
 * a second document, so switching one cannot lose an edit.
 *
 * **The tabs are the library's answer.** The deck tabs come from `catalogue.safety_families`,
 * not a list here: a family the registry gains is a tab, and a family it loses is gone, without
 * this file changing.
 */

import { useState } from "react";

import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyEnvelope,
} from "../../wire/types";
import { BowtieView } from "./BowtieView";
import { SafetyCatalogue } from "./SafetyCatalogue";
import { ScenarioDeck } from "./ScenarioDeck";
import { SafetyOverview } from "./SafetyOverview";
import { deckFamily, isDeck, type SafetyView } from "./views";

export function SafetyWorkspace({
  view,
  onView,
  envelope,
  catalogue,
  units,
  activeBowtie,
  activeScenario,
  onActiveBowtie,
  onActiveScenario,
  onAddScenario,
  onAddBowtie,
  onRemoveScenario,
  onRemoveBowtie,
  onCommand,
  onCalculate,
  onMoveNode,
}: {
  view: SafetyView;
  onView: (view: SafetyView) => void;
  envelope: SafetyEnvelope | null;
  catalogue: Catalogue | null;
  units: Units;
  activeBowtie: string | null;
  activeScenario: string | null;
  onActiveBowtie: (id: string | null) => void;
  onActiveScenario: (id: string | null) => void;
  onAddScenario: (entry: SafetyCalculation) => void;
  onAddBowtie: () => void;
  onRemoveScenario: (id: string) => void;
  onRemoveBowtie: (id: string) => void;
  onCommand: (command: SafetyCommand) => void;
  onCalculate: (target: SafetyTarget | null) => void;
  onMoveNode: (bowtie: string, node: string, x: number, y: number) => void;
}) {
  const families = catalogue?.safety_families ?? [];
  /** The family the Overview asked the Catalogue to open at. */
  const [focus, setFocus] = useState<string | null>(null);

  const choose = (next: SafetyView) => {
    setFocus(null);
    onView(next);
  };

  const openCatalogue = (family: string) => {
    setFocus(family);
    onView("catalogue");
  };

  const selectScenario = (id: string) => {
    const row = envelope?.study.scenarios.find((scenario) => scenario.id === id);
    const entry = catalogue?.safety?.find((model) => model.id === row?.calculation);
    // A scenario whose calculation has no deck (a hand-written release or rupture id) is listed
    // with its unsupported diagnostic on the overview, never silently dropped.
    const target = entry?.sub_interface ?? row?.family ?? null;
    onActiveScenario(id);
    onView(isDeck(target, families) ? target : "overview");
  };

  const tabs: { id: SafetyView; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "catalogue", label: "Catalogue" },
    { id: "bowtie", label: "Bow-tie" },
    ...families.flatMap((family) =>
      family.deck === null ? [] : [{ id: family.id, label: family.deck.tab }],
    ),
  ];

  const deck = deckFamily(view, families);

  return (
    <div className="safety-workspace">
      <nav className="safety-views" aria-label="Safety view">
        {tabs.map((entry) => (
          <button
            key={entry.id}
            type="button"
            className={entry.id === view ? "active" : ""}
            aria-pressed={entry.id === view}
            onClick={() => choose(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </nav>
      {envelope === null ? (
        <p className="note">loading the safety study…</p>
      ) : view === "overview" ? (
        <SafetyOverview
          envelope={envelope}
          catalogue={catalogue}
          onView={onView}
          onCatalogue={openCatalogue}
          onBowtie={onActiveBowtie}
          onScenario={selectScenario}
          onAddBowtie={onAddBowtie}
        />
      ) : view === "catalogue" ? (
        <SafetyCatalogue
          catalogue={catalogue}
          focus={focus}
          onView={onView}
          onAddScenario={onAddScenario}
        />
      ) : view === "bowtie" ? (
        <BowtieView
          envelope={envelope}
          units={units}
          active={activeBowtie}
          onActive={onActiveBowtie}
          onAdd={onAddBowtie}
          onRemove={onRemoveBowtie}
          onCommand={onCommand}
          onCalculate={onCalculate}
          onMoveNode={onMoveNode}
          onScenario={selectScenario}
        />
      ) : deck === null ? (
        <SafetyCatalogue
          catalogue={catalogue}
          focus={null}
          onView={onView}
          onAddScenario={onAddScenario}
        />
      ) : (
        <ScenarioDeck
          key={view}
          family={deck.family}
          deck={deck.deck}
          envelope={envelope}
          catalogue={catalogue}
          units={units}
          active={activeScenario}
          onActive={onActiveScenario}
          onAdd={onAddScenario}
          onRemove={onRemoveScenario}
          onCommand={onCommand}
          onCalculate={onCalculate}
        />
      )}
    </div>
  );
}
