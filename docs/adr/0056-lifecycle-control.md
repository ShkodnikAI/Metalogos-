# ADR-0056: Lifecycle Control — checkpoint/resume для долгих задач

**Status:** Implemented
**Date:** 2026-06-11

## Context

Metalogos flows execute their pipeline steps sequentially. For long-running analysis pipelines (e.g., data gathering → multi-step AI analysis → report generation in Fosved Office), the entire flow must complete in a single process invocation. If the process is killed, crashes, or the server restarts mid-flow, all intermediate results are lost and the flow must restart from the beginning. This is wasteful for expensive operations like LLM calls or large data processing steps.

Prior art: OpenHands lifecycle management, Temporal.io workflow checkpoints, Apache Airflow checkpointing, Prefect task persistence.

## Decision

Introduce `checkpoint("name")` as a pipeline marker in flow declarations. When a flow step completes and a checkpoint follows, the interpreter serializes the current pipeline state to persistent storage (SQLite or in-memory fallback). A new CLI command `mlog resume` reloads the state and continues execution from the step after the checkpoint.

### Syntax

```mlog
flow LongAnalysis {
    input: String = source_data
    -> GatherSources -> checkpoint("sources")
    -> Analyze -> checkpoint("analyzed")
    -> WriteReport -> output
}
```

The `checkpoint("name")` marker is a pipeline element (not a step). It appears between steps via the standard `->` arrow. Multiple checkpoints are allowed in a single flow.

### Grammar Changes

- New rule: `checkpoint_call = { "checkpoint" ~ "(" ~ STRING_LITERAL ~ ")" }`
- New rule: `flow_step = { ARROW ~ (checkpoint_call | step_ident) }` (replaces `(ARROW ~ step_ident)*`)
- `"checkpoint"` added to `step_ident` negative lookahead

### AST Changes

- `FlowDecl` gains `checkpoints: HashMap<String, usize>` field mapping checkpoint name to the pipeline step index AFTER which the checkpoint fires.
- E.g., `checkpoint("mid")` after Step1 (pipeline index 0) → `checkpoints = {"mid": 0}`

### State Serialization

A checkpoint serializes:
1. **Current value** — the pipeline's threaded value at the checkpoint point (via `serde_json`, since `Value` implements `Serialize`/`Deserialize`)
2. **Variable scope** — all `variables` in the interpreter at checkpoint time
3. **Pipeline position** — the step index
4. **Timestamp** — Unix milliseconds

Storage backend:
- **SQLite** (preferred): when `memory { persist: "path" }` is declared, checkpoints are stored in `checkpoints.db` alongside the memory store. Uses `INSERT OR REPLACE` with composite primary key `(flow_name, checkpoint_name)`.
- **In-memory fallback**: `HashMap<String, CheckpointData>` on the Interpreter struct. Lost on process restart (by design for tests and ephemeral use).

### Resume Logic

When `set_resume_target(flow_name, checkpoint_name)` is called before `run()`:

1. The interpreter loads checkpoint data for `(flow_name, checkpoint_name)`
2. Restores `variables` from the checkpoint's serialized scope
3. Sets `start_idx = step_index + 1` (resume from the NEXT step)
4. Sets `current` to the checkpoint's saved value
5. Skips all pipeline steps before `start_idx`
6. Continues normal execution from `start_idx` onward

### CLI

New subcommand: `mlog resume <file> --flow=<name> --from=<checkpoint>`

```bash
# Run normally — checkpoint saved after GatherSources
mlog run analysis.mlog

# Kill/restart happens here...

# Resume from "sources" checkpoint — skips GatherSources
mlog resume analysis.mlog --flow=LongAnalysis --from=sources
```

### Public API

```rust
// Set resume target before run()
interp.set_resume_target("FlowName", "checkpoint_name");

// Resume via lib
metalogos::resume_program(source, "FlowName", "checkpoint_name")?;

// List checkpoints
let cps = interp.list_checkpoints("FlowName")?;
// Returns: Vec<(checkpoint_name, step_index, created_at)>

// Delete a checkpoint
interp.delete_checkpoint("FlowName", "checkpoint_name")?;

// Reset all in-memory checkpoints (test isolation)
interp.reset_checkpoints();
```

### Contract Tests

10 tests in `tests/lifecycle_control_contract.rs`:

| Test | Contract |
|------|----------|
| C1: test_checkpoint_saves_state | checkpoint("name") saves state after step |
| C2: test_resume_continues_from_checkpoint | resume skips steps, continues from next |
| C3: test_multiple_checkpoints | Multiple checkpoints in one flow |
| C4: test_list_checkpoints | Lists all checkpoints with correct indices |
| C5: test_delete_checkpoint | Deletion removes checkpoint |
| C6: test_resume_nonexistent_checkpoint | Error on missing checkpoint |
| C7: test_flow_without_checkpoints_backward_compat | Flows without checkpoints run normally |
| C8: test_checkpoint_captures_value | Current value correctly serialized |
| C9: test_resume_restores_variables | Variable scope restored on resume |
| C10: test_reset_checkpoints | Bulk reset clears all |

## Consequences

- **Positive**: Enables crash recovery for long-running flows. Minimal syntax addition (single marker). Checkpoints are transparent — patterns don't need modification. SQLite persistence ensures survival across process restarts.
- **Negative**: Checkpoint serialization includes the full variable scope, which may be large for complex flows. `Value` serialization via serde_json may not handle all opaque types (Secret, Encrypted) gracefully — these serialize as markers.
- **Neutral**: Checkpoint storage piggybacks on the memory persist path. Without `memory { persist: "..." }`, checkpoints are in-memory only (lost on restart). This is acceptable since production deployments with long-running flows will have persistence enabled.
