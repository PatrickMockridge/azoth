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
  fireEvent.click(screen.getByRole("button", { name: "Safety" }));
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
    fireEvent.click(screen.getByRole("button", { name: "Safety" }));
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
