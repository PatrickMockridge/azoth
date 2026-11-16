/**
 * One plate on the bow-tie canvas.
 *
 * **The plate says what the arithmetic does.** A mitigation barrier is drawn because the class
 * records it, and it says so; a non-functional barrier says inactive; a threat says whether it
 * contributes. The diagram therefore cannot look like it computes something the model does not.
 */

import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";

import type { SafetyNodeData } from "../../wire/types";

export type BowtieFlowNode = Node<
  SafetyNodeData,
  "threat" | "barrier" | "top_event" | "consequence"
>;

const KIND: Record<BowtieFlowNode["type"], string> = {
  threat: "Threat",
  barrier: "Barrier",
  top_event: "Top event",
  consequence: "Consequence",
};

export function BowtieNode({ data, type, selected }: NodeProps<BowtieFlowNode>) {
  const inactive =
    (type === "threat" && data.active === false) ||
    (type === "barrier" && data.functional === false);
  return (
    <div
      className={`safety-node ${type}${selected === true ? " selected" : ""}${
        inactive ? " inactive" : ""
      }`}
    >
      <Handle type="target" position={Position.Left} isConnectable={false} />
      <Handle type="source" position={Position.Right} isConnectable={false} />
      <span className="safety-node-kind">{KIND[type]}</span>
      <strong>{data.label}</strong>
      {type === "threat" ? (
        <span className="safety-node-value">
          {data.frequency} /year{data.active === false ? " · inactive" : ""}
        </span>
      ) : null}
      {type === "barrier" ? (
        <>
          <span className="safety-node-value">
            PFD {data.pfd} · {data.barrier_type}
            {data.functional === false ? " · inactive" : ""}
          </span>
          {data.barrier_type === "mitigation" ? (
            <span className="safety-node-note">recorded; does not change the frequency</span>
          ) : null}
        </>
      ) : null}
      {type === "consequence" ? (
        <span className="safety-node-value">severity {data.severity}</span>
      ) : null}
    </div>
  );
}

export const BOWTIE_NODE_TYPES = {
  threat: BowtieNode,
  barrier: BowtieNode,
  top_event: BowtieNode,
  consequence: BowtieNode,
};
