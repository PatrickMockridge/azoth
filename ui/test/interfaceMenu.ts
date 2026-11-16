/**
 * Choosing an interface the way a reader does: open the visible Case menu, pick a row.
 *
 * Hidden interfaces keep their menu bar in a `[hidden]` subtree, so the role queries find only
 * the strip in front; a helper that selected `.menu-trigger` would find every mounted strip.
 */

import { fireEvent, screen } from "@testing-library/react";

/** Choose an interface from the visible Case menu. */
export function chooseInterface(label: string): void {
  fireEvent.click(screen.getByRole("button", { name: "Case" }));
  fireEvent.click(screen.getByRole("menuitem", { name: label }));
}

/** The label the visible Case menu marks current, or `null`; closes the menu afterwards. */
export function currentInterface(): string | null {
  fireEvent.click(screen.getByRole("button", { name: "Case" }));
  const current = screen
    .getAllByRole("menuitem")
    .find((item) => item.getAttribute("aria-current") === "true");
  const label = current?.querySelector("span")?.textContent ?? null;
  fireEvent.keyDown(window, { key: "Escape" });
  return label;
}
