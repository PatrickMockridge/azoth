// @vitest-environment jsdom
/**
 * The control layer's two sheets: the instrument's record and the signal's carrier.
 *
 * **`set_instrument` replaces the whole record**, so every edit must carry the fields it did not
 * touch — a service edit that dropped the range would be a sheet that lost the drawing's data one
 * field at a time. These tests read the command the sheet sends, which is the only thing it does.
 */

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { InstrumentSheet } from "../src/components/property/InstrumentSheet";
import { SignalSheet } from "../src/components/property/SignalSheet";
import type { GraphEdge, GraphNode, Instrument } from "../src/wire/types";

afterEach(cleanup);

const instrument: Instrument = {
  id: "PSHH-101",
  function: "PSHH",
  service: "separator overhead pressure",
  measures: "sep1.vapour",
  location: "field",
  range: { min: 0, max: 10, unit: "bar" },
};

const node: GraphNode = {
  id: "instrument:PSHH-101",
  type: "instrument",
  role: "instrument",
  position: { x: 0, y: 0 },
  data: { name: "PSHH-101", instrument },
};

const signal: GraphEdge = {
  id: "e3",
  source: "instrument:PSHH-101",
  sourceHandle: "out",
  target: "instrument:TIC-102",
  targetHandle: "in",
  data: {
    kind: "signal",
    from: "PSHH-101",
    to: "TIC-102",
    path: "PSHH-101->TIC-102",
    signal_kind: "electric",
  },
};

describe("the instrument sheet", () => {
  it("sends the whole record when one field changes", () => {
    const onCommand = vi.fn();
    render(<InstrumentSheet node={node} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("service"), {
      target: { value: "separator pressure high-high" },
    });
    fireEvent.blur(screen.getByLabelText("service"));

    expect(onCommand).toHaveBeenCalledOnce();
    expect(onCommand).toHaveBeenCalledWith({
      command: "set_instrument",
      id: "PSHH-101",
      function: "PSHH",
      service: "separator pressure high-high",
      measures: "sep1.vapour",
      location: "field",
      range: { min: 0, max: 10, unit: "bar" },
    });
  });

  it("clears an optional field by sending it absent, not empty", () => {
    const onCommand = vi.fn();
    render(<InstrumentSheet node={node} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("service"), { target: { value: "" } });
    fireEvent.blur(screen.getByLabelText("service"));

    const command = onCommand.mock.calls[0]?.[0] as Record<string, unknown>;
    // A present-but-empty string would be a word the drawing now says; the field is unstated.
    expect(command).not.toHaveProperty("service");
    expect(command).toHaveProperty("function", "PSHH");
  });

  it("refuses to empty the one required field", () => {
    const onCommand = vi.fn();
    render(<InstrumentSheet node={node} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("function"), { target: { value: "" } });
    fireEvent.blur(screen.getByLabelText("function"));

    expect(onCommand).not.toHaveBeenCalled();
  });

  it("changes the location through the three names DEXPI keeps", () => {
    const onCommand = vi.fn();
    render(<InstrumentSheet node={node} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("location"), { target: { value: "central" } });

    expect(onCommand).toHaveBeenCalledWith(
      expect.objectContaining({ command: "set_instrument", location: "central" }),
    );
  });

  it("clears a range whole, because a range is a record and not a field", () => {
    const onCommand = vi.fn();
    render(<InstrumentSheet node={node} onCommand={onCommand} />);

    fireEvent.click(screen.getByRole("button", { name: "Clear range" }));

    const command = onCommand.mock.calls[0]?.[0] as Record<string, unknown>;
    expect(command).not.toHaveProperty("range");
    expect(command).toHaveProperty("function", "PSHH");
  });
});

describe("the signal sheet", () => {
  it("shows the endpoints and the carrier the edge carries", () => {
    render(<SignalSheet edge={signal} onCommand={vi.fn()} />);

    expect(screen.getByLabelText("from")).toHaveProperty("value", "PSHH-101");
    expect(screen.getByLabelText("to")).toHaveProperty("value", "TIC-102");
    expect(screen.getByLabelText("carrier")).toHaveProperty("value", "electric");
  });

  it("sends the carrier as the signal's own kind", () => {
    const onCommand = vi.fn();
    render(<SignalSheet edge={signal} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("carrier"), { target: { value: "pneumatic" } });

    expect(onCommand).toHaveBeenCalledWith({
      command: "set_signal",
      from: "PSHH-101",
      to: "TIC-102",
      kind: "pneumatic",
    });
  });

  it("leaves the carrier unstated rather than writing an empty name", () => {
    const onCommand = vi.fn();
    render(<SignalSheet edge={signal} onCommand={onCommand} />);

    fireEvent.change(screen.getByLabelText("carrier"), { target: { value: "" } });

    expect(onCommand).toHaveBeenCalledWith({
      command: "set_signal",
      from: "PSHH-101",
      to: "TIC-102",
    });
  });
});
