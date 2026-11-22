/**
 * Every safety calculation, searchable, grouped by the registry's families.
 *
 * **The one surface on which all 96 appear.** The ten decks run their family's scenarios and
 * list the rest refused; the thirteen families without a deck had no surface at all. Here every
 * calculation has a row, its family's label, the status its shape earns and a link to its
 * manual page — a listed model is reachable rather than absent.
 *
 * **The registry is the catalogue's.** Families, their labels, their blurbs and the deck copy
 * are served with the calculations, so this component holds no list of them.
 */

import { useEffect, useState } from "react";

import { calcManualUrl } from "../../manual";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyFamily,
} from "../../wire/types";
import type { SafetyView } from "./views";

function badgeOf(entry: SafetyCalculation): { text: string; className: string } {
  if (entry.sub_interface === "bowtie") {
    return { text: "bow-tie", className: "native" };
  }
  if (entry.native) {
    return { text: "scenario", className: "native" };
  }
  return { text: "listed", className: "listed" };
}

function deckTabOf(families: readonly SafetyFamily[], id: string | null): string {
  return families.find((family) => family.id === id)?.deck?.tab ?? "scenario";
}

export function SafetyCatalogue({
  catalogue,
  focus,
  onView,
  onAddScenario,
}: {
  catalogue: Catalogue | null;
  /** The family the Overview asked the catalogue to open at, or `null`. */
  focus: string | null;
  onView: (view: SafetyView) => void;
  onAddScenario: (entry: SafetyCalculation) => void;
}) {
  const [query, setQuery] = useState("");
  const entries = catalogue?.safety ?? [];
  const families = catalogue?.safety_families ?? [];
  const nativeScenarios = entries.filter((entry) => entry.form !== undefined).length;
  const needle = query.trim().toLowerCase();

  const matches = (entry: SafetyCalculation, family: SafetyFamily): boolean =>
    needle === "" ||
    [entry.id, entry.name, entry.purpose, family.label, family.blurb].some((text) =>
      text.toLowerCase().includes(needle),
    );

  const shown = families.reduce(
    (count, family) =>
      count + entries.filter((entry) => entry.family === family.id && matches(entry, family)).length,
    0,
  );

  useEffect(() => {
    if (focus === null) {
      return;
    }
    const section = document.getElementById(`safety-family-${focus}`);
    section?.scrollIntoView?.({ block: "start" });
  }, [focus]);

  const jump = (family: string) => {
    document.getElementById(`safety-family-${family}`)?.scrollIntoView?.({ block: "start" });
  };

  return (
    <div className="safety-catalogue">
      <header className="safety-catalogue-head">
        <h2>Safety catalogue</h2>
        <p className="note">
          {entries.length} calculations · {families.length} families · {nativeScenarios} scenario
          models and the bow-tie run here
        </p>
      </header>

      <div className="safety-catalogue-tools">
        <input
          className="calc-search"
          type="search"
          value={query}
          aria-label="Search the safety catalogue"
          placeholder="Search the safety catalogue"
          onChange={(event) => setQuery(event.target.value)}
        />
        <p className="note" role="status" aria-live="polite">
          {shown} of {entries.length} calculations
        </p>
      </div>

      <nav className="safety-family-index" aria-label="Safety families">
        {families.map((family) => (
          <button key={family.id} type="button" onClick={() => jump(family.id)}>
            {family.label}
          </button>
        ))}
      </nav>

      <div className="safety-catalogue-families">
        {families.map((family) => {
          const rows = entries.filter(
            (entry) => entry.family === family.id && matches(entry, family),
          );
          if (rows.length === 0) {
            return null;
          }
          return (
            <section
              className={
                focus === family.id
                  ? "safety-catalogue-family focused"
                  : "safety-catalogue-family"
              }
              key={family.id}
              id={`safety-family-${family.id}`}
              data-family-id={family.id}
            >
              <header className="safety-catalogue-family-head">
                <h3>{family.label}</h3>
                <span className="safety-meta">{rows.length} calculations</span>
              </header>
              <p className="note">{family.blurb}</p>
              {family.deck === null ? null : (
                <button type="button" onClick={() => onView(family.id)}>
                  Open the {family.deck.tab} deck
                </button>
              )}
              <ul className="safety-catalogue-list">
                {rows.map((entry) => {
                  const badge = badgeOf(entry);
                  return (
                    <li
                      className="safety-calculation"
                      key={entry.id}
                      data-calculation-id={entry.id}
                    >
                      <div className="safety-calculation-head">
                        <a
                          className="safety-name"
                          href={calcManualUrl(entry.id)}
                          target="_blank"
                          rel="noreferrer"
                        >
                          {entry.name}
                        </a>
                        <code>{entry.id}</code>
                        <span className={`safety-badge ${badge.className}`}>{badge.text}</span>
                      </div>
                      {entry.purpose === "" ? null : (
                        <p className="calc-entry-summary">{entry.purpose}</p>
                      )}
                      {entry.native ? null : (
                        <p className="refusal">
                          {entry.refusal ?? "not runnable in this workspace yet"}
                        </p>
                      )}
                      <div className="safety-row-actions">
                        {entry.form === undefined ? null : (
                          <button
                            type="button"
                            aria-label={`Add ${entry.name} as a scenario`}
                            onClick={() => onAddScenario(entry)}
                          >
                            Add as a scenario
                          </button>
                        )}
                        {entry.sub_interface === "bowtie" ? (
                          <button type="button" onClick={() => onView("bowtie")}>
                            Open the bow-tie view
                          </button>
                        ) : null}
                        {entry.sub_interface === null || entry.sub_interface === "bowtie" ? null : (
                          <button type="button" onClick={() => onView(entry.sub_interface ?? "")}>
                            Open the {deckTabOf(families, entry.sub_interface)} deck
                          </button>
                        )}
                        <a
                          className="calc-manual-link"
                          href={calcManualUrl(entry.id)}
                          target="_blank"
                          rel="noreferrer"
                        >
                          Manual page
                        </a>
                      </div>
                    </li>
                  );
                })}
              </ul>
            </section>
          );
        })}
        {shown === 0 ? <p className="note">nothing matches “{query}”</p> : null}
      </div>
    </div>
  );
}
