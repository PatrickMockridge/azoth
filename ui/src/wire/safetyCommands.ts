/**
 * One edit to a safety study.
 *
 * **A closed union, as the flowsheet's commands are.** A scenario input is an SI magnitude for a
 * quantity; a bow-tie is sent whole because it is one record the inspector edits. The tag is the
 * library's `Command` enum's own `snake_case` spelling.
 */

/** One bow-tie, as a command writes it. The shape the library's `Bowtie` reads. */
export interface BowtieRecord {
  id: string;
  name: string;
  top_event: string;
  threats: {
    id: string;
    label: string;
    frequency: number;
    active: boolean;
  }[];
  barriers: {
    id: string;
    label: string;
    pfd: number;
    barrier_type: "prevention" | "mitigation" | "both";
    functional: boolean;
  }[];
  consequences: {
    id: string;
    label: string;
    severity: number;
    scenario?: string;
  }[];
}

export type SafetyCommand =
  | { command: "set_study_name"; name: string }
  | { command: "add_scenario"; id: string; calculation: string; name?: string }
  | { command: "remove_scenario"; id: string }
  | { command: "rename_scenario"; id: string; name: string }
  | {
      command: "set_scenario_input";
      scenario: string;
      input: string;
      value: number | boolean | string;
    }
  | { command: "unset_scenario_input"; scenario: string; input: string }
  | { command: "set_bowtie"; bowtie: BowtieRecord }
  | { command: "remove_bowtie"; id: string }
  | { command: "set_bowtie_position"; bowtie: string; node: string; x: number; y: number };

/** Every command's tag, in the order the library declares them. */
export const SAFETY_COMMAND_NAMES: readonly SafetyCommand["command"][] = [
  "set_study_name",
  "add_scenario",
  "remove_scenario",
  "rename_scenario",
  "set_scenario_input",
  "unset_scenario_input",
  "set_bowtie",
  "remove_bowtie",
  "set_bowtie_position",
];
