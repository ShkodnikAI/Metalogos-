# Metalogos Memory — Roadmap (Phases 3-6)

> **Who**: Rust development by Z + owner. Admin monitors readiness only.
> **Status**: Phase 3 (parallel accumulation) is ACTIVE.

## Current Architecture

Two memory systems run in parallel inside FOSVED Office:

| System | Layer | Mode | Technology |
|---|---|---|---|
| **Python FosvedMemory** | HTTP (port 4000) | PRIMARY (read+write) | Python, 13 types, decay, supersession |
| **Metalogos Rust Memory** | Builtin | ACCUMULATOR (write-only) | Rust, SQLite FTS5 BM25 + cosine RRF |

Integration point: `app.mlog` → `YanaAutoMemorize()` pattern (lines 828-849).
Every fact extracted by Groq and sent to Python memory is ALSO written via `memorize(content, conf, mem_type)`.

## Phase 3: Parallel Accumulation (ACTIVE)

**What**: Shadow-write all facts from YanaAutoMemorize into Metalogos memory.
**Status**: DONE — `app.mlog` already has dual-write.
**Monitoring**: `/memory` command in Telegram, `scripts/memory_monitor.py` dashboard.
**DB path**: `data/metalogos_memory.db` (production), `test_memory.db` (tests).

### Readiness Thresholds for Phase 4

| Metric | Threshold | Current |
|---|---|---|
| Total memories | >= 500 | 12 (test data) |
| Distinct mem_types with >=20 entries | >= 5 types | 0 |
| Target types: fact, preference, pattern, persona, rule | all active | none yet |
| Average confidence > 0.5 | required | 0.8 (test only) |

### Monitoring Commands

- **Telegram**: `/memory` — quick status of both systems
- **Dashboard**: `python3 scripts/memory_monitor.py` → `download/memory_dashboard.html`
- **SQL**: `SELECT mem_type, COUNT(*) FROM memories GROUP BY mem_type ORDER BY COUNT(*) DESC`

---

## Phase 4: L2 Scenario Grouping (BLOCKED — needs data)

**Owner decision required**: Admin says "data is ready" (>=500 typed entries).

### Scope

1. New SQLite tables in `memory_store.rs`:
   - `scenarios` (id, name, description, centroid_embedding, created_at, mem_type_filter)
   - `scenario_members` (scenario_id, memory_id, similarity_score)

2. New builtin: `group_scenarios(query, k, type_filter?)`
   - Clusters memories by semantic similarity
   - Uses embedding cosine distance + BM25 co-occurrence
   - Creates scenario groups automatically
   - Returns scenario summaries

3. New builtin: `recall_from_scenario(scenario_id, query, k)`
   - Search within a specific scenario group
   - Higher precision than global recall

4. Files to modify:
   - `src/memory_store.rs` — new tables, clustering logic
   - `src/interpreter/memory.rs` — new builtins
   - `src/interpreter/execution.rs` — dispatch routing
   - `tests/phase4_scenario.rs` — integration tests

### Deliverables
- [ ] Scenario table schema + migrations
- [ ] `group_scenarios()` builtin with tests
- [ ] `recall_from_scenario()` builtin with tests
- [ ] ADR document: `docs/adr/NNN-scenario-grouping.md`
- [ ] Updated `MEMORY_ROADMAP.md` with Phase 4 results

---

## Phase 5: Python Memory Migration Assessment

**After Phase 4 is stable.**

Evaluate whether Metalogos memory can fully replace Python FosvedMemory:
- Compare recall quality (precision/recall) between systems
- Feature gap analysis: decay, supersession, conflict detection
- Decision: keep Python as fallback, or remove entirely
- Owner approval required for removal

---

## Phase 6: L3 Persona Aggregation (BLOCKED — needs Phase 4)

**Owner decision required**: Admin says "persona data is ready" (>=200 persona entries, >=1000 total).

### Scope

1. New SQLite table: `personas` (id, name, attributes_json, centroid_embedding, source_scenario_ids, created_at)

2. New builtin: `build_persona(name, scenario_ids?)`
   - Aggregates memories from multiple scenarios into a persona profile
   - Extracts key attributes: preferences, patterns, goals, constraints
   - Stores persona with centroid embedding

3. New builtin: `persona_profile(name)` → JSON with attributes

4. New builtin: `recall_persona(name, query, k)` → persona-filtered recall

5. Files to modify:
   - `src/memory_store.rs` — persona table, aggregation
   - `src/interpreter/memory.rs` — new builtins
   - `tests/phase6_persona.rs` — integration tests

### Deliverables
- [ ] Persona table schema
- [ ] `build_persona()` builtin with tests
- [ ] `persona_profile()` and `recall_persona()` with tests
- [ ] ADR: `docs/adr/NNN-persona-aggregation.md`

---

## Pending Tasks (from previous sessions)

| Priority | Task | Status |
|---|---|---|
| P1 | Phase 4 — L2 scenario grouping | BLOCKED: needs >=500 memories |
| P1 | Phase 6 — L3 persona aggregation | BLOCKED: needs Phase 4 |
| P2 | Наряд №43 Block 2-3 — VM ExecuteRules priority bug | DONE: sort added (session 0805) |
| P2 | Наряд №42 Block 4 — Confidence propagation | DONE: min() heuristic (session 0805) |
| P3 | Наряд №45 — Merge naryad-44-sql-params into main | Pending (branch not found in origin) |

## Key Files Reference

```
Metalogos-/src/memory_store.rs          — Core memory backend (SQLite + FTS5 + cosine)
Metalogos-/src/interpreter/memory.rs    — Memory builtins (memorize, recall_top_k)
Metalogos-/src/interpreter/execution.rs — Dispatch routing
Metalogos-/tests/phase76_contract.rs   — Phase 7.6 integration tests
FOSVED-office-v2/app.mlog               — Office config (lines 828-849: dual-write)
FOSVED-office-v2/lib/fosved_memory.py   — Python memory (13 types, NOT modified)
scripts/memory_monitor.py               — Dashboard generator
```
