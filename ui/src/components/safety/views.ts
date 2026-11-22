/**
 * The safety workspace's views.
 *
 * **The registry is the catalogue's, not this file's.** A view is one of the three fixed
 * readings — the overview, the catalogue, the bow-tie — or a family id the served registry gives
 * a deck. The ten deck ids used to be a literal union here, which was a fifth copy of a list the
 * library already states; a family it gains is now a tab with no edit in this file.
 */

import type { SafetyDeck, SafetyFamily } from "../../wire/types";

/** The three readings every study has. */
export type FixedSafetyView = "overview" | "catalogue" | "bowtie";

/** A view is a fixed reading or a family id; the registry decides which ids have a deck. */
export type SafetyView = string;

/** The family row and the deck copy for a view, or `null` when the view has no deck. */
export function deckFamily(
  id: string | null | undefined,
  families: readonly SafetyFamily[],
): { family: SafetyFamily; deck: SafetyDeck } | null {
  const family = families.find((entry) => entry.id === id);
  if (family === undefined || family.deck === null) {
    return null;
  }
  return { family, deck: family.deck };
}

/** Whether a view id names a family the registry gives a deck. */
export function isDeck(
  id: string | null | undefined,
  families: readonly SafetyFamily[],
): id is string {
  return deckFamily(id, families) !== null;
}
