// The wasm module, driven the way a browser drives it.
//
//     wasm-pack build crates/azoth-wasm --target nodejs --out-dir pkg --release \
//         --no-pack --no-typescript
//     node crates/azoth-wasm/test/smoke.mjs crates/azoth-wasm/pkg/azoth_wasm.js
//
// **What this covers that the Rust tests cannot.** Every function in the crate is a call into the
// wire layer and a string out, so what is untested there is the *boundary*: that a wasm-bindgen
// constructor works, that a `Result` becomes a JS exception rather than a trap, and that the
// module carries its own palette with no filesystem and no fetch. This is that, and nothing else.
//
// It reads the shipped document, so it is run from the repository root.

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const modulePath = process.argv[2];
if (!modulePath) {
  console.error("usage: node smoke.mjs <path to azoth_wasm.js>");
  process.exit(2);
}
const azoth = await import(pathToFileURL(modulePath).href);

const failures = [];
const check = (what, condition, detail = "") => {
  if (!condition) failures.push(`${what}${detail ? `: ${detail}` : ""}`);
};

// 1. The palette the module carries, with no filesystem anywhere in sight.
const catalogue = JSON.parse(azoth.palette_json(false));
check("31 palette entries", catalogue.unit_ops.length === 31, `${catalogue.unit_ops.length}`);
// **Two censuses, and both print what they found rather than only what they wanted.**
// `unit_ops.simple_absorber` is the one entry with neither a model nor a kernel, so both counts
// sit one short of the entry count. A census has to move when the palette grows; a failure that
// named only the expected number is how these two went stale unnoticed.
const withModel = catalogue.unit_ops.filter((entry) => entry.model !== null).length;
const runnable = catalogue.unit_ops.filter((entry) => entry.runnable).length;
check("30 with a model", withModel === 30, `${withModel}`);
check("30 runnable", runnable === 30, `${runnable}`);
const withTools = JSON.parse(azoth.palette_json(true));
check("20 tools", withTools.tools.length === 20, `${withTools.tools.length}`);
check("98 calculations", catalogue.calcs.length === 98, `${catalogue.calcs.length}`);
const available = catalogue.calcs.filter((entry) => entry.available).length;
check("94 a sheet can draw", available === 94, `${available}`);
check("82 safety calculations", catalogue.safety.length === 82, `${catalogue.safety.length}`);
const nativeSafety = catalogue.safety.filter((entry) => entry.native).length;
check("32 native safety ids", nativeSafety === 32, `${nativeSafety}`);

// 2. Opening the shipped document.
const document = readFileSync("specs/flowsheets/demo.toml", "utf8");
const editor = new azoth.Editor(document);
check("a fresh document is dirty", editor.dirty === true);
check("and sound", editor.ok === true);

// 3. A command, and the envelope it answers with.
const after = JSON.parse(
  editor.apply('{"command":"set_position","node":"instance:sep1","x":1234.0,"y":56.0}'),
);
check("the envelope says OK", after.ok === true, JSON.stringify(after.diagnostics));
check("6 nodes", after.flowsheet.graph.nodes.length === 6);
check("6 edges", after.flowsheet.graph.edges.length === 6);
const sep1 = after.flowsheet.graph.nodes.find((node) => node.id === "instance:sep1");
check("the position landed", sep1.position.x === 1234.0 && sep1.position.y === 56.0);
check("the edit is in the document", after.flowsheet.document.includes("[layout.instances]"));
check("and the values are stale", after.dirty === true);
check("with no session", after.session === null);

// 4. A run, and a value by path.
const ran = JSON.parse(editor.run());
check("it converged", ran.session.converged === true);
check("in two passes", ran.session.iterations === 2, `${ran.session.iterations}`);
check("with the tear", ran.session.tears[0].stream === "recycle_1");
check("and 43 values", ran.paths.length === 43, `${ran.paths.length}`);
check("a value by path", editor.value("p1.outlet.P") === 2000000.0);
check("the values are current now", editor.dirty === false);

// 5. A command that is not one, and a document that is not one: exceptions, not traps.
let refused = false;
try {
  editor.apply('{"command":"delete_everything"}');
} catch (error) {
  refused = String(error).includes("delete_everything");
}
check("an unknown command raises with the name in it", refused);

refused = false;
try {
  new azoth.Editor("this is not TOML");
} catch (error) {
  refused = String(error).length > 0;
}
check("a document that does not read raises", refused);

// 6. The execution order, which is a property of the session and not of a widget.
const reordered = JSON.parse(editor.set_order("topological"));
check("the order is in the envelope", reordered.execution_order === "topological");
check("and setting it is not a document change", reordered.dirty === false);
check("the document is untouched", reordered.flowsheet.document.includes("flowsheets.demo"));

let refusedOrder = false;
try {
  editor.set_order("kahn");
} catch (error) {
  refusedOrder = String(error).includes("kahn") && String(error).includes("insertion");
}
check("a name that is neither order raises with both in it", refusedOrder);

// 7. The calculation book, which is a second document on the same module.
const book = readFileSync("specs/calcbooks/demo.toml", "utf8");
const sheets = new azoth.SheetEditor(book);
const opened = JSON.parse(sheets.envelope());
check("two sheets", opened.book.sheets.length === 2, `${opened.book.sheets.length}`);
const calculated = JSON.parse(sheets.calculate("pump_power_1"));
check("the worked example ran", calculated.book.sheets[0].stale === false);
check(
  "and the spec's answer is on the wire",
  Math.abs(calculated.book.sheets[0].result.outputs.power.magnitude_si - 3914.81468) < 1e-6,
);
const added = JSON.parse(
  sheets.apply(
    '{"command":"add_sheet","id":"reynolds_1","calculation":"hydraulics.reynolds_number"}',
  ),
);
check("a sheet is added", added.book.sheets.length === 3, `${added.book.sheets.length}`);
check("its inputs are empty", Object.keys(added.book.sheets[2].inputs).length === 0);

// 8. The safety study, a third document on the same module.
const study = readFileSync("specs/safety/demo.toml", "utf8");
const safety = new azoth.SafetyEditor(study);
const openedStudy = JSON.parse(safety.envelope());
check("six safety scenarios", openedStudy.study.scenarios.length === 6);
check("and one bow-tie", openedStudy.study.bowties.length === 1);
const ranStudy = JSON.parse(safety.calculate(undefined));
check("the jet fire ran", ranStudy.study.scenarios[0].stale === false);
const studyFlux = ranStudy.study.scenarios[0].result.outputs.incident_heat_flux.magnitude_si;
check("with the spec's flux", Math.abs(studyFlux - 2119.417169158887) < 1e-6, `${studyFlux}`);
const studySif = ranStudy.study.scenarios.find((row) => row.id === "sif_1");
check(
  "and the SIF's RRF",
  Math.abs(studySif.result.outputs.risk_reduction_factor - 200) < 1e-9,
  `${studySif.result.outputs.risk_reduction_factor}`,
);
const studyRrf = ranStudy.study.bowties[0].derived.total_rrf;
check("the bow-tie has RRF 100", Math.abs(studyRrf - 100) < 1e-9, `${studyRrf}`);
const moved = JSON.parse(
  safety.apply(
    '{"command":"set_bowtie_position","bowtie":"bt_separator_release","node":"top_event","x":500,"y":130}',
  ),
);
check("a position lands in the document", moved.study.document.includes("500"));
check("and does not stale the result", moved.dirty === false);

// 9. And the module is still alive after every refusal - a trap would have taken it with them.
check("the module survives its refusals", editor.value("p1.outlet.T") > 300.0);

if (failures.length > 0) {
  console.error(`wasm smoke: ${failures.length} failure(s)`);
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log(
  "wasm smoke: OK (31 palette entries, 16 tools, 98 calculations, 82 safety ids, 32 native, 6 nodes, 43 values)",
);
