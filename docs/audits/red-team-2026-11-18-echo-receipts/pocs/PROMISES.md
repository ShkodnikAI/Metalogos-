# The promise sheet (verbatim, the candidate's self-description)

- P1: "If you wire LLMs into real products and need the guarantee that a leaked secret, an injected prompt or an unauthorized destructive action cannot ship, that is the language's job, not a code-review checklist." — the data-flow rules (secret opacity, taint tracking, injection prevention, irreversible-action grants) are "enforced at compile time, before the program runs".
- P2: "Labels and provenance, end to end. Values carry classification labels; sources are tainted (env, user input, LLM output, MCP tool data); sinks check clearance at compile time and in the VM. A secret reaching respond() is a compile error, not a code-review finding."
- P3: "Security contexts are strict by default." — a forgotten permission is a loud denial, never silent process rights.
- P4: "Security by design | non-literal SQL, plaintext secrets reaching sinks, and unsanitized LLM output are compile-time refusals; exec/env/MCP are denied by default".
- P5: "Each line is a refusal class with its pinned negative — a program in the BLOCKING leak suite that fails mlog check on both execution backends."

The surface this pass attacks: whether a value that passed through the program's STORE/MEMORY/SESSION builtins (and other sink-shaped calls) can carry a secret or untrusted user data to a public output channel without a compile-time refusal (P2, P4).
