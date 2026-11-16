/**
 * The document records a view stands for.
 *
 * **The envelope carries more than the command does.** A bow-tie view adds a barrier's side and a
 * consequence's scenario name and readiness; a command writes exactly the record the library's
 * `Bowtie` reads. One translation lives here rather than in each component, and it is the only
 * place a display field is dropped.
 */

import type { BowtieRecord } from "../../wire/safetyCommands";
import type { SafetyBowtieView } from "../../wire/types";

/** The barrier type the command model carries, from the string the wire carries. */
export function barrierType(value: string): BowtieRecord["barriers"][number]["barrier_type"] {
  if (value === "mitigation" || value === "both") {
    return value;
  }
  return "prevention";
}

/** The bow-tie record a view stands for, without the envelope's display-only additions. */
export function recordOf(bowtie: SafetyBowtieView): BowtieRecord {
  return {
    id: bowtie.id,
    name: bowtie.name,
    top_event: bowtie.top_event,
    threats: bowtie.threats.map((threat) => ({
      id: threat.id,
      label: threat.label,
      frequency: threat.frequency,
      active: threat.active,
    })),
    barriers: bowtie.barriers.map((barrier) => ({
      id: barrier.id,
      label: barrier.label,
      pfd: barrier.pfd,
      barrier_type: barrierType(barrier.barrier_type),
      functional: barrier.functional,
    })),
    consequences: bowtie.consequences.map((consequence) => ({
      id: consequence.id,
      label: consequence.label,
      severity: consequence.severity,
      ...(consequence.scenario === null ? {} : { scenario: consequence.scenario }),
    })),
  };
}

/** A fresh row id that the record does not already carry. */
export function nextRowId(prefix: string, taken: readonly string[]): string {
  const used = new Set(taken);
  for (let index = 1; index < 100; index += 1) {
    const candidate = `${prefix}_${index}`;
    if (!used.has(candidate)) {
      return candidate;
    }
  }
  return `${prefix}_100`;
}
