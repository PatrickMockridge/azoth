/**
 * The Safety workspace, as its own document and its own envelope.
 *
 * **Mounted beside the other workspaces and hidden when it is not in use.** Switching workspaces
 * must not throw the study away, so every surface stays mounted and one is `display: none`; each
 * still reads only its own envelope.
 */

import { useCallback, useEffect, useRef, useState } from "react";

import blankStudy from "../../../../specs/safety/blank.toml?raw";
import demoStudy from "../../../../specs/safety/demo.toml?raw";
import { CommandIcon } from "../CommandIcon";
import { standardMenus, type CommandSpec, type MenuSpec } from "../../commands/model";
import { manualHome } from "../../manual";
import { saveText } from "../../save";
import { RibbonGroup } from "../RibbonGroup";
import { WindowChrome } from "../shell/WindowChrome";
import type { Units } from "../../state/units";
import type { Appearance } from "../../state/appearance";
import type {
  SafetyDoor,
  SafetyOpened,
  SafetySession,
  SafetyTarget,
} from "../../wire/session";
import type { BowtieRecord, SafetyCommand } from "../../wire/safetyCommands";
import type {
  Catalogue,
  SafetyCalculation,
  SafetyEnvelope,
} from "../../wire/types";
import { SafetyTitleStrip } from "./SafetyTitleStrip";
import { SafetyWorkspace } from "./SafetyWorkspace";
import { isScenarioView, type SafetyView } from "./views";
import { caseMenuCommands, type InterfaceId } from "../../interfaces";

/** One study the workspace has stood at, for undo and redo. */
interface Snapshot {
  document: string;
  view: SafetyView;
  bowtie: string | null;
  scenario: string | null;
}

function fileStem(name: string, id: string): string {
  const stem = name.trim().replace(/[^A-Za-z0-9._-]+/g, "_");
  return stem === "" ? id : stem;
}

export function SafetyApp({
  door,
  catalogue,
  units,
  appearance,
  version,
  interfaceId,
  hidden,
  onInterface,
  onAppearance,
  onUnits,
}: {
  door: SafetyDoor | null;
  catalogue: Catalogue | null;
  units: Units;
  appearance: Appearance;
  version: string;
  interfaceId: InterfaceId;
  hidden: boolean;
  onInterface: (id: InterfaceId) => void;
  onAppearance: (appearance: Appearance) => void;
  onUnits: (id: string) => void;
}) {
  const [session, setSession] = useState<SafetySession | null>(null);
  const [envelope, setEnvelope] = useState<SafetyEnvelope | null>(null);
  const [savedDocument, setSavedDocument] = useState<string | null>(null);
  const [view, setView] = useState<SafetyView>("overview");
  const [activeBowtie, setActiveBowtie] = useState<string | null>(null);
  const [activeScenario, setActiveScenario] = useState<string | null>(null);
  const [fault, setFault] = useState<string | null>(null);
  const [history, setHistory] = useState<Snapshot[]>([]);
  const [future, setFuture] = useState<Snapshot[]>([]);

  const issued = useRef(0);
  const openInput = useRef<HTMLInputElement>(null);
  const lastDocument = useRef<string | null>(null);
  const viewRef = useRef<SafetyView>("overview");
  const bowtieRef = useRef<string | null>(null);
  const scenarioRef = useRef<string | null>(null);

  useEffect(() => {
    viewRef.current = view;
    bowtieRef.current = activeBowtie;
    scenarioRef.current = activeScenario;
  }, [view, activeBowtie, activeScenario]);

  const openable = door?.kind === "openable";
  const unsaved =
    envelope !== null &&
    savedDocument !== null &&
    envelope.study.document !== savedDocument;

  /** One call's answer, or the sentence that refused it. */
  const answer = useCallback((pending: Promise<SafetyEnvelope>, record: boolean) => {
    const ticket = (issued.current += 1);
    void pending.then(
      (next) => {
        if (ticket !== issued.current) {
          return;
        }
        if (record && lastDocument.current !== null && lastDocument.current !== next.study.document) {
          const previous = lastDocument.current;
          const where: Snapshot = {
            document: previous,
            view: viewRef.current,
            bowtie: bowtieRef.current,
            scenario: scenarioRef.current,
          };
          setHistory((entries) => [...entries, where].slice(-50));
          setFuture([]);
        }
        lastDocument.current = next.study.document;
        setEnvelope(next);
        setFault(null);
      },
      (error: unknown) => {
        if (ticket === issued.current) {
          setFault(String(error));
        }
      },
    );
  }, []);

  /** A study opened from a file, the shipped demo, or blank. */
  const land = useCallback((opening: Promise<SafetyOpened>) => {
    const ticket = (issued.current += 1);
    void opening.then(
      (opened) => {
        if (ticket !== issued.current) {
          return;
        }
        setSession(opened.session);
        setEnvelope(opened.envelope);
        setSavedDocument(opened.envelope.study.document);
        lastDocument.current = opened.envelope.study.document;
        setHistory([]);
        setFuture([]);
        setView("overview");
        setActiveBowtie(opened.envelope.study.bowties[0]?.id ?? null);
        setActiveScenario(opened.envelope.study.scenarios[0]?.id ?? null);
        setFault(null);
      },
      (error: unknown) => {
        if (ticket === issued.current) {
          setFault(String(error));
        }
      },
    );
  }, []);

  useEffect(() => {
    if (door === null) {
      return;
    }
    land(door.kind === "openable" ? door.open(demoStudy) : door.attach());
  }, [door, land]);

  const send = useCallback(
    (command: SafetyCommand) => {
      if (session !== null) {
        answer(session.apply(command), true);
      }
    },
    [session, answer],
  );

  const calculate = useCallback(
    (target: SafetyTarget | null) => {
      if (session !== null) {
        answer(session.calculate(target), false);
      }
    },
    [session, answer],
  );

  const addScenario = useCallback(
    (entry: SafetyCalculation) => {
      const id = nextScenarioId(entry, envelope);
      setActiveScenario(id);
      setView(isScenarioView(entry.sub_interface) ? entry.sub_interface : "fire");
      send({ command: "add_scenario", id, calculation: entry.id, name: entry.name });
    },
    [envelope, send],
  );

  const addBowtie = useCallback(() => {
    const id = nextBowtieId(envelope);
    const bowtie: BowtieRecord = {
      id,
      name: "New bow-tie",
      top_event: "Describe the top event",
      threats: [],
      barriers: [],
      consequences: [],
    };
    setActiveBowtie(id);
    setView("bowtie");
    send({ command: "set_bowtie", bowtie });
  }, [envelope, send]);

  const removeScenario = useCallback(
    (id: string) => {
      if (envelope !== null) {
        const rest = envelope.study.scenarios.filter((scenario) => scenario.id !== id);
        setActiveScenario(rest[0]?.id ?? null);
      }
      send({ command: "remove_scenario", id });
    },
    [envelope, send],
  );

  const removeBowtie = useCallback(
    (id: string) => {
      if (envelope !== null) {
        const rest = envelope.study.bowties.filter((bowtie) => bowtie.id !== id);
        setActiveBowtie(rest[0]?.id ?? null);
      }
      send({ command: "remove_bowtie", id });
    },
    [envelope, send],
  );

  const moveBowtieNode = useCallback(
    (bowtie: string, node: string, x: number, y: number) => {
      send({ command: "set_bowtie_position", bowtie, node, x, y });
    },
    [send],
  );

  const save = useCallback(() => {
    if (envelope === null) {
      return;
    }
    const stem = fileStem(envelope.study.name, envelope.study.id);
    void saveText(`${stem}.toml`, envelope.study.document).then(
      () => setSavedDocument(envelope.study.document),
      (error: unknown) => setFault(String(error)),
    );
  }, [envelope]);

  const open = useCallback(
    (text: string) => {
      if (door === null || door.kind !== "openable") {
        return;
      }
      if (
        unsaved &&
        !window.confirm("This safety study has unsaved changes. Open another and lose them?")
      ) {
        return;
      }
      land(door.open(text));
    },
    [door, land, unsaved],
  );

  /** Restore a snapshot through the same open path a file uses. */
  const restore = useCallback(
    (snapshot: Snapshot) => {
      if (door === null || door.kind !== "openable") {
        return;
      }
      const ticket = (issued.current += 1);
      void door.open(snapshot.document).then(
        (opened) => {
          if (ticket !== issued.current) {
            return;
          }
          setSession(opened.session);
          setEnvelope(opened.envelope);
          lastDocument.current = snapshot.document;
          setView(snapshot.view);
          setActiveBowtie(snapshot.bowtie);
          setActiveScenario(snapshot.scenario);
          setFault(null);
        },
        (error: unknown) => {
          if (ticket === issued.current) {
            setFault(String(error));
          }
        },
      );
    },
    [door],
  );

  const undo = useCallback(() => {
    if (!openable || history.length === 0) {
      return;
    }
    const snapshot = history[history.length - 1];
    if (snapshot === undefined) {
      return;
    }
    const current = lastDocument.current ?? "";
    const where: Snapshot = {
      document: current,
      view: viewRef.current,
      bowtie: bowtieRef.current,
      scenario: scenarioRef.current,
    };
    setHistory((entries) => entries.slice(0, -1));
    setFuture((entries) => [...entries, where].slice(-50));
    restore(snapshot);
  }, [history, openable, restore]);

  const redo = useCallback(() => {
    if (!openable || future.length === 0) {
      return;
    }
    const snapshot = future[future.length - 1];
    if (snapshot === undefined) {
      return;
    }
    const current = lastDocument.current ?? "";
    const where: Snapshot = {
      document: current,
      view: viewRef.current,
      bowtie: bowtieRef.current,
      scenario: scenarioRef.current,
    };
    setFuture((entries) => entries.slice(0, -1));
    setHistory((entries) => [...entries, where].slice(-50));
    restore(snapshot);
  }, [future, openable, restore]);

  useEffect(() => {
    if (!unsaved) {
      return;
    }
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [unsaved]);

  const canUndo = openable && history.length > 0;
  const canRedo = openable && future.length > 0;

  const openFile = (file: File): void => {
    void file.text().then(open);
  };

  const caseDocumentCommands: CommandSpec[] = [
    { id: "new", label: "New", menu: "case", run: () => open(blankStudy), enabled: openable },
    {
      id: "open",
      label: "Open…",
      menu: "case",
      run: () => openInput.current?.click(),
      enabled: openable,
    },
    { id: "demo", label: "Demo", menu: "case", run: () => open(demoStudy), enabled: openable },
    { id: "save", label: "Save…", menu: "case", run: save, enabled: session !== null },
  ];
  const otherCommands: CommandSpec[] = [
    { id: "undo", label: "Undo", menu: "edit", run: undo, enabled: canUndo, shortcut: "Ctrl+Z" },
    { id: "redo", label: "Redo", menu: "edit", run: redo, enabled: canRedo, shortcut: "Ctrl+Y" },
    {
      id: "calculate-all",
      label: "Calculate all",
      menu: "run",
      run: () => calculate(null),
      enabled: envelope?.dirty === true,
    },
    {
      id: "manual",
      label: "User manual",
      menu: "help",
      run: () => {
        window.open(manualHome(), "_blank", "noopener");
      },
      enabled: true,
    },
  ];
  const menus: MenuSpec[] = standardMenus([
    ...caseMenuCommands(caseDocumentCommands, { active: interfaceId, onChoose: onInterface }),
    ...otherCommands,
  ]);

  return (
    <div className={`app safety${hidden ? " off" : ""}`} hidden={hidden}>
      <input
        ref={openInput}
        className="sr-only"
        type="file"
        accept=".toml,text/plain"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file !== undefined) {
            openFile(file);
          }
          event.target.value = "";
        }}
      />
      <SafetyTitleStrip
        envelope={envelope}
        menus={menus}
        appearance={appearance}
        units={units}
        unsaved={unsaved}
        version={version}
        onName={(name) => send({ command: "set_study_name", name })}
        onAppearance={onAppearance}
        onUnits={onUnits}
      />

      <div className="ribbon">
        <WindowChrome title="Tools">
          <RibbonGroup label="Run">
            <button
              type="button"
              className="primary"
              onClick={() => calculate(null)}
              disabled={!envelope?.dirty}
              title="run every scenario and bow-tie that is ready and older than its inputs"
            >
              <CommandIcon kind="calculate-all" />
              Calculate all
            </button>
            <button
              type="button"
              onClick={undo}
              disabled={!canUndo}
              title={openable ? "undo the last edit" : "undo is not available on a served study"}
            >
              <CommandIcon kind="undo" />
              Undo
            </button>
            <button
              type="button"
              onClick={redo}
              disabled={!canRedo}
              title={openable ? "redo the last undone edit" : "redo is not available on a served study"}
            >
              <CommandIcon kind="redo" />
              Redo
            </button>
          </RibbonGroup>
          <RibbonGroup label="Study">
            <button
              type="button"
              onClick={() => open(blankStudy)}
              disabled={!openable}
              title={openable ? "a study with no scenarios or bow-ties" : "a served study cannot be replaced"}
            >
              <CommandIcon kind="new" />
              New
            </button>
            <button
              type="button"
              onClick={() => openInput.current?.click()}
              disabled={!openable}
              title={openable ? "open a TOML study" : "a served study cannot be replaced"}
            >
              <CommandIcon kind="open" />
              Open
            </button>
            <button
              type="button"
              onClick={() => open(demoStudy)}
              disabled={!openable}
              title={openable ? "the shipped worked-example study" : "a served study cannot be replaced"}
            >
              <CommandIcon kind="demo" />
              Demo
            </button>
            <button
              type="button"
              onClick={save}
              disabled={session === null}
              title="write the study where you say; it rides every envelope, so this is no call"
            >
              <CommandIcon kind="save" />
              Save
            </button>
          </RibbonGroup>
        </WindowChrome>
      </div>

      <div className="safety-body">
        {fault === null ? null : <div className="fault">{fault}</div>}
        <SafetyWorkspace
          view={view}
          onView={setView}
          envelope={envelope}
          catalogue={catalogue}
          units={units}
          activeBowtie={activeBowtie}
          activeScenario={activeScenario}
          onActiveBowtie={setActiveBowtie}
          onActiveScenario={setActiveScenario}
          onAddScenario={addScenario}
          onAddBowtie={addBowtie}
          onRemoveScenario={removeScenario}
          onRemoveBowtie={removeBowtie}
          onCommand={send}
          onCalculate={calculate}
          onMoveNode={moveBowtieNode}
        />
      </div>
    </div>
  );
}

function nextScenarioId(entry: SafetyCalculation, envelope: SafetyEnvelope | null): string {
  const leaf = entry.id.split(".")[1] ?? "scenario";
  const taken = new Set(envelope?.study.scenarios.map((scenario) => scenario.id) ?? []);
  for (let index = 1; index < 100; index += 1) {
    const candidate = `${leaf}_${index}`;
    if (!taken.has(candidate)) {
      return candidate;
    }
  }
  return `${leaf}_100`;
}

function nextBowtieId(envelope: SafetyEnvelope | null): string {
  const taken = new Set(envelope?.study.bowties.map((bowtie) => bowtie.id) ?? []);
  for (let index = 1; index < 100; index += 1) {
    const candidate = `bowtie_${index}`;
    if (!taken.has(candidate)) {
      return candidate;
    }
  }
  return "bowtie_100";
}

