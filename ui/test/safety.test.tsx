// @vitest-environment jsdom
/**
 * The Safety workspace, driven through the real wasm module and the served door.
 *
 * **The same discipline as `app.test.tsx` and `calculations.test.tsx`.** `fetch` serves the
 * module this build produced, the demo study is the shipped `specs/safety/demo.toml`, and the
 * assertions are about the DOM: the bow-tie projection, the RRF the model computed, the
 * distance-to-threshold table and a field edit that makes the answer out of date.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import catalogueJson from "./fixtures/catalogue.json";
import envelopeJson from "./fixtures/envelope.json";
import safetyJson from "./fixtures/safety-envelope.json";
import { chooseInterface } from "./interfaceMenu";
import { stubXyflowEnvironment } from "./xyflow-env";

beforeAll(stubXyflowEnvironment);

function stubFetch(): void {
  const wasm = readFileSync(resolve("src/wasm/pkg/azoth_wasm_bg.wasm"));
  globalThis.fetch = (input: RequestInfo | URL): Promise<Response> => {
    expect(String(input)).toContain("azoth_wasm_bg.wasm");
    return Promise.resolve(
      new Response(wasm, { headers: { "Content-Type": "application/wasm" } }),
    );
  };
}

async function openSafety(stored: Record<string, string> = {}): Promise<HTMLElement> {
  window.localStorage.setItem("azoth.welcome", "seen");
  window.localStorage.removeItem("azoth.units");
  window.localStorage.removeItem("azoth.appearance");
  window.localStorage.removeItem("azoth.theme");
  for (const [key, value] of Object.entries(stored)) {
    window.localStorage.setItem(key, value);
  }
  const { App } = await import("../src/App");
  const rendered = render(<App />);
  await waitFor(() => {
    expect(rendered.container.querySelectorAll(".react-flow__node")).not.toHaveLength(0);
  });
  chooseInterface("Safety study");
  await waitFor(() => {
    expect(screen.getByRole("heading", { name: "Bow-ties" })).toBeDefined();
  });
  const app = document.querySelector<HTMLElement>(".app.safety");
  if (app === null) {
    throw new Error("the safety workspace is mounted");
  }
  return app;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("the safety workspace", () => {
  it("opens the demo study and draws its overview", async () => {
    stubFetch();
    const app = await openSafety();
    expect(within(app).getByDisplayValue("Demo safety study")).toBeDefined();
    expect(within(app).getByText("Bow-ties in this study")).toBeDefined();
    expect(within(app).getByText("Scenarios in this study")).toBeDefined();
    const families = within(app).getAllByRole("heading", { name: /dispersion|firewater|rupture/i });
    expect(families.length).toBeGreaterThan(0);
    // **The card the hand list forgot.** Depressurization has a deck, and now a card too.
    expect(
      within(app).getByRole("heading", { name: "Depressurization scenarios" }),
    ).toBeDefined();
    // Every family is on the overview index, including the five no deck used to hide.
    for (const label of [
      "Emergency shutdown",
      "Safety envelope",
      "Inventory",
      "Leak detection",
      "Open drain",
    ]) {
      expect(within(app).getByRole("button", { name: new RegExp(label) })).toBeDefined();
    }
  });

  it("browses all 96 calculations in the catalogue", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Catalogue" }));
    await waitFor(() => {
      expect(app.querySelectorAll("[data-calculation-id]")).toHaveLength(96);
    });
    expect(app.querySelectorAll("[data-family-id]")).toHaveLength(23);
    for (const id of [
      "process.safety_envelope",
      "process.esd_response_time",
      "process.trapped_inventory",
      "process.mass_balance_leak_detection",
      "process.open_drain_review",
    ]) {
      expect(app.querySelector(`[data-calculation-id="${id}"]`)).not.toBeNull();
    }
    const jet = app.querySelector('[data-calculation-id="process.jet_fire"]');
    if (jet === null) {
      throw new Error("jet fire is in the catalogue");
    }
    expect(
      within(jet as HTMLElement)
        .getByRole("link", { name: "Jet fire radiation" })
        .getAttribute("href"),
    ).toMatch(/process\/jet_fire\.html$/);
    const dispersion = app.querySelector('[data-calculation-id="process.gas_dispersion"]');
    expect(dispersion?.querySelector(".refusal")?.textContent).toMatch(/fluid/);
  });

  it("narrows the catalogue by search", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Catalogue" }));
    const search = await waitFor(() =>
      within(app).getByRole("searchbox", { name: "Search the safety catalogue" }),
    );
    fireEvent.change(search, { target: { value: "relief" } });
    await waitFor(() => {
      const shown = app.querySelectorAll("[data-calculation-id]").length;
      expect(shown).toBeGreaterThan(0);
      expect(shown).toBeLessThan(96);
    });
    const tools = app.querySelector(".safety-catalogue-tools");
    if (tools === null) {
      throw new Error("the catalogue tools are drawn");
    }
    expect(within(tools as HTMLElement).getByRole("status").textContent).not.toMatch(/^96 of 96/);
    expect(app.querySelector('[data-calculation-id="process.blocked_outlet_relief"]')).not.toBeNull();
  });

  it("adds a scenario from the catalogue and opens its deck", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Catalogue" }));
    const add = await waitFor(() =>
      within(app).getByRole("button", { name: "Add Jet fire radiation as a scenario" }),
    );
    fireEvent.click(add);
    await waitFor(() => {
      expect(within(app).getAllByRole("tab")).toHaveLength(3);
    });
  });

  it("draws the bow-tie projection and computes the RRF", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Open the bow-tie view" }));
    await waitFor(() => {
      expect(app.querySelectorAll(".react-flow__node")).toHaveLength(7);
    });
    fireEvent.click(within(app).getByRole("button", { name: "Calculate" }));
    await waitFor(() => {
      expect(within(app).getByText("100")).toBeDefined();
    });
    expect(within(app).getByText(/0\.15[0-9]* \/year/)).toBeDefined();
    expect(within(app).getByText(/0\.0015[0-9]* \/year/)).toBeDefined();
  });

  it("calculates a fire scenario, then marks it out of date after an edit", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Fire" }));
    await waitFor(() => {
      expect(within(app).getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
        "HP separator jet fire",
        "Diesel spill pool fire",
      ]);
    });

    fireEvent.click(within(app).getByRole("button", { name: "Calculate" }));
    await waitFor(() => {
      expect(within(app).getByRole("heading", { name: "Results" })).toBeDefined();
    });
    expect(app.querySelectorAll(".safety-profile tbody tr")).toHaveLength(6);

    const distance = within(app).getByDisplayValue("20");
    fireEvent.change(distance, { target: { value: "80" } });
    fireEvent.blur(distance);
    await waitFor(() => {
      expect(within(app).getByText(/These are the last calculated values/)).toBeDefined();
    });

    fireEvent.click(within(app).getByRole("button", { name: "Calculate" }));
    await waitFor(() => {
      expect(within(app).queryByText(/These are the last calculated values/)).toBeNull();
    });
  });

  it("draws the dispersion deck from the same study, and calculates its scenario", async () => {
    stubFetch();
    const app = await openSafety();
    fireEvent.click(within(app).getByRole("button", { name: "Dispersion" }));
    // **The deck filters the study by family.** The study holds six scenarios; this view holds
    // the one dispersion scenario, which is what keeps the fire view's two tabs two.
    await waitFor(() => {
      expect(within(app).getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
        "Urban plume at 500 m",
      ]);
    });

    fireEvent.click(within(app).getByRole("button", { name: "Calculate" }));
    await waitFor(() => {
      expect(within(app).getByRole("heading", { name: "Results" })).toBeDefined();
    });
    expect(within(app).getByText("Concentration")).toBeDefined();
  });

  it("reads a served study from /safety", async () => {
    globalThis.fetch = (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
      const url = String(input);
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      const answer = url.endsWith("/safety")
        ? safetyJson
        : "catalogue" in body
          ? catalogueJson
          : envelopeJson;
      return Promise.resolve(
        new Response(JSON.stringify(answer), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    };

    history.replaceState({}, "", "/?serve=http://127.0.0.1:4000");
    const { App } = await import("../src/App");
    const rendered = render(<App />);
    await waitFor(() => {
      expect(rendered.container.querySelectorAll(".react-flow__node")).not.toHaveLength(0);
    });
    chooseInterface("Safety study");
    await waitFor(() => {
      expect(screen.getByDisplayValue("Demo safety study")).toBeDefined();
    });
    const app = document.querySelector<HTMLElement>(".app.safety");
    if (app === null) {
      throw new Error("the safety workspace is mounted");
    }
    expect(within(app).getByText("Bow-ties in this study")).toBeDefined();
  });
});
