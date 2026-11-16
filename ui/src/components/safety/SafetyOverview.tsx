/**
 * The safety study's overview.
 *
 * **What this workspace can run, and what it can only point at.** Five fire models and the
 * bow-tie are native here; the other eight families are listed with their manual pages, because
 * a card that ran nothing would be a promise the workspace does not keep.
 */

import { calcManualUrl } from "../../manual";
import type { Catalogue, SafetyEnvelope } from "../../wire/types";
import type { SafetyView } from "./SafetyWorkspace";

const FAMILIES: { id: string; label: string }[] = [
  { id: "dispersion", label: "Dispersion" },
  { id: "fire", label: "Fire and explosion" },
  { id: "firewater", label: "Firewater" },
  { id: "overpressure", label: "Overpressure and relief" },
  { id: "qra", label: "Quantitative risk" },
  { id: "release", label: "Release" },
  { id: "risk", label: "Risk and SIL" },
  { id: "rupture", label: "Rupture" },
];

export function SafetyOverview({
  envelope,
  catalogue,
  onView,
  onBowtie,
  onScenario,
  onAddBowtie,
}: {
  envelope: SafetyEnvelope;
  catalogue: Catalogue | null;
  onView: (view: SafetyView) => void;
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
        <article className="safety-card">
          <h2>Fire scenarios</h2>
          <p className="safety-count">{study.scenarios.length}</p>
          <p>
            Jet fire, pool fire, BLEVE, vapour-cloud explosion and passive fire protection, each
            one a registered model with its own inputs and a distance-to-threshold table.
          </p>
          <div className="safety-actions">
            <button type="button" onClick={() => onView("fire")}>
              Open the fire view
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
          Every calculation whose source is under <code>process/safety/</code>. The five fire
          models and the bow-tie run here; the rest are not runnable in this workspace yet, and
          their manual pages state the inputs, the valid range and the worked example.
        </p>
        {FAMILIES.map((family) => {
          const rows = entries.filter((entry) => entry.family === family.id);
          return rows.length === 0 ? null : (
            <div className="safety-family" key={family.id}>
              <h3>{family.label}</h3>
              <ul>
                {rows.map((entry) => (
                  <li key={entry.id}>
                    <a
                      href={calcManualUrl(entry.id)}
                      target="_blank"
                      rel="noreferrer"
                      title="the generated manual page for this calculation"
                    >
                      {entry.name}
                    </a>
                    {entry.native ? <span className="safety-native">native here</span> : null}
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </section>
    </div>
  );
}
