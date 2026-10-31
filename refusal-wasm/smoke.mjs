#!/usr/bin/env node
// №855 (issue #1680, wave 74 — W-C): the smoke consumer of the
// Portable Refusal Algebra's WASM facade — runs from the HOST with
// node only (no Rust toolchain in the consumption path). The CI
// `wasm-facade (blocking)` job runs this against the freshly built
// artifact on every push. The consumer logic lives in consume.mjs
// (the same module the №856 policy-check Action drives).
//
// usage: node refusal-wasm/smoke.mjs <path-to-refusal_wasm.wasm>
import process from 'node:process';
import { loadFacade } from './consume.mjs';

const [, , wasmPath] = process.argv;
if (!wasmPath) {
  console.error('usage: node refusal-wasm/smoke.mjs <refusal_wasm.wasm>');
  process.exit(2);
}

const evalRequest = await loadFacade(wasmPath);

const cases = [
  // join — the MORE SEVERE cause dominates (SINK_CLEARANCE rank 10 < SECRET_LEAK rank 13)
  [{ op: 'join', reasons: ['SECRET_LEAK', 'SINK_CLEARANCE'] },
    (r) => r.verdict === 'deny' && r.code === 'SINK_CLEARANCE' && r.cause === 'SINK_CLEARANCE'],
  // meet — the LESS SEVERE cause survives
  [{ op: 'meet', reasons: ['SECRET_LEAK', 'SINK_CLEARANCE'] },
    (r) => r.verdict === 'deny' && r.code === 'SECRET_LEAK'],
  // empty reasons — Allow is the bottom element
  [{ op: 'verdict', reasons: [] }, (r) => r.verdict === 'allow'],
  // the core class carries the frozen code verbatim
  [{ op: 'verdict', reasons: ['IRREVERSIBLE_NO_GRANT'] },
    (r) => r.verdict === 'deny' && r.code === 'IRREVERSIBLE_NO_GRANT'],
  // the closed-set rule: an unknown word is a loud error, never a silent pass
  [{ op: 'join', reasons: ['NOT_A_REASON'] }, (r) => r.verdict === 'error'],
];

let fails = 0;
for (const [req, ok] of cases) {
  const r = await evalRequest(req);
  if (ok(r)) {
    console.log(`smoke ok: ${JSON.stringify(req)} -> ${JSON.stringify(r)}`);
  } else {
    fails += 1;
    console.error(`SMOKE FAIL: ${JSON.stringify(req)} -> ${JSON.stringify(r)}`);
  }
}
if (fails) {
  console.error(`smoke: ${fails} case(s) failed`);
  process.exit(1);
}
console.log('smoke: ALL PASS — the algebra answers from the host through the wasm boundary');
