/**
 * The safety study over the served door.
 *
 * **`/safety` is its own route.** The flowsheet's `/rpc` holds one document and the study is a
 * third; this test is that the browser asks the third route, with the body shape the route reads,
 * and decodes the study envelope it answers.
 */

import { afterEach, describe, expect, it, vi } from "vitest";

import { HttpSafetyDoor } from "../src/wire/http";
import safetyJson from "./fixtures/safety-envelope.json";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the safety study on a served door", () => {
  it("attaches with a null command and decodes the study envelope", async () => {
    const calls: { url: string; body: unknown }[] = [];
    vi.stubGlobal(
      "fetch",
      (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
        calls.push({ url: String(input), body: JSON.parse(String(init?.body)) as unknown });
        return Promise.resolve(
          new Response(JSON.stringify(safetyJson), {
            status: 200,
            headers: { "Content-Type": "application/json" },
          }),
        );
      },
    );

    const door = new HttpSafetyDoor("http://127.0.0.1:4000");
    const opened = await door.attach();
    expect(calls[0]?.url).toBe("http://127.0.0.1:4000/safety");
    expect(calls[0]?.body).toEqual({ command: null });
    expect(opened.envelope.study.scenarios).toHaveLength(6);
    expect(opened.envelope.study.bowties).toHaveLength(1);

    await opened.session.calculate({ scenario: "jet_fire_1" });
    expect(calls[1]?.body).toEqual({ calculate: { scenario: "jet_fire_1" } });

    await opened.session.apply({ command: "set_study_name", name: "Edited" });
    expect(calls[2]?.body).toEqual({
      command: { command: "set_study_name", name: "Edited" },
    });
  });
});
