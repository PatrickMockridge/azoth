/**
 * The bow-tie as a canvas.
 *
 * **The nodes are the envelope's projection, and a drag is a document edit.** Nothing here keeps
 * a copy of the diagram: the derivation is a function of the envelope, and a dropped plate sends
 * a position command whose answer is the next envelope.
 */

import { useEffect, useMemo } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Edge,
} from "@xyflow/react";

import type { SafetyBowtieView } from "../../wire/types";
import { BOWTIE_NODE_TYPES, type BowtieFlowNode } from "./BowtieNode";

export function BowtieCanvas({
  bowtie,
  selected,
  onSelect,
  onMove,
}: {
  bowtie: SafetyBowtieView;
  selected: string | null;
  onSelect: (id: string | null) => void;
  onMove: (node: string, x: number, y: number) => void;
}) {
  const derived = useMemo(
    () => ({
      nodes: bowtie.graph.nodes.map(
        (node): BowtieFlowNode => ({
          id: node.id,
          type: node.type,
          position: node.position,
          data: node.data,
          selected: node.id === selected,
        }),
      ),
      edges: bowtie.graph.edges.map(
        (edge): Edge => ({ id: edge.id, source: edge.source, target: edge.target }),
      ),
    }),
    [bowtie, selected],
  );
  const [nodes, setNodes, onNodesChange] = useNodesState<BowtieFlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  useEffect(() => {
    setNodes(derived.nodes);
    setEdges(derived.edges);
  }, [derived, setNodes, setEdges]);

  return (
    <ReactFlow
      key={bowtie.id}
      nodes={nodes}
      edges={edges}
      nodeTypes={BOWTIE_NODE_TYPES}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onNodeClick={(_event, node) => onSelect(node.id)}
      onPaneClick={() => onSelect(null)}
      onNodeDragStop={(_event, node) =>
        onMove(node.id, Math.round(node.position.x), Math.round(node.position.y))
      }
      nodesConnectable={false}
      fitView
      proOptions={{ hideAttribution: true }}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
