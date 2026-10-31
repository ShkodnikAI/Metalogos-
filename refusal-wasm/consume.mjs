#!/usr/bin/env node
// №856 (issue #1681, wave 74 — W-C): the CONSUMER of the Portable
// Refusal Algebra's WASM facade — the shared host-side module the
// smoke test (smoke.mjs) and the reusable `policy-check` Action
// (action.yml) both drive. One consumer implementation, two consumers
// — the same JSON/C-ABI contract (refusal-wasm/src/lib.rs).
//
// CLI mode (the Action's eval step):
//   node refusal-wasm/consume.mjs <wasm> <policy.json> [--summary] [--outputs]
//
// The policy file: {"requests":[{"name"?, "op":"join"|"meet"|"verdict",
//   "reasons":["WORD",...], "expect"?: "deny"|"allow"|"error",
//   "expect_code"?: "WORD"}]}
// — a request with an `expect` field is CHECKED (a mismatch exits 1);
// a request without one is reported only. The aggregate verdict: deny
// if ANY request denies; the aggregate code = the join (the MOST
// severe cause, ADR-0204 §2) across the denying requests, computed BY
// THE ALGEBRA ITSELF (one aggregate join request through the facade —
// the runner never ranks words locally).
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const enc = new TextEncoder();
const dec = new TextDecoder();

/** Load a facade module and return an async request evaluator. */
export async function loadFacade(wasmPath) {
  const bytes = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  return async function evalRequest(req) {
    const raw = enc.encode(JSON.stringify(req));
    if (raw.length > e.rw_pool_len()) throw new Error('request exceeds the input pool');
    const addr = e.rw_alloc(raw.length);
    if (addr === 0xffffffff) throw new Error('rw_alloc refused the request');
    new Uint8Array(e.memory.buffer).set(raw, addr);
    const packed = e.rw_eval(addr, raw.length);
    const ptr = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const out = dec.decode(new Uint8Array(e.memory.buffer, ptr, len));
    e.rw_reset();
    return JSON.parse(out);
  };
}

/** Run the policy: evaluate every request, check the expects, compute
 * the aggregate verdict/code. Returns { rows, verdict, code, fails }. */
export async function runPolicy(evalRequest, policy) {
  const requests = policy.requests || [];
  if (requests.length === 0) throw new Error('the policy carries no requests');
  const rows = [];
  const denyCodes = [];
  for (const req of requests) {
    const r = await evalRequest({ op: req.op, reasons: req.reasons || [] });
    let ok = true;
    if (req.expect && r.verdict !== req.expect) ok = false;
    if (req.expect_code && r.code !== req.expect_code) ok = false;
    if (r.verdict === 'deny') denyCodes.push(r.code);
    rows.push({ name: req.name || `${req.op}(${(req.reasons || []).join(',')})`, req, r, ok });
  }
  let verdict = 'allow';
  let code = null;
  if (denyCodes.length > 0) {
    // The aggregate code is the ALGEBRA's join over the deny causes —
    // never a local ranking in the runner.
    const agg = await evalRequest({ op: 'join', reasons: denyCodes });
    if (agg.verdict !== 'deny') throw new Error('the aggregate join refused to deny');
    verdict = 'deny';
    code = agg.code;
  }
  const fails = rows.filter((row) => !row.ok);
  return { rows, verdict, code, fails };
}

function summaryText(rows, verdict, code) {
  const lines = [
    '### policy-check — the Portable Refusal Algebra (ADR-0204)',
    '',
    `**Aggregate verdict: \`${verdict}\`` + (code ? ` (code \`${code}\`)` : '') + '**',
    '',
    '| request | verdict | code | expect | result |',
    '|---|---|---|---|---|',
  ];
  for (const { name, req, r, ok } of rows) {
    const expect = [req.expect, req.expect_code].filter(Boolean).join(' / ') || '—';
    lines.push(
      `| ${name} | ${r.verdict} | ${r.code || '—'} | ${expect} | ${ok ? '✅' : '❌'} |`,
    );
  }
  lines.push('');
  return lines.join('\n');
}

async function main() {
  const args = process.argv.slice(2);
  const [wasmPath, policyPath] = args.filter((a) => !a.startsWith('--'));
  if (!wasmPath || !policyPath) {
    console.error('usage: node consume.mjs <wasm> <policy.json> [--summary] [--outputs]');
    process.exit(2);
  }
  const evalRequest = await loadFacade(wasmPath);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  const { rows, verdict, code, fails } = await runPolicy(evalRequest, policy);
  for (const { name, r, ok } of rows) {
    const mark = ok ? 'ok' : 'MISMATCH';
    console.log(`policy ${mark}: ${name} -> ${r.verdict}${r.code ? ` (${r.code})` : ''}`);
  }
  console.log(`policy aggregate: ${verdict}${code ? ` (${code})` : ''}`);
  if (args.includes('--summary') && process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summaryText(rows, verdict, code));
  }
  if (args.includes('--outputs') && process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT, `verdict=${verdict}\ncode=${code || ''}\n`);
  }
  process.exit(fails.length ? 1 : 0);
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) await main();
