# METALOGOS — Methodology for Feature Intake

> **Version:** 1.0 | **Date:** 2026-07-04
> Defines the process, criteria, and decision framework for accepting new features
> (builtins, language constructs, standard library additions) into the Metalogos core.

---

## 1. The Cost of a Builtin

Every new builtin is not a single function — it is a permanent maintenance contract.
Adding one builtin touches 2 files and costs ~30 minutes of disciplined work:

| Step | File | What changes |
|---|---|---|
| Implementation | `src/builtins/*.rs` | Function body in the appropriate domain module |
| Registry spec | `src/builtins/registry.rs` | `spec!()` entry (name, arity, category) |
| Documentation | `REFERENCE.md` | Signature, description, return type, example |
| Tests | `tests/` | At least 1 integration test covering happy path + error path |

Total per builtin: **4 locations**. Semantic arity, compiler index, and VM name table are all derived from `registry.rs` automatically.

This cost is why we filter strictly. A builtin added hastily is a builtin maintained forever.

---

## 2. Feature Categories

Not all features are equal. Metalogos has three tiers of adoption:

### Tier 1 — Core Builtin (in `src/builtins/`)

Goes into the Rust runtime. Available in all execution modes (interpreter, VM, JIT).

**Criteria (ALL must be met):**
- Used by 2+ unrelated domains (e.g., not only "finance" or only "Telegram")
- Implementable in <200 lines of Rust without new heavy dependencies (>1MB crate)
- No domain-specific business logic (no "calculate MAPE", no "check trademark")
- Has a clear, general-purpose name (not `send_telegram_voice_as_alloy`)

**Examples of correct Tier 1:**
- `json_get()` — used everywhere
- `mtree_store()` — memory is cross-domain
- `cron_run()` — scheduling is universal

**Examples of WRONG Tier 1:**
- `calculate_mape()` — finance-only
- `check_belarus_vat()` — jurisdiction-specific
- `send_email_via_sendgrid()` — specific provider

### Tier 2 — Standard Library (`std/*.mlog`)

Written in Metalogos itself. Composable from existing builtins.

**Criteria:**
- Useful but domain-specific or complex enough that Rust implementation is unnecessary
- Can be composed from existing Tier 1 builtins
- Benefits from being readable and modifiable by users

**Examples:**
- `std/string.levenshtein(a, b)` — fuzzy matching via string builtins
- `std/math.linear_regression(points)` — statistical calculation via math builtins
- `std/collections.group_by(list, key_fn)` — via each/filter/map

### Tier 3 — Skill / Application Layer (`FOSVED-office-v2/`)

Written as mlog patterns or Python/JS glue. Not in the language repo at all.

**Criteria:**
- Tied to a specific business process, API provider, or domain
- Requires external services, API keys, or domain knowledge
- Useful only within a specific deployment context

**Examples:**
- Department skills (finance, legal, marketing)
- API integrations (OpenAI routing, Telegram bot logic)
- Workflow orchestrations (narad execution, audit pipelines)

---

## 3. Intake Process

```
Source (GitHub, idea, user request)
        │
        ▼
  ┌─────────────┐
  │ 1. One-liner │  "This solves X"
  └──────┬──────┘
         │
         ▼
  ┌──────────────────┐
  │ 2. Tier decision │  Tier 1? Tier 2? Tier 3?
  └──────┬───────────┘
         │
    ┌────┴────┐
    ▼         ▼         ▼
  Tier 1    Tier 2    Tier 3
    │         │         │
    ▼         ▼         ▼
  ┌──────┐ ┌──────┐ ┌──────────┐
  │ 3a.  │ │ 3b.  │ │ 3c.      │
  │ Rust │ │ mlog │ │ Skill /  │
  │ impl │ │ std  │ │ FOSVED   │
  └──┬───┘ └──┬───┘ └──────────┘
     │        │
     ▼        ▼
  ┌──────────────────┐
  │ 4. 5-file commit │  builtins + semantic + compiler + vm + test
  └──────────────────┘
         │
         ▼
  ┌──────────────────┐
  │ 5. REFERENCE.md  │  Document signature + example
  └──────────────────┘
```

### Step 1 — One-liner

Every feature request must answer one question: **"What problem does this solve?"**

Not "what does it do" — "what problem does it solve." The distinction filters vanity features.

| Good one-liner | Bad one-liner |
|---|---|
| "Users need fuzzy string matching for search" | "Add Levenshtein distance" |
| "Scheduled pattern execution for recurring tasks" | "Add cron syntax parsing" |
| "Store hierarchical knowledge that compresses over time" | "Add a memory tree" |

### Step 2 — Tier Decision

Apply the decision matrix:

| Question | Tier 1 | Tier 2 | Tier 3 |
|---|---|---|---|
| Used by 2+ domains? | **Yes** | Maybe | No |
| New dependency >1MB? | **No** | N/A | N/A |
| <200 lines Rust? | **Yes** | N/A | N/A |
| Domain-specific logic? | **No** | Maybe | Yes |
| Requires external service? | **No** | Maybe | Yes |

If 3+ answers point to a tier — that's the tier. If split — default to higher tier (Tier 2 over Tier 1).

### Step 3 — Implementation

- **Tier 1:** Full 5-file pipeline (builtins → semantic → compiler → VM → test)
- **Tier 2:** Write in `std/*.mlog`, add example, no Rust changes
- **Tier 3:** Write in FOSVED-office-v2 or as standalone .mlog file

### Step 4 — Commit Discipline

For Tier 1, a single commit should touch all 5+ files. Never split a builtin across commits — partial additions create broken intermediate states where semantic knows the name but compiler doesn't, causing VM crashes.

### Step 5 — Documentation

Every Tier 1 builtin gets a REFERENCE.md entry with:
- Signature: `name(arg1: Type, arg2: Type) -> ReturnType`
- One-line description
- Example usage (2-5 lines)
- Return struct fields (if applicable)

---

## 4. Anti-Patterns (DO NOT)

### A. "Feature by feature" accumulation

Adding every interesting GitHub project as a builtin. Result: 500 builtins, unmaintainable, slow compile.

**Rule:** If the last 5 features were all Tier 1, something is wrong. Most features should be Tier 2 or 3.

### B. "Premature generalization"

Abstracting a domain-specific need into a "general" builtin that only the original requester uses.

**Rule:** Wait for 2 independent use cases before generalizing.

### C. "Dependency creep"

Adding a 5MB crate for a function that could be 50 lines of pure Rust.

**Rule:** Default to no new dependencies. If a crate is needed, it must save >500 lines of code.

### D. "Name specificity"

Naming a builtin after its first use case: `telegram_send_voice_as_alloy` instead of `tts_send`.

**Rule:** Builtin names should be domain-agnostic. If you can't name it without mentioning a specific service, it's Tier 3.

---

## 5. Quantitative Thresholds

| Metric | Current | Warning at | Hard limit |
|---|---|---|---|
| New dependencies per version | — | 2 | 5 |
| Compile time (clean) | ~60s | 90s | 120s |
| Binary size (Linux x86_64) | ~6MB | 8MB | 12MB |
| Per-module LOC (any `src/builtins/*.rs`) | 3 170 | 4 000 | 5 000 |

When a warning threshold is hit, the next feature intake MUST justify why it should still be Tier 1.

> **Note (v0.17):** The "Total builtins" and "`builtins.rs` lines" rows were removed.
> The codebase migrated from a single 5K-line `builtins.rs` to 34 domain modules
> (29K lines total, avg 10.5 builtins per module). A hard cap on total builtin
> count no longer reflects the project's architecture — per-module complexity
> and dependency hygiene are more meaningful signals.

---

## 6. Source Categories for Feature Ideas

Not all sources are equal. Prioritize accordingly:

| Source | Reliability | Action |
|---|---|---|
| **User pain point** (real usage of FOSVED/Metalogos) | High | Immediate intake |
| **Repeated pattern** in existing mlog code | High | Extract to Tier 2, then consider Tier 1 |
| **GitHub project** with proven adoption (>1K stars) | Medium | Evaluate against Tier 1 criteria |
| **Academic paper** with clear algorithm | Medium | Consider for Tier 2 (pure mlog implementation) |
| **"Would be cool to have"** | Low | Log, do not act |
| **Hype-driven** (trending repo, no proven use) | Low | Ignore until proven |

---

## 7. Version Planning

Features are not shipped individually. They are batched into versions:

- **Patch (0.8.x):** Bugfixes only. No new builtins.
- **Minor (0.x.0):** New builtins, language features. Batched from the intake backlog.
- **Major (x.0.0):** Breaking changes. Requires migration guide.

The intake backlog is maintained in ADR format: when a feature is proposed and accepted,
an ADR is created documenting the decision, alternatives, and rationale.

---

*This document is itself versioned. Updates require a commit message referencing this file.*