# Doc-tests (Naryad #287) — contract

`mlog test --docs [GLOB...]` — the rustdoc doc-tests pattern for Metalogos:
every ```mlog block of documentation is an execution candidate; smuggled-in
falsehood is impossible. Default files: `REFERENCE.md`, `README.md`,
`docs/book/**/*.md` (the living documentation of the LANGUAGE).

> `docs/adr/**` and `docs/research/**` — historical decision records: they
> capture the state at the moment of the decision and are NOT scanned by default.
> Explicit request: `mlog test --docs "docs/adr/**/*.md"`.

## Block contract

| Marker (on any line of the block) | Semantics |
|---|---|
| *(no marker)* | The block must execute without an error |
| `// expect: <value>` | The last line of the program output equals `<value>` (supported on blocks with `flow`) |
| `// expect-error: <code?>` | Execution must fail; the optional `<code>` is checked as a substring of the error text |
| `// no-run` | Parsing only, no execution |
| `// doc-test: skip` | Full skip (grammar cheat sheets, sketches; counted in the counter) |

## Classification and execution

- A block with `flow` — a full program (TW run; `--backend vm` — compilation + VM; a VM compilation error = **skip**, ADR-0105, a VM runtime error = fail).
- A block of declarations (`pattern`, `learnable`, `entity`, `fluid`, `reflex`, `sandbox`, `db`, `schema`, `template`, `tool`, `hook`, …) — parse + registration without execution.
- A statements fragment — wrapped in `pattern __DocTest` + `flow Main`; the `import` lines of the fragment are hoisted to the top level.

## Read-only profile

1. **Ephemeral cwd** — each block executes in a fresh tempdir: file/db effects are isolated, no side effects on the CI machine; the sandbox #131/#252 cuts escapes outward loudly.
2. **Network/exec builtins replaced with stubs** that fail loudly with `[DOC_SANDBOX]`: `http_get`, `http_post`, `http_post_multipart`, `http_download`, `smtp_send`, `smtp_send_html`, `imap_*`, `mcp_*`, `exec`, `exec_argv`. The substitution is LOCAL to the interpreter (the SSOT registry is untouched).
3. `call_llm`/`call_claude` — mock backend (without keys they do not touch the network).

## Report

```
doc-tests: <files> files, <N> extracted, <M> executed, <K> no-run, <E> expect-error, <S> skipped, <F> failures
```

Every error is printed with a semantic anchor `file: section: block #k`
(the file's mlog-block number + the nearest markdown heading — lines change,
the anchor does not). Exit code 1 when `F > 0` — the merge CI gate.

## Repository status

`mlog test --docs` (default) — green: REFERENCE/README/docs/book
execute; the grammar cheat sheets of §5 of REFERENCE and of the syntax
reference are marked `// doc-test: skip`; the mutation-error demo examples are
`// expect-error`. New documentation examples must pass the doc-tests
(gate in ci.yml) — "documentation for humans and agents" becomes verified.
