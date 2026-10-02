# docs/privacy.md — the engineering privacy policy: what the language stores, where, and how it goes away

> **Status: engineering policy — an honest inventory, not a legal document.**
> Created by naryad №519 (issue #803, the consolidated audit 28.09 C-20,
> wave 21), maintained under the same naryad protocol as
> [limitations.md](limitations.md). This page states what the Metalogos
> runtime actually does with data a person can recognize themselves in —
> each claim carries a verdict: **CONFIRMED** (the code anchor is given and
> was verified at the check-in commit), **PARTIAL** (implemented with the
> named caveat), or **ABSENT** (not implemented — the honest absences are
> the load-bearing part of this page). The document promises nothing the
> code does not do. It is NOT a GDPR declaration, NOT a privacy statement
> for a deployed product, and NOT legal advice: an external adoption needs
> its own legal review — the language gives the deployer the mechanics and
> the honest limits; the deployer owns the compliance case (the same
> division as the threat model's key-management note).

---

## 0. The one-paragraph summary

The language runtime has **no hidden data directory and no telemetry**: every
persistent path is explicit — declared by the program (`memory { persist: }`,
`db { url: }`, `vec_store`), chosen by the operator via environment variables
(`METALOGOS_MEMORY_DB`, `METALOGOS_LLM_TRACE`), or written by an explicit
`*_save`/`*_export` builtin call that passes the filesystem sandbox. In-process
state (registries, caches without a declared persist file, consent ledger,
media store) dies with the process. Voiceprint persistence exists as library
code with AES-256-GCM at rest (№517), but **is not wired to any production
call site** — the voice builtins are loud stubs, so a shipped runtime stores no
voiceprints at all. Deletion comes in three honest flavors: hard SQL `DELETE`
(the legacy memory lane), a soft forget-ledger with an owner-side vacuum
(`memory_forget`, №280/№445), and process-exit eviction (everything RAM-only).

---

## 1. The storage model — every path is explicit (CONFIRMED)

There is no fixed "data home". The runtime never invents a location for
personal data:

| Path source | Mechanism | Anchor |
|---|---|---|
| The program | `memory { persist: "path" }` — enables the `memories`/`kv_store` SQLite file, the FTS5 index, `checkpoints.db` in the same directory | `src/interpreter/memory.rs:161–176`, `src/memory_store.rs:408–449` |
| The program | `db { url: "sqlite:path" \| "sqlite::memory:" }` — program-declared tables and engine services that reuse the file (kv, LLM caches, distill hub) | `src/interpreter/db.rs:149–180`, `src/distill_hub.rs:199–208` |
| The program | `vec_store(db_path, …)` — sqlite-vec virtual tables in a sandboxed path | `src/builtins/vector.rs:12–38, 230–330` |
| The operator | `METALOGOS_MEMORY_DB` — the typed memory lane; fail-closed if the file cannot be opened | `src/memory_typed.rs:747–757, 800` |
| The operator | `METALOGOS_LLM_TRACE` — the JSONL trace file (off by default, metadata-only content) | `src/llm.rs:1013–1019, 1058–1059` |
| Explicit call | `media_save(handle, path)`, `vision_export`, `video_export`, `consent_ledger_export(path)` — sandboxed writes | `src/builtins/media.rs:166–215`, `src/video/mod.rs:572–592` |

Every program-supplied path passes the fs sandbox `sandbox_path_ex`
(naryads №131/№252/№475) with the deny-list that includes `.env*`, `*.db`,
`*.sqlite*`, `.git/**` — anchor `src/interpreter/values.rs:594–600`. The
path-canonicalization rule is one fact wide and one fact deep: **if no
explicit path was given, nothing was written** (the in-process registries in
§2.10 hold their state in RAM and evaporate at exit).

---

## 2. The data inventory, category by category

### 2.1 Voice fingerprints — biometric data (PARTIAL: honest crypto, UNWIRED)

A voiceprint is a 192-dim f32 embedding (ECAPA contract,
`src/voice/encoder.rs:15`) — biometric data, a GDPR Art. 9 special category
when it identifies a person. The storage layer (`VoiceStore`, SQLite; tables
`voiceprints`, `voice_artifacts`, `consent_ledger` — `src/voice/store.rs:96–128`)
is **CONFIRMED** to do the following in the REAL runtime: AES-256-GCM at rest
(the same `aes-gcm` primitive as the `encrypt()`/`decrypt()` builtins, no new
dependencies), the key arriving ONLY through the secret()-gate semantics
(env-sourced 64-hex `METALOGOS_VOICEPRINT_KEY`, never derived from the name),
a fresh random 96-bit nonce per write, the self-contained
`nonce ‖ ciphertext+tag` blob, and the `algo = 'AES-256-GCM-v1'` schema label
(`src/voice/store.rs:39, 536–588`). A keyless or short-key save is
fail-closed (`[VOICE_INSECURE_STORE]`, store.rs:217–229); a legacy
pre-№517 row is never silently decrypted (`[VOICEPRINT_STALE]`, store.rs:289–298);
a wrong key (or a legacy №517-era row after the v0.28.0 deadline) refuses at
the GCM auth tag (`[VOICEPRINT_DECRYPT]`, store.rs:605–664).
Since №527 the ciphertext is BOUND TO ITS SUBJECT: every write carries the GCM
AAD = (subject_id, registry, schema version) (`src/voice/store.rs:50–73`) — a
blob transplanted onto another subject's row fails authentication (the swap
attack the bare tag accepted is closed), and the decoded key buffer lives under
`Zeroizing` (wiped at the operation's scope exit). The №517-era transition
window (empty-AAD rows readable with a loud warning) CLOSED at v0.28.0, as the
limitations.md deadline row required (the №524 rule): the transitional
empty-AAD decrypt fallback and the `VoiceprintCryptoStatus::LegacyNoAad` flag
are removed — a legacy №517-era row now refuses with the single coded
`[VOICEPRINT_DECRYPT]` refusal through both load entry points, and the only
path back is re-enroll/re-save (`store.rs:262–340`); every write was
AAD-bound since №527, so the legacy population only shrank toward the deadline.

**The PARTIAL caveat, stated twice on purpose:**

- The **mock runtime** (`METALOGOS_MOCK_LLM=1`) persists through an INSECURE
  XOR placeholder keyed by the PUBLIC name (`src/voice/store.rs:391–407`) —
  it is not encryption, and the schema marks such rows `algo =
  'INSECURE-XOR-MOCK'` (store.rs:29) so a mock row can never masquerade as
  encrypted.
- `VoiceStore::new` has **no production call site** — only tests construct it
  (`src/voice/store.rs:523` is a test-module helper; the test files
  `tests/naryad_512_voice_insecure_store.rs`, `tests/naryad_517_voice_aes_gcm.rs`).
  The program-facing builtins `voice_enroll` / `voice_save` / `voice_load`
  are **loud stubs that always error** (`src/voice/mod.rs:269–303`,
  registered at `src/builtins/registry.rs:909–919`). **A shipped runtime
  therefore stores zero voiceprints**; the honest crypto is the load-bearing
  foundation for the day the wiring lands, not a working feature today.

**Delete path: available since the v0.27.x line (№526, gh#835 — closed in
the same PR as this row per the №524 rule).** The erasure surface has two
layers, both landed by №526:
- **Runtime registry** (the live surface — a shipped runtime persists zero
  voiceprints, see above): `voice_delete(handle)` erases a voiceprint (a
  Voice handle) or an audio artifact (an Audio handle) from
  `VOICE_REGISTRY`, IDEMPOTENT (`"deleted"`/`"absent"` — a repeated erase
  succeeds, GDPR Art. 17(1)); `voice_list()` lists the held prints (id +
  model) and artifacts (id + size) as the informed-deletion basis. Neither
  is feature-gated: the erasure right cannot depend on a build flag.
- **Persisted store** (`VoiceStore`): `delete_voiceprint(name)` runs the
  SECURE-DELETE path — the ciphertext blob is zero-overwritten in place
  BEFORE the row is removed, the same-name audio artifact row gets the
  same treatment ("файл артефакта + запись реестра"); `list_voiceprints()`
  enumerates the persisted prints (name/model/saved_at/algo/ciphertext
  length) WITHOUT decrypting. HONEST BOUNDARY: SQLite cannot guarantee
  per-row block-level erasure (freelist/WAL page images may persist until
  reused) — the row-level overwrite erases the row's live copy; the
  file-level guarantee stays the DB file owner's (delete the whole file).
- The `consent_ledger` rows SURVIVE erasure BY DESIGN: they are the
  Art. 9 consent proof (a pseudonymous embedding hash, no biometric
  bytes), the retention basis this policy documents.

Retention (unchanged by №526): rows live until erased by the delete path
above, replaced, or the DB file is deleted by its owner. The in-process
companion `VOICE_REGISTRY` (audio
artifacts + mock voiceprints, RAM only) is bounded at 64 entries with
oldest-id eviction (№515; `src/voice/mod.rs:46–53, 71–83, 178–179`) and dies at
process exit.

### 2.2 Memory statements — the remember/recall/forget surface (CONFIRMED, with two lanes)

What is remembered: free-text `value` plus priority/confidence/decay/timestamp
and an embedding (`MemoryEntry`, `src/memory_store.rs:25–43`) on the legacy
lane (`memorize`/`recall`/`forget` — `src/memory_ops.rs:54, 208, 259`), or
KV records on the `kv_set`/`kv_get`/`kv_delete` lane
(`src/builtins/memory.rs:99–235`).

- **In-process by default (CONFIRMED):** the store is `InMemoryStore`
  (Vec-backed, `src/memory_store.rs:6`). Nothing reaches disk unless the
  program declares persistence.
- **Persisted (CONFIRMED):** with `memory { persist: }` the SQLite tables
  `memories` + `memories_fts` (FTS5) and `kv_store` are created
  (`src/memory_store.rs:416–449`, `src/builtins/memory.rs:64`) — **plaintext,
  unencrypted** (see §5). The typed lane (`METALOGOS_MEMORY_DB`) encrypts
  private-container entries with AES-256-GCM (`is_enc` column,
  `src/memory_typed.rs:20–28, 234–250, 815–838`).
- **Forget, flavor 1 — hard DELETE (CONFIRMED):** `forget(query, cutoff)`
  executes `DELETE FROM memories WHERE value LIKE … AND created_at < cutoff`
  (`src/memory_store.rs:654–663`); `kv_delete`/`mem_delete` hard-`DELETE` from
  `kv_store` (`src/builtins/memory.rs:134–152, 216–235`). When the store is
  in-process, the same code path deletes from RAM.
- **Forget, flavor 2 — the soft ledger (CONFIRMED):** `memory_forget`
  (№280/№445) does NOT physically delete; it records forgotten ids into the
  `__forgotten` ledger table with a batch id and a consent gate
  (`MEMORY_FORGET_CONSENT_REQUIRED`) and a poisoned quarantine cascade
  (`MEMORY_POISONED`, ADR-0173) — `src/builtins/memory_forget.rs:24–33,
  337–396, 473–523`; `dry_run` is the default arity. The physical vacuum is
  an owner operation, not a builtin (memory_forget.rs:28).
- **`memory_forget_all`: ABSENT.** There is no "wipe everything" builtin
  (`rg memory_forget_all src/` → no matches at the check-in commit). The
  typed lane has a TTL auto-forget sweep (`memory_retain_ttl`, the
  `ttl_unix` column — ADR-0176) as the closest automated analog.

### 2.3 Contacts and calendar — the data lives on the remote server (CONFIRMED)

Both pillars are CalDAV/CardDAV **clients**: contacts and events are relayed
to and from the server the program connected to; the runtime writes nothing
of them to disk. Deletion is real deletion on the server:
`card_delete(contact_uid)` performs an HTTP `DELETE` against the CardDAV
server (`src/builtins/contacts.rs:573–613`), `cal_delete(event_uid)` the same
against CalDAV (`src/builtins/calendar.rs:609–650`).

The locally held personal data is the **session credentials**: `CARD_SESSIONS`
/ `CAL_SESSIONS` are process-global maps holding `{url, user, pass, home_set}`
as plaintext `String`s — **PARTIAL**: not `Zeroizing`, not bounded, no
disconnect builtin; they are cleared only by process exit
(`src/builtins/contacts.rs:14–24`, `src/builtins/calendar.rs:23`). This is a
named gap (§7), not a hidden one.

### 2.4 Consent and likeness (CONFIRMED, process-local)

The consent ledger is a **process-local in-memory SQLite**
(`Connection::open_in_memory`, `src/consent.rs:43`; the header comment states
"the ledger is PROCESS-LOCAL" at consent.rs:11–14): table `consent_ledger`
(grant/revoke rows, subject, scope, TTL — consent.rs:47–56). Grants record;
**revocation exists** (`consent_revoke`, `src/builtins/consent.rs:80`) and
triggers the derived→poisoned cascade (builtins/consent.rs:114); the
active-grant check is fail-closed (consent.rs:135–160, №428). Export is a
classified FILE EGRESS with an audit event (`consent_ledger_export`,
builtins/consent.rs:221). **No purge function exists** — the ledger is
cleared by process exit; the voice store keeps a separate `consent_ledger`
table inside the VoiceStore DB (same test-only wiring, §2.1).

Likeness tokens are **opaque, RAM-only, never serializable as strings**
(`src/likeness.rs:19–20, 41–56, 117–129`) — pending challenges and issued
tokens live in the in-process `LIKENESS_REGISTRY` (unbounded; §7) and vanish
at exit. A token doubles as the runtime credential unsealing `media_save`
for camera/likeness origins (threat-model.md, the `VIDEO_LIKENESS_NO_CONSENT`
row).

### 2.5 Media — sealed in process, manifests on egress (CONFIRMED)

Media bytes never enter program values; programs hold opaque handles
(`src/media/mod.rs:37–137`). The per-run `MediaStore` is in-process ("media
belongs to a program run, not to the process" — media/mod.rs:293–296); entries
with non-`public` sensitivity are **sealed with AES-256-GCM**
(`nonce(12) ‖ ct+tag`), the per-store key is random, `Zeroizing`, never
serialized, `Debug`-redacted (media/mod.rs:174–186, 227–275, 296–349);
`media_release` to refcount 0 evicts and zeroizes (media/mod.rs:480–494).
The only sanctioned materialization is `media_save(handle, path)` — a
sandboxed write through the fs gate (compile-time `SECRET_LEAK` clearance +
runtime `MEDIA_SEALED_EGRESS` backstop) **plus a mandatory sidecar manifest**
`<path>.manifest.json` with `kind, origin, conf, bytes_sha256, synthetic,
timestamp` (media/mod.rs:166–215; a failed sidecar write is a loud error).
Vision artifacts persist as PNG blobs **inside the program-declared `db { url
}` SQLite** with verbatim provenance manifests — no upsert, no delete; a name
collision is a loud error (`src/vision/store.rs:9–36, 94–108`).
`video_export(handle, path)` writes the muxed video to a program-chosen path
(`src/video/mod.rs:572–592`).

### 2.6 LLM traces and caches (CONFIRMED, opt-in / persisted-with-caveats)

- **Traces are opt-in and metadata-only (CONFIRMED).** Without
  `METALOGOS_LLM_TRACE` nothing is traced; with it, every LLM call appends
  ONE JSONL line with OpenTelemetry GenAI field names — provider, model,
  token counts, latency, status, cache hit, backend, finish reason — and
  **no prompt or response text** (`src/llm.rs:940–994, 1013–1019`); the path
  is operator-configured, never program-controlled (№475; llm.rs:1058–1059).
  No rotation — the operator rotates the file (llm.rs:1007–1012).
- **Exact LLM cache (CONFIRMED):** `llm_cache (hash, response, created_at,
  ttl)` in the `memory { persist }` file — stores LLM responses keyed by the
  request hash, TTL-enforced (`src/interpreter/learnable.rs:1174`).
- **Semantic cache (CONFIRMED, the derived-data caveat):**
  `llm_cache_semantic (key, response TEXT, embedding BLOB, dim, created_at,
  ttl)` — it stores the **embedding of the user input** (a derived personal
  datum, reversible to intent in the loose sense) next to the response text
  (learnable.rs:983–1080). It refuses loudly without a declared persistence
  (learnable.rs:966–971); expired rows are deleted (learnable.rs:1043–1045).
- **`trace_start`/`trace_end`** are in-session named spans in process memory
  (`src/builtins/memory.rs:1039–1053`) — never persisted.

### 2.7 Vector store and the forget ledger (CONFIRMED)

`vec_store`/`vec_search` keep embeddings plus an optional text payload in
sqlite-vec `vec0` virtual tables at a program-specified, sandboxed db path,
with scope binding (`vec_meta`, `vec_scopes`, №281;
`src/builtins/vector.rs:182–330`) — **no TTL, no encryption**. The
`memory_forget` ledger adds the `<table>__forgotten` table
(vector.rs:807) — the soft-delete record of §2.2, flavor 2.

### 2.8 Profile — a read-only projection of KV (CONFIRMED)

`user_profile(db_path, container)` aggregates existing `kv_store` rows with
the key convention `container:<container>:<bucket>:<key}` deterministically
(`src/builtins/profile.rs:62, 146–218`); there is **no profile writer** —
profile facts are ordinary memory records, deletable only by deleting their
keys. The compile-time `src/profile.rs` is a different module entirely
(compat profiles, nothing persisted).

### 2.9 Secrets and environment (CONFIRMED)

`secret("KEY")` reads env into `Value::Secret` backed by `Zeroizing<String>`
— zeroized on drop, never printable, never concatenable; `respond(secret(…))`
is a compile-time `SECRET_LEAK` refusal (`src/builtins/crypto.rs:264–295`,
`src/interpreter/values.rs:16–40`; the threat-model `SECRET_LEAK` row). The
runtime never writes env-sourced secrets to disk. One named factual note:
`web_search` sends the operator's `SERPAPI_KEY` as a URL query parameter to
the search provider (`src/builtins/http.rs:1051–1075`) — it can land in
remote/proxy logs; that is outside the local storage story but belongs in
any deployment's privacy review.

### 2.10 Process-global registries — RAM-only, bounded (№515) (CONFIRMED)

All registries hold state in process memory and evaporate at exit. Since
№515 the error-growth registries are **bounded at 64 with oldest-entry
eviction** ("bounded ≠ request-scoped" — limitations.md): `PDF_DOCS`
(in-progress PDF builds; `src/builtins/pdf.rs:126–162`), `VIDEO_REGISTRY`
(`src/video/mod.rs:52–63, 264`), `VOICE_REGISTRY` (`src/voice/mod.rs:53, 178–179`),
and `LLM_STREAM_REGISTRY` is capped with a loud `STREAM_LIMIT_REACHED`
(№263; `src/llm.rs:2137–2162`). Not bounded, named openly: `CARD_SESSIONS`/
`CAL_SESSIONS` (§2.3), `LIKENESS_REGISTRY` (§2.4), `REMINDERS`
(`src/builtins/cron.rs:67–78` — and its SQLite persistence helper
`init_reminder_persist` exists but has no call site, cron.rs:80). GLOBAL_TEMPLATES
is bounded by name-overwrite semantics (`src/builtins/http.rs:26`).

### 2.11 Server sessions and audit (CONFIRMED)

`mlog serve` keeps its session and audit tables (`sessions`, `audit_log` —
user ids, session ids, action rows) in an **in-memory SQLite**
(`Connection::open_in_memory`, `src/server.rs:1276, 2078–2088`); the audit
ring is a RwLock Vec (server.rs:1284). They are restart-volatile by
construction.

---

## 3. Deletion semantics — the three flavors, consolidated

| Flavor | Mechanism | Where it applies | The honest caveat |
|---|---|---|---|
| **Hard DELETE** | SQL `DELETE` (or Vec `retain` in RAM) at the moment of the call | `forget`, `kv_delete`, `mem_delete`, `mtree_forget`, expired cache rows, `voice_delete` (№526 — the registry row + the store row after a zero-overwrite) | Immediate and irreversible; FTS5 indexes are kept in sync by triggers (`src/memory_store.rs:437–449`) |
| **Soft ledger + owner vacuum** | `__forgotten` rows record the deletion intent; nothing physically leaves the file until an owner-side vacuum | `memory_forget` (№280/№445) | The ledger table itself contains ids/reasons; the vacuum is NOT a builtin — **ABSENT** as an automated path |
| **Process-exit eviction** | Everything RAM-only dies with the process | consent ledger, likeness tokens, sessions/audit, all bounded registries, in-process memory and media | No durability and no deletion problem at the same time; while the process lives, the data lives |
| **Remote DELETE** | HTTP `DELETE` to the CardDAV/CalDAV server | `card_delete`, `cal_delete` | Deletion authority is the remote server's, not this runtime's |

**What does NOT exist (ABSENT — the load-bearing list; the №526 erasure
path REMOVED `voice_delete`/`voice_list` from this list — see §2.1):**
`memory_forget_all`; a consent-ledger purge; a VISION artifact
delete-or-upsert builtin; CardDAV/CalDAV session
eviction; a reminder persistence wiring. Nothing in this document should be
read as promising any of them.

---

## 4. Encryption status — what is and is NOT encrypted at rest

| Data | At-rest status | Anchor |
|---|---|---|
| Voiceprints, REAL runtime | **AES-256-GCM** (env-key via secret() semantics, per-write nonce, the №527 AAD subject binding) — but the store is unwired (§2.1) | `src/voice/store.rs:536–588` |
| Voiceprints, mock runtime | **INSECURE name-keyed XOR**, visibly marked `INSECURE-XOR-MOCK` | `src/voice/store.rs:41, 467–479` |
| Media entries, non-public sensitivity | **AES-256-GCM**, per-store random `Zeroizing` key | `src/media/mod.rs:227–349` |
| Typed-memory private containers | **AES-256-GCM** (`is_enc` rows) | `src/memory_typed.rs:234–250, 815–838` |
| Legacy `memories` / `kv_store` / vector payloads / LLM caches / distill samples | **NOT encrypted** — plaintext SQLite, protected only by the file system and the fs sandbox on the way in | §2.2, §2.7 |
| Consent ledger, sessions, audit | **NOT encrypted** (in-memory) | §2.4, §2.11 |

The plaintext list is the honest frontier: **everything that can be persisted
as free text today can be persisted unencrypted today.** The encryption
mechanics exist (the same `aes-gcm` primitive in three places) but the
legacy memory lane predates them and was never retrofitted — that is a
recorded debt, not a secret.

---

## 5. Retention defaults (CONFIRMED)

- **Default retention: unbounded.** No TTL on the legacy memory lane, no TTL
  on vector payloads, no TTL on voiceprint rows (once the store is wired).
  Data lives until an explicit delete or the file is removed.
- **TTL where it exists:** LLM caches carry `ttl` columns with row deletion
  on expiry (`src/interpreter/learnable.rs:1043–1045, 1174`); consent grants
  carry `ttl_seconds` with expiry-checked validation (`src/consent.rs:47–160`);
  the typed memory lane has the TTL auto-forget sweep (ADR-0176); decay and
  prune builtins can age memory out on demand (`src/builtins/memory.rs:857–941`).
- **Restart-volatile by default:** everything in §2.10/§2.11 and the default
  in-process memory store.

---

## 6. What this document does NOT claim (and where the boundaries live)

1. **No legal compliance claim of any kind.** GDPR Art. 9, BIPA and similar
   regimes are named in the threat model only as risk context
   (threat-model.md, the `VIDEO_LIKENESS_NO_CONSENT` and voice-at-rest rows).
   Whether a deployment is compliant is the deployer's legal case, not an
   engineering verdict.
2. **No security claim beyond the threat model.** The threat model
   (threat-model.md) owns the taint/sink/gate story (`SECRET_LEAK`,
   `PII_EGRESS_*`, `VOICE_EGRESS_UNCONSENTED`, sink clearance); this page
   owns only the storage/deletion/retention inventory. The two must not
   drift: if a row here contradicts a row there, one of them is a defect.
3. **No promises about future wiring.** The voice store's honest crypto
   (№517), the bounded registries (№515) and the forget ledger (№280/№445)
   are shipped facts; their production wiring, a voiceprint delete path and
   a plaintext-lane retrofit are NOT shipped and are not scheduled by this
   document.
4. **No claim that mock-runtime data handling is safe.** The
   `METALOGOS_MOCK_LLM=1` runtime exists for tests; its XOR placeholder and
   its schema mark exist precisely so that no mock row can pretend to be
   real.

Maintained under the naryad protocol: edits arrive with a naryad, verdicts
carry anchors, and an anchor that no longer verifies is a defect to fix in
the same naryad that moved it. Related reading: [threat-model.md](threat-model.md),
[limitations.md](limitations.md), [REALITY.md](REALITY.md).
