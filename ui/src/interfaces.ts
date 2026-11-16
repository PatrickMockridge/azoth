/**
 * The interfaces the Case menu offers.
 *
 * **The row is the model.** The menu label, the strip word, the document noun, the
 * document-or-viewer fact, the route, the save format and the mount policy all read from here.
 * A new interface is a row and a render case, not a fourth boolean, a fourth button and a fourth
 * copy of the same menu commands.
 *
 * **Data only.** No component is imported, so an interface cannot create an import cycle; the
 * render case stays where the document and its door are wired.
 */

import type { CommandSpec, MenuId } from "./commands/model";

/** One surface the Case menu offers. */
export interface InterfaceSpec {
  /** Stable key: commands, state and tests. */
  readonly id: string;
  /** The Case-menu option and its accessible name: "Process simulation". */
  readonly label: string;
  /** The strip word: "Flowsheet", "Calculations", "Safety", "Drawing". */
  readonly short: string;
  /** The noun a sentence uses: "case", "calculation book", "safety study", "drawing". */
  readonly noun: string;
  /** A document the reader edits and saves, or a read-only viewer. */
  readonly kind: "document" | "viewer";
  /** The wire route the surface rides; the flowsheet's served route is `/rpc`. */
  readonly route: string | null;
  /** What Save writes; `null` when the surface saves no document of its own. */
  readonly saveFormat: string | null;
  /** The flowsheet is always mounted; the others mount on first choice and stay. */
  readonly mount: "always" | "first-visit";
  /** The menu the surface's own document commands belong to; every first menu is Case. */
  readonly menu: MenuId;
}

/** The interfaces, in Case-menu order. */
export const INTERFACES = [
  {
    id: "flowsheet",
    label: "Process simulation",
    short: "Flowsheet",
    noun: "case",
    kind: "document",
    route: "/rpc",
    saveFormat: "TOML",
    mount: "always",
    menu: "case",
  },
  {
    id: "calculations",
    label: "Calculation",
    short: "Calculations",
    noun: "calculation book",
    kind: "document",
    route: "/sheets",
    saveFormat: "TOML",
    mount: "first-visit",
    menu: "case",
  },
  {
    id: "safety",
    label: "Safety study",
    short: "Safety",
    noun: "safety study",
    kind: "document",
    route: "/safety",
    saveFormat: "TOML",
    mount: "first-visit",
    menu: "case",
  },
  {
    id: "drawing",
    label: "Drawing",
    short: "Drawing",
    noun: "drawing",
    kind: "viewer",
    route: "/dexpi",
    saveFormat: null,
    mount: "first-visit",
    menu: "case",
  },
] as const satisfies readonly InterfaceSpec[];

/** The id union is derived from the rows, never written twice. */
export type InterfaceId = (typeof INTERFACES)[number]["id"];

/** The row for an id. The throw is for a caller outside the union, never a menu choice. */
export function interfaceOf(id: InterfaceId): InterfaceSpec {
  const found = INTERFACES.find((spec) => spec.id === id);
  if (found === undefined) {
    throw new Error(`no interface registered for \`${id}\``);
  }
  return found;
}

/**
 * One Case-menu command per interface, the one in front marked current.
 *
 * **Flat, not a submenu.** The menu bar supports one nested level and `Demo cases…` already
 * uses it; a second nesting would make the choice two clicks without making it clearer.
 */
export function interfaceCommands({
  active,
  onChoose,
}: {
  active: InterfaceId;
  onChoose: (id: InterfaceId) => void;
}): CommandSpec[] {
  return INTERFACES.map((spec) => ({
    id: `case-${spec.id}`,
    label: spec.label,
    menu: spec.menu,
    run: () => onChoose(spec.id),
    enabled: true,
    current: spec.id === active,
    keywords: `switch interface ${spec.short} ${spec.id} ${spec.noun}`,
    hint:
      spec.kind === "viewer"
        ? `the read-only ${spec.noun}`
        : `the ${spec.noun}${spec.saveFormat === null ? "" : ` · ${spec.saveFormat}`}`,
  }));
}

/**
 * The Case menu: the interfaces first, a rule, then this case's own commands.
 *
 * **The picker is the first thing.** A reader who opens Case is asking which case to work on;
 * New/Open/Save sit below the rule because they act on the case already in front.
 */
export function caseMenuCommands(
  documentCommands: readonly CommandSpec[],
  {
    active,
    onChoose,
  }: {
    active: InterfaceId;
    onChoose: (id: InterfaceId) => void;
  },
): CommandSpec[] {
  const kinds = interfaceCommands({ active, onChoose });
  if (documentCommands.length === 0) {
    return kinds;
  }
  return [
    ...kinds,
    ...documentCommands.map((command, index) =>
      index === 0 ? { ...command, divider: true } : command,
    ),
  ];
}
