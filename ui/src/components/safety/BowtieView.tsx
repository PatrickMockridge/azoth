/**
 * The bow-tie sub-interface: a list of diagrams, and one open at a time.
 *
 * **The canvas and the inspector are two readings of one projection.** Selecting a plate is UI
 * state; the plate's contents are the envelope's; an edit sends the whole bow-tie record.
 */

import { useEffect, useState } from "react";

import { WindowChrome } from "../shell/WindowChrome";
import type { Units } from "../../state/units";
import type { SafetyTarget } from "../../wire/session";
import type { SafetyCommand } from "../../wire/safetyCommands";
import type { SafetyEnvelope } from "../../wire/types";
import { BowtieCanvas } from "./BowtieCanvas";
import { BowtieInspector } from "./BowtieInspector";

export function BowtieView({
  envelope,
  active,
  onActive,
  onAdd,
  onRemove,
  onCommand,
  onCalculate,
  onMoveNode,
  onScenario,
}: {
  envelope: SafetyEnvelope;
  units: Units;
  active: string | null;
  onActive: (id: string | null) => void;
  onAdd: () => void;
  onRemove: (id: string) => void;
  onCommand: (command: SafetyCommand) => void;
  onCalculate: (target: SafetyTarget | null) => void;
  onMoveNode: (bowtie: string, node: string, x: number, y: number) => void;
  onScenario: (id: string) => void;
}) {
  const bowties = envelope.study.bowties;
  const current = bowties.find((bowtie) => bowtie.id === active) ?? bowties[0];
  const [selected, setSelected] = useState<string | null>(null);
  const currentId = current?.id ?? null;

  useEffect(() => {
    setSelected(null);
  }, [currentId]);

  return (
    <div className="safety-bowtie">
      <aside className="safety-side" data-window="picker">
        <WindowChrome title="Bow-ties">
          <div className="calc-list">
            {bowties.map((bowtie) => (
              <div className="calc-entry" key={bowtie.id}>
                <button
                  type="button"
                  className={bowtie.id === current?.id ? "active" : ""}
                  onClick={() => onActive(bowtie.id)}
                >
                  {bowtie.name}
                </button>
                <p className="calc-entry-summary">{bowtie.top_event}</p>
              </div>
            ))}
            {bowties.length === 0 ? (
              <p className="note">No bow-tie yet. Add one to describe a top event.</p>
            ) : null}
            <button type="button" className="safety-add" onClick={onAdd}>
              Add a bow-tie
            </button>
          </div>
        </WindowChrome>
      </aside>
      <section className="safety-main" data-window="worksheet">
        <WindowChrome title={current?.name ?? "Bow-ties"}>
          {current === undefined ? (
            <div className="calc-starter">
              <h2>Bow-ties</h2>
              <p>
                A bow-tie names the threats that can cause a top event, the prevention barriers
                between them, and the consequences on the other side. Its frequency and RRF are
                the <code>process.bow_tie</code> arithmetic.
              </p>
            </div>
          ) : (
            <>
              <header className="safety-scenario-head">
                <div>
                  <h2>{current.name}</h2>
                  <p className="calc-entry-summary">{current.top_event}</p>
                </div>
                <div className="safety-actions">
                  <button
                    type="button"
                    className="primary"
                    onClick={() => onCalculate({ bowtie: current.id })}
                    disabled={!current.ready}
                    title={
                      current.ready
                        ? "calculate this bow-tie"
                        : "fix the diagnostics on the plates first"
                    }
                  >
                    Calculate
                  </button>
                  <button
                    type="button"
                    onClick={() => onRemove(current.id)}
                    title="remove this bow-tie from the study"
                  >
                    Remove
                  </button>
                </div>
              </header>

              {current.diagnostics.length === 0 ? null : (
                <div className="calc-diagnostics">
                  {current.diagnostics.map((diagnostic) => (
                    <p key={`${diagnostic.code}:${diagnostic.field ?? ""}`}>
                      {diagnostic.message}
                    </p>
                  ))}
                </div>
              )}
              {current.error === null ? null : (
                <div className="fault">
                  {current.error.message}
                  {current.error.field === null ? null : (
                    <span className="where"> · {current.error.field}</span>
                  )}
                </div>
              )}

              <div className="safety-bowtie-grid">
                <div className="safety-canvas">
                  <BowtieCanvas
                    bowtie={current}
                    selected={selected}
                    onSelect={setSelected}
                    onMove={(node, x, y) => onMoveNode(current.id, node, x, y)}
                  />
                </div>
                <BowtieInspector
                  bowtie={current}
                  selected={selected}
                  scenarios={envelope.study.scenarios.map((scenario) => ({
                    id: scenario.id,
                    name: scenario.name,
                  }))}
                  onCommand={onCommand}
                  onScenario={onScenario}
                />
              </div>
            </>
          )}
        </WindowChrome>
      </section>
    </div>
  );
}
