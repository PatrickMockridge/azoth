// @vitest-environment jsdom
/**
 * The two new menu shapes: a current row and a rule.
 *
 * The accessible name of a current row must stay the label, because the menu is how every
 * interface is chosen and a check glyph in the name would make the choice unaddressable.
 */

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { MenuBar } from "../src/components/MenuBar";
import { standardMenus } from "../src/commands/model";

const menus = standardMenus([
  {
    id: "case-drawing",
    label: "Drawing",
    menu: "case",
    run: () => {},
    enabled: true,
    current: true,
  },
  { id: "new", label: "New", menu: "case", run: () => {}, enabled: true, divider: true },
]);

afterEach(cleanup);

describe("the menu bar's current row and rule", () => {
  it("marks the current row and keeps its accessible name", () => {
    render(<MenuBar menus={menus} />);
    fireEvent.click(screen.getByRole("button", { name: "Case" }));

    const current = screen.getByRole("menuitem", { name: "Drawing" });
    expect(current.getAttribute("aria-current")).toBe("true");
    expect(current.classList.contains("current")).toBe(true);
    expect(current.querySelector(".menu-check")?.getAttribute("aria-hidden")).toBe("true");

    const other = screen.getByRole("menuitem", { name: "New" });
    expect(other.getAttribute("aria-current")).toBeNull();
    expect(other.querySelector(".menu-check")).toBeNull();
  });

  it("draws the rule inside the menu, before the command that carries it", () => {
    render(<MenuBar menus={menus} />);
    fireEvent.click(screen.getByRole("button", { name: "Case" }));

    const menu = screen.getByRole("menu", { name: "Case" });
    const rule = menu.querySelector('[role="separator"]');
    expect(rule).not.toBeNull();
    expect(rule?.nextElementSibling?.textContent).toContain("New");
  });

  it("still closes on Escape and returns focus to the trigger", () => {
    render(<MenuBar menus={menus} />);
    const trigger = screen.getByRole("button", { name: "Case" });
    fireEvent.click(trigger);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
