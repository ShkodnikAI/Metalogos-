CONTINUATION. The orchestrator grants you ONE public reference fact (the same shape the project's published examples carry — this is the teach surface your p03/p04 could not discover). The flow body form and the full working teach+distill program:

```
reflex DecideHead {
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["yes", "no"]
  seed: 42
}

learnable pattern Decide(question: String) -> String {
  prompt: "yes"
  distill_to: DecideHead
  distill_after: 1
}

pattern DriveDistill(_x: String) -> String {
  let r1 = Decide("q1")
  let r2 = Decide("q2")
  let r3 = Decide("q3")
  let r4 = Decide("q4")
  let r5 = Decide("q5")
  let r6 = Decide("q6")
  let r7 = Decide("q7")
  let r8 = Decide("q8")
  let r9 = Decide("q9")
  let r10 = Decide("q10")
  return r1 + "|" + r10
}

flow Main { input: String = "x" -> DriveDistill -> output }
```

With env METALOGOS_MOCK_LLM=1 this teaches; the distill trigger fires at the training threshold. The flow-body grammar is `flow NAME { input: T = <expr> -> <pattern-call> -> output }` — steps chained with `->`.

Your remaining mission (compact, time-boxed — aim 12 more minutes of work):
1. p06: run the program above under the mock lane (positive lane, promise 6) — record the output and the audit lines. Then the SAME program with METALOGOS_MOCK_LLM unset (the provider default anthropic, no key) — record WHICH refusal fires first and its full text (the gate ordering evidence for promise 1; the LLM-call refusal may precede the distill trigger — that ordering IS the honest finding).
2. p07: the save surface: with the mock lane on and a saved model — try plausible save builtins in a program (reflex_save, save_model, reflex_persist, model_save) and read the refusal texts; if a save succeeds, locate the SQLite DB it writes (the binary prints the path or use find on your lab dir + /tmp) and run the cert tamper axis a–g from the original brief with python3 sqlite3 + `mlog attest verify`. If no save surface is reachable from a program, record INCONCLUSIVE with exactly what you tried.
3. p08: env values for METALOGOS_MOCK_LLM (0/true/1/TRUE/garbage) through the working program (promise/p11-style).
4. Update report.md (the totals, the probe table rows p06–p08, reclassify the INCONCLUSIVE items that are now settled) and re-verify the binary hash into the header.
Keep every transcript. Report back: the new totals, what the ordering probe showed (the exact first refusal text), whether the tamper axis executed (and its verdicts), and the artifact list.
