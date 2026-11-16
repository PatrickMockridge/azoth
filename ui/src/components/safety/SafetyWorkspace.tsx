/**
 * The Safety workspace's sub-interfaces.
 *
 * **One document, six readings.** Overview lists what the study holds; Bow-tie draws the causes
 * and outcomes; the five scenario decks run their family's models. A tab is a view and never a
 * second document, so switching one cannot lose an edit.
 */

import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyEnvelope,
} from "../../wire/types";
import { BowtieView } from "./BowtieView";
import { ScenarioDeck } from "./ScenarioDeck";
import { SafetyOverview } from "./SafetyOverview";
import { isScenarioView, type SafetyView } from "./views";

const VIEWS: { id: SafetyView; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "bowtie", label: "Bow-tie" },
  { id: "fire", label: "Fire" },
  { id: "dispersion", label: "Dispersion" },
  { id: "firewater", label: "Firewater" },
  { id: "overpressure", label: "Relief" },
  { id: "risk", label: "Risk" },
];

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
  const selectScenario = (id: string) => {
    const row = envelope?.study.scenarios.find((scenario) => scenario.id === id);
    const entry = catalogue?.safety?.find((model) => model.id === row?.calculation);
    // A scenario whose calculation has no deck (a hand-written release or rupture id) is listed
    // with its unsupported diagnostic on the overview, never silently dropped.
    const target = entry?.sub_interface ?? row?.family ?? null;
    onActiveScenario(id);
    onView(isScenarioView(target) ? target : "overview");
  };

  return (
    <div className="safety-workspace">
      <nav className="safety-views" aria-label="Safety view">
        {VIEWS.map((entry) => (
          <button
            key={entry.id}
            type="button"
            className={entry.id === view ? "active" : ""}
            aria-pressed={entry.id === view}
            onClick={() => onView(entry.id)}
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
          onBowtie={onActiveBowtie}
          onScenario={selectScenario}
          onAddBowtie={onAddBowtie}
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
      ) : (
        <ScenarioDeck
          key={view}
          view={view}
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
