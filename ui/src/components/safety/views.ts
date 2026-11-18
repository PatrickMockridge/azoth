/**
 * The safety workspace's views.
 *
 * **One union, one place.** The tab list, the overview cards and the routing all read the same
 * ten scenario family ids. A view is a reading of the one study, never a second document.
 */

export const SCENARIO_VIEWS = [
  "fire",
  "dispersion",
  "equipmentsafety",
  "firewater",
  "overpressure",
  "processsafetysystem",
  "risk",
  "safetyassessment",
  "vibration",
  "depressurization",
] as const;

/** A family with a deck of scenario cards. */
export type ScenarioView = (typeof SCENARIO_VIEWS)[number];

/** Every reading of the study: the overview, the bow-tie, and the ten scenario decks. */
export type SafetyView = "overview" | "bowtie" | ScenarioView;

/** Whether a catalogue entry's `sub_interface` names a deck. */
export function isScenarioView(value: string | null | undefined): value is ScenarioView {
  switch (value) {
    case "fire":
    case "dispersion":
    case "equipmentsafety":
    case "firewater":
    case "overpressure":
    case "processsafetysystem":
    case "risk":
    case "safetyassessment":
    case "vibration":
    case "depressurization":
      return true;
    default:
      return false;
  }
}
