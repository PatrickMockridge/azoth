/**
 * The safety workspace's views, held to the served registry.
 *
 * **A deck is a family the catalogue serves with one.** The ten ids used to be a literal union in
 * `views.ts`; the predicate now asks the registry, so these tests are about the answer rather
 * than about a list beside it.
 */

import { describe, expect, it } from "vitest";

import { deckFamily, isDeck } from "../src/components/safety/views";
import { decodeCatalogue } from "../src/wire/decode";
import catalogueJson from "./fixtures/catalogue.json";

const families = decodeCatalogue(catalogueJson).safety_families ?? [];

describe("the safety views", () => {
  it("accepts a family the registry gives a deck", () => {
    expect(isDeck("fire", families)).toBe(true);
    expect(deckFamily("fire", families)?.deck.tab).toBe("Fire");
    expect(deckFamily("depressurization", families)?.deck.tab).toBe("Depressurization");
  });

  it("refuses a listed family, a missing view and an unknown id", () => {
    expect(isDeck("esd", families)).toBe(false);
    expect(isDeck(null, families)).toBe(false);
    expect(isDeck(undefined, families)).toBe(false);
    expect(isDeck("overview", families)).toBe(false);
    expect(isDeck("no_such_family", families)).toBe(false);
    expect(deckFamily("release", families)).toBeNull();
  });
});
