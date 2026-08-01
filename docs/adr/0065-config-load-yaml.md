# ADR-0065: config_load YAML Support

**Status:** Implemented
**Date:** 2026-07-23
**Work Order:** O-2

## Context

`config_load(path)` in v0.10.0 only supported JSON files. While JSON is ubiquitous, YAML is widely used in the AI/agent community for configuration (Docker Compose, GitHub Actions, Kubernetes, CI/CD pipelines). Agents that coordinate with external tools frequently encounter YAML config files.

## Decision

Extend `config_load` with automatic format detection by file extension:
- `.yaml` / `.yml` -> parse as YAML via `serde_yaml`
- `.json` / other -> parse as JSON (unchanged)

Implementation strategy:
1. Add `serde_yaml = "0.9"` dependency
2. Parse YAML to `serde_yaml::Value`, then convert to `serde_json::Value` via `yaml_to_json_value()` helper
3. Reuse existing `json_value_to_mlog_value_with_type()` for unified struct conversion

This avoids duplicating the struct conversion logic and keeps a single code path for the final Metalogos value construction.

### Dependencies

- `serde_yaml = "0.9"` -- note: crate is marked deprecated on crates.io but compiles and works correctly. The deprecation is about maintainer status, not functionality. For a simple config loading use case, this is acceptable.

## Consequences

- **Positive:** Metalogos programs can now load YAML configs natively. No code duplication -- YAML->JSON->Metalogos pipeline reuses existing conversion. Zero-cost for JSON users (same code path).
- **Negative:** Additional ~200KB to binary size from `serde_yaml` + `libyaml` (linked statically). YAML tags are silently dropped (only the value is preserved).
- **Neutral:** The function signature and return type are unchanged. Existing `config_load("file.json")` calls are unaffected.
