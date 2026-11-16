/**
 * The interface registry, and the Case-menu commands built from it.
 *
 * The menu is the model's only reader: if a row loses its label, route or mount policy, or two
 * rows claim to be current, this is where it is caught.
 */

import { describe, expect, it, vi } from "vitest";

import { standardMenus, type CommandSpec } from "../src/commands/model";
import {
  INTERFACES,
  caseMenuCommands,
  interfaceCommands,
  interfaceOf,
} from "../src/interfaces";

describe("the interface model", () => {
  it("names each interface once, in Case-menu order", () => {
    expect(INTERFACES.map((spec) => spec.id)).toEqual([
      "flowsheet",
      "calculations",
      "safety",
      "drawing",
    ]);
    expect(new Set(INTERFACES.map((spec) => spec.label)).size).toBe(INTERFACES.length);
    expect(new Set(INTERFACES.map((spec) => spec.noun)).size).toBe(INTERFACES.length);
    for (const spec of INTERFACES) {
      expect(spec.label.trim()).not.toBe("");
      expect(spec.route).not.toBe("");
      expect(spec.menu).toBe("case");
    }
  });

  it("keeps the flowsheet mounted and the rest first-visit", () => {
    const always = INTERFACES.filter((spec) => spec.mount === "always");
    expect(always.map((spec) => spec.id)).toEqual(["flowsheet"]);
    expect(INTERFACES.filter((spec) => spec.mount === "first-visit")).toHaveLength(3);
  });

  it("gives the documents a save format and the viewer none", () => {
    for (const spec of INTERFACES) {
      if (spec.kind === "viewer") {
        expect(spec.saveFormat).toBeNull();
      } else {
        expect(spec.saveFormat).toBe("TOML");
      }
    }
    expect(interfaceOf("drawing").kind).toBe("viewer");
    expect(interfaceOf("safety").noun).toBe("safety study");
  });
});

describe("the Case-menu commands", () => {
  it("offers every interface once and marks exactly the one in front", () => {
    const choose = vi.fn();
    const commands = interfaceCommands({ active: "calculations", onChoose: choose });

    expect(commands).toHaveLength(INTERFACES.length);
    expect(commands.map((command) => command.id)).toEqual(
      INTERFACES.map((spec) => `case-${spec.id}`),
    );
    expect(commands.every((command) => command.menu === "case" && command.enabled)).toBe(true);
    const current = commands.filter((command) => command.current === true);
    expect(current).toHaveLength(1);
    expect(current[0]?.label).toBe("Calculation");

    commands.find((command) => command.id === "case-drawing")?.run();
    expect(choose).toHaveBeenCalledWith("drawing");
  });

  it("puts the interface options before the case's own commands, after a rule", () => {
    const documentCommands: CommandSpec[] = [
      { id: "new", label: "New", menu: "case", run: () => {}, enabled: true },
      { id: "save", label: "Save…", menu: "case", run: () => {}, enabled: true },
    ];
    const commands = caseMenuCommands(documentCommands, {
      active: "flowsheet",
      onChoose: () => {},
    });

    expect(commands).toHaveLength(INTERFACES.length + documentCommands.length);
    expect(
      commands.slice(0, INTERFACES.length).every((command) => command.id.startsWith("case-")),
    ).toBe(true);
    const firstDocument = commands[INTERFACES.length];
    expect(firstDocument?.id).toBe("new");
    expect(firstDocument?.divider).toBe(true);
    expect(commands.at(-1)?.divider).toBeUndefined();
  });

  it("leaves a menu without a rule when the interface has no commands of its own", () => {
    const commands = caseMenuCommands([], { active: "drawing", onChoose: () => {} });
    expect(commands).toHaveLength(INTERFACES.length);
    expect(commands.some((command) => command.divider === true)).toBe(false);
  });

  it("drops an empty menu from the standard bar", () => {
    const menus = standardMenus([
      { id: "new", label: "New", menu: "case", run: () => {}, enabled: true },
    ]);
    expect(menus.map((menu) => menu.id)).toEqual(["case"]);
    expect(menus[0]?.label).toBe("Case");
  });
});
