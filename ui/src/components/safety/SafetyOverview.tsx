/**
 * The safety study's overview.
 *
 * **What this study holds, and where the whole surface lives.** The deck cards and the family
 * index are drawn from the library's family registry, so a family the catalogue gains appears
 * here without a second list; the Catalogue tab is where every calculation is browsable, with
 * its status and its manual page.
 */

import type { Catalogue, SafetyEnvelope } from "../../wire/types";
import type { SafetyView } from "./views";

export function SafetyOverview({
  envelope,
  catalogue,
  onView,
  onCatalogue,
  onBowtie,
  onScenario,
  onAddBowtie,
}: {
  envelope: SafetyEnvelope;
  catalogue: Catalogue | null;
  onView: (view: SafetyView) => void;
  onCatalogue: (family: string) => void;
  onBowtie: (id: string) => void;
  onScenario: (id: string) => void;
  onAddBowtie: () => void;
}) {
  const study = envelope.study;
  const diagnostics = [
    ...study.scenarios.flatMap((scenario) =>
      scenario.diagnostics.map((entry) => ({ where: scenario.name, ...entry })),
    ),
    ...study.bowties.flatMap((bowtie) =>
      bowtie.diagnostics.map((entry) => ({ where: bowtie.name, ...entry })),
    ),
  ];
  const errors = diagnostics.filter((entry) => entry.severity === "error");
  const warnings = diagnostics.filter((entry) => entry.severity === "warning");
  const entries = catalogue?.safety ?? [];
  const families = catalogue?.safety_families ?? [];
  const nativeScenarios = entries.filter((entry) => entry.form !== undefined).length;
  const decks = families.flatMap((family) =>
    family.deck === null ? [] : [{ family, deck: family.deck }],
  );

  return (
    <div className="safety-overview">
      <section className="safety-cards" aria-label="This study">
        <article className="safety-card">
          <h2>Bow-ties</h2>
          <p className="safety-count">{study.bowties.length}</p>
          <p>
            Threats, prevention barriers, the top event and its consequences. The frequency and
            RRF are <code>process.bow_tie</code>'s arithmetic.
          </p>
          <div className="safety-actions">
            <button type="button" onClick={() => onView("bowtie")}>
              Open the bow-tie view
            </button>
            <button type="button" onClick={onAddBowtie}>
              Add a bow-tie
            </button>
          </div>
        </article>
        {decks.map(({ family, deck }) => (
          <article className="safety-card" key={family.id}>
            <h2>{deck.heading}</h2>
            <p className="safety-count">
              {study.scenarios.filter((scenario) => scenario.family === family.id).length}
            </p>
            <p>{family.blurb}</p>
            <div className="safety-actions">
              <button type="button" onClick={() => onView(family.id)}>
                Open the {deck.heading.toLowerCase()} view
              </button>
            </div>
          </article>
        ))}
        <article className="safety-card">
          <h2>Safety catalogue</h2>
          <p className="safety-count">{entries.length}</p>
          <p>
            {families.length} families; {nativeScenarios} scenario models and the bow-tie run
            here. Every calculation is listed with its manual page.
          </p>
          <div className="safety-actions">
            <button type="button" onClick={() => onView("catalogue")}>
              Browse the catalogue
            </button>
          </div>
        </article>
      </section>

      {study.bowties.length === 0 ? null : (
        <section className="safety-list">
          <h2>Bow-ties in this study</h2>
          {study.bowties.map((bowtie) => (
            <button key={bowtie.id} type="button" onClick={() => onBowtie(bowtie.id)}>
              {bowtie.name}
              <span className="safety-meta">
                {bowtie.counts.threats} threats · {bowtie.counts.consequences} consequences
              </span>
            </button>
          ))}
        </section>
      )}

      {study.scenarios.length === 0 ? null : (
        <section className="safety-list">
          <h2>Scenarios in this study</h2>
          {study.scenarios.map((scenario) => (
            <button key={scenario.id} type="button" onClick={() => onScenario(scenario.id)}>
              {scenario.name}
              <span className="safety-meta">
                {scenario.calculation}
                {scenario.family === null ? "" : ` · ${scenario.family}`}
              </span>
            </button>
          ))}
        </section>
      )}

      {diagnostics.length === 0 ? null : (
        <section className="safety-diagnostics">
          <h2>Study messages</h2>
          <p className="note">
            {errors.length} error{errors.length === 1 ? "" : "s"}, {warnings.length} warning
            {warnings.length === 1 ? "" : "s"}
          </p>
          {diagnostics.map((entry, index) => (
            <p key={`${entry.where}:${entry.code}:${index}`} className={entry.severity}>
              <strong>{entry.where}</strong> {entry.message}
            </p>
          ))}
        </section>
      )}

      <section className="safety-families">
        <h2>The safety surface</h2>
        <p className="note">
          Every calculation whose source is under <code>process/safety/</code>. {nativeScenarios}{" "}
          scenario models and the bow-tie run here; the rest are listed with the refusal their
          shape earns and a manual page. Choose a family to open the catalogue there.
        </p>
        <ul className="safety-family-index">
          {families.map((family) => (
            <li key={family.id}>
              <button type="button" onClick={() => onCatalogue(family.id)}>
                <span>{family.label}</span>
                <span className="safety-meta">
                  {entries.filter((entry) => entry.family === family.id).length} calculations
                </span>
              </button>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
