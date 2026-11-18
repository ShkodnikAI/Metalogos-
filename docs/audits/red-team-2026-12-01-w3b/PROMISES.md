# PROMISES (W3b — the teacher registry, №975 / ADR-0200 §9)

You hold ONLY the candidate binary (`mlog`) and this sheet. Attack each
promise by observation. Every claim below is what the machine is
supposed to do — try to make it do otherwise, offline, black-box.

1. **The gate.** A distillation training run against a teacher that has
   no teacher-registry record with a permissive (OSI) license class AND
   `training_on_output_allowed = true` refuses BEFORE the training
   executes. The refusal is loud, names the teacher and the class, and
   carries the check id `DISTILL_TEACHER_UNLICENSED`.
2. **Sink-parity.** The refusal holds on BOTH execution backends (the
   tree-walking interpreter and the bytecode VM) and on the
   serve/hub-attached lane.
3. **The registry is not program-reachable.** A .mlog program cannot
   read, enumerate, extend or influence the teacher registry: no
   builtin exposes it; no program-side state can add a record.
4. **The certificate fills honestly.** The attestation certificate's
   `teacher{...}` block appears ONLY when the model carries a distill
   lineage (a learnable pattern distilled into it) and the process's
   actual teacher has a registry record; the block carries exactly
   `backend`, `weights_id`, `license_class`, `training_on_output_allowed`.
   The no-teacher case (direct `reflex_train`) and the no-record case
   stay `null` WITH a sibling `teacher_reason` — nothing is invented.
5. **The verify check.** `attest_verify` (the builtin) and the CLI
   verify route refuse a certificate whose teacher block carries an
   unknown `license_class` word, an unknown backend, or a class/flag
   that disagrees with the registry record — loudly, with
   `[ATTESTATION_TEACHER_MISMATCH]`. An HONEST non-permissive teacher
   is NOT refused: verify stays `ok: true` (provenance consistency)
   but the reason line names the teacher and the class loudly. A
   permissive teacher verifies silently (no reason-append).
6. **The positive lane.** The in-tree mock teacher (the explicit
   `METALOGOS_MOCK_LLM=1` lane) is permissive: a program that teaches a
   learnable pattern under it distills end-to-end (the training runs,
   the pattern switches to the distilled model).
7. **The audit trail.** A gate refusal is audited and ledger-recorded
   (`distill.teacher-refused`).
