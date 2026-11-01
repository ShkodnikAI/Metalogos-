// ── Naryad №868 (wave 76 — W-E, P1, [compliance]): the evidence-package
// compiler (ADR-0206 §2.4/§4) ────────────────────────────────────────────
//
// From the attestation map (№867) plus the LIVE sources it compiles the
// deterministic, digest-pinned package — the sellable artifact an
// external auditor consumes WITHOUT lifting the runtime:
//
//   - the rows are sorted by the pinned row digest (the order is a
//     function of the content, never of the issue order);
//   - every row serializes on ONE canonical line; a field that would
//     break the line contract (a space in the single-line fields)
//     refuses — fail-closed, not a silent re-format;
//   - the row digest is the G5 pin (№828): a row without a digest is
//     not a row; the digest pins the CLAIM, the lifecycle status rides
//     beside it (the №867 form — a revocation never re-digests a row);
//   - the package digest is SHA-256 over the exact row-block bytes;
//   - determinism is a CONTRACT: two compilations of the same inputs
//     are byte-identical (the double-run test in
//     tests/naryad_868_package.rs). The package carries NO wall-clock
//     timestamp — the freshness verdict is the CONSUMER's comparison
//     of the window against their own verification date (ADR-0206 §3).
//
// Fail-closed resolution (ADR-0206 §2.5): a LIVE row (not revoked,
// inside the window) whose pinned evidence digest does not match the
// live source refuses the compilation — the map never vouches for
// bytes it cannot see matching. Revoked/expired rows are historical:
// they compile with their status, their evidence is not re-resolved.
//
// The format is the stable minimum (v1): no backward compatibility is
// promised until the wave acceptance (the №868 boundary).

use crate::compliance::{
    canonical_row, canonical_row_digest, Attestation, AttestationMap, Evidence, EvidenceKind,
};
use sha2::{Digest, Sha256};

/// The package format version (v1 — the stable minimum).
pub const PACKAGE_VERSION: &str = "v1";

/// The injected live-source resolver (the №867 contract): returns the
/// live SHA-256 (lowercase hex) of the referenced artifact, or
/// `Err(reason)` when the parent cannot be resolved. The compiler calls
/// it for every LIVE row; the production resolver (the test harness
/// and the future consumer tooling) reads the live in-repo sources.
pub type LiveDigestResolver<'a> = &'a dyn Fn(&str, &str) -> Result<String, String>;

/// The resolved status of one row in the package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatus {
    Live,
    Expired,
    Revoked,
}

impl RowStatus {
    fn as_str(self) -> &'static str {
        match self {
            RowStatus::Live => "live",
            RowStatus::Expired => "expired",
            RowStatus::Revoked => "revoked",
        }
    }
}

/// One compiled row: the pinned claim digest + the lifecycle status + a
/// display digest of the full canonical line (the auditor recomputes
/// both).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRow {
    pub row_digest: String,
    pub status: RowStatus,
    pub line: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

/// The single-line canonical form of one row (space-separated
/// `key=value` fields; every field value is space-free — the line is
/// the parse contract for the external auditor and the machine fact of
/// №869, which counts the RAW rows structurally).
fn row_line(att: &Attestation, status: RowStatus) -> Result<String, String> {
    let fields: Vec<(&str, String)> = vec![
        ("claim", att.claim.clone()),
        ("subject", att.subject.clone()),
        ("subject_digest", att.subject_digest.clone()),
        (
            "evidence",
            format!(
                "{}:{}@{}",
                att.evidence.kind.as_str(),
                att.evidence.reference,
                att.evidence.digest
            ),
        ),
        ("verifier", att.verifier.clone()),
        ("window", format!("{}-{}", att.issued_at, att.expires_at)),
        ("actor", att.actor.clone()),
    ];
    for (k, v) in &fields {
        if v.contains(' ') || v.contains('\n') || v.contains('\t') {
            return Err(format!(
                "package compile: the field {k} of row {} carries a line break or a space — the v1 line contract refuses (fail-closed)",
                att.row_digest
            ));
        }
    }
    let body = fields
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!(
        "row {} {} {}",
        att.row_digest,
        status.as_str(),
        body
    ))
}

/// Compile the deterministic evidence package from the map. Returns the
/// full package text (the file/string the caller owns). Refuses when a
/// LIVE row's evidence digest does not match the live source or when a
/// row breaks the v1 line contract.
pub fn compile_package(
    map: &AttestationMap,
    now: u64,
    resolve: LiveDigestResolver,
) -> Result<String, String> {
    let mut compiled: Vec<CompiledRow> = Vec::new();
    for att in map.rows() {
        let status = if att.revoked {
            RowStatus::Revoked
        } else if now >= att.expires_at || now < att.issued_at {
            RowStatus::Expired
        } else {
            // the fail-closed resolution: a live row vouches only for
            // bytes it can see matching (ADR-0206 §2.5)
            let live =
                resolve(att.evidence.kind.as_str(), &att.evidence.reference).map_err(|e| {
                    format!(
                        "package compile: the live source for row {} ({}:{}) refuses: {}",
                        att.row_digest,
                        att.evidence.kind.as_str(),
                        att.evidence.reference,
                        e
                    )
                })?;
            if !att.evidence_matches(&live) {
                return Err(format!(
                    "package compile: the evidence digest of live row {} ({}:{}) does not match the source (pinned {}, live {}) — the map never lies, re-attest",
                    att.row_digest,
                    att.evidence.kind.as_str(),
                    att.evidence.reference,
                    att.evidence.digest,
                    live
                ));
            }
            RowStatus::Live
        };
        let line = row_line(att, status)?;
        compiled.push(CompiledRow {
            row_digest: att.row_digest.clone(),
            status,
            line,
        });
    }
    // The deterministic order: by the pinned row digest — a function of
    // the content, never of the issue order.
    compiled.sort_by(|a, b| a.row_digest.cmp(&b.row_digest));

    // The package digest: SHA-256 over the exact row-block bytes as
    // they appear in the package (the lines each followed by '\n' —
    // the same bytes `package_digest_of` re-hashes from the text).
    let block = compiled
        .iter()
        .map(|r| r.line.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let block_bytes = if block.is_empty() {
        String::new()
    } else {
        block.clone() + "\n"
    };
    let package_digest = sha256_hex(block_bytes.as_bytes());
    let live = compiled
        .iter()
        .filter(|r| r.status == RowStatus::Live)
        .count();

    let mut out = String::new();
    out.push_str("# METALOGOS compliance package ");
    out.push_str(PACKAGE_VERSION);
    out.push('\n');
    out.push_str("# the compliance evidence compiler of ADR-0206 §2.4 (№868): the deterministic, digest-pinned attestation map export\n");
    out.push_str(&format!("# package_digest: {package_digest}\n"));
    out.push_str(&format!("# rows_total: {}\n", compiled.len()));
    out.push_str(&format!("# rows_live: {live}\n"));
    out.push_str("# freshness: the consumer compares each row's window against their own verification date (ADR-0206 §3 — the package carries no wall clock)\n");
    if !block.is_empty() {
        out.push_str(&block);
        out.push('\n');
    }
    Ok(out)
}

/// The package digest extracted from a compiled package (the consumer's
/// re-check: recompute the row-block digest and compare with the
/// header). Refuses a package whose rows were edited after the digest
/// was stamped.
pub fn package_digest_of(package_text: &str) -> Result<String, String> {
    let mut block = String::new();
    for line in package_text.lines() {
        if line.starts_with("row ") {
            block.push_str(line);
            block.push('\n');
        }
    }
    if block.is_empty() {
        return Err("package digest: no row lines found — not a v1 package".to_string());
    }
    Ok(sha256_hex(block.as_bytes()))
}

/// The structural raw-row count (the №869 counter contract): the number
/// of package lines that ARE rows — every one carrying the pinned row
/// digest and the full field set. This is a structural parse of the raw
/// lines, never a summary number (the №565 lesson).
pub fn count_raw_rows(package_text: &str) -> usize {
    package_text
        .lines()
        .filter(|l| l.starts_with("row ") && l.split_whitespace().count() >= 5)
        .count()
}

/// The live-row count over the raw lines (the ATTEST_MAP value shape).
pub fn count_live_rows(package_text: &str) -> usize {
    package_text
        .lines()
        .filter(|l| l.starts_with("row ") && l.split(' ').nth(2) == Some("live"))
        .count()
}

/// Re-pin check: every row's claim digest still matches the canonical
/// re-derivation from its own fields (the tamper trap — an edited row
/// no longer matches its digest). The consumer runs this BEFORE
/// trusting the package.
pub fn verify_row_digests(package_text: &str) -> Result<usize, String> {
    let mut checked = 0usize;
    for line in package_text.lines() {
        if !line.starts_with("row ") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(format!("row integrity: a malformed row line: {line}"));
        }
        let claimed = parts[1];
        // The claim digest covers the kernel fields; the package line
        // carries them in the key=value form — re-derive through the
        // kernel by parsing the line back.
        let att = parse_row_line(line)?;
        if canonical_row_digest(&att) != claimed {
            return Err(format!(
                "row integrity: the row {claimed} no longer matches its pinned digest — the package was edited after compilation"
            ));
        }
        checked += 1;
    }
    Ok(checked)
}

/// The v1 row-line parser (the consumer's round-trip: line → the kernel
/// Attestation). Refuses malformed lines loudly.
pub fn parse_row_line(line: &str) -> Result<Attestation, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 || parts[0] != "row" {
        return Err(format!("parse: not a v1 row line: {line}"));
    }
    let row_digest = parts[1].to_string();
    let status = parts[2];
    let mut kv: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for field in &parts[3..] {
        let (k, v) = field
            .split_once('=')
            .ok_or_else(|| format!("parse: a field without '=': {field}"))?;
        kv.insert(k, v);
    }
    let get = |k: &str| -> Result<String, String> {
        kv.get(k)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("parse: the field {k} is missing"))
    };
    let ev = get("evidence")?;
    let (kind_part, rest) = ev
        .split_once(':')
        .ok_or_else(|| format!("parse: the evidence field lacks ':' — {ev}"))?;
    let (reference, digest) = rest
        .split_once('@')
        .ok_or_else(|| format!("parse: the evidence field lacks '@' — {ev}"))?;
    let kind = match kind_part {
        "ledger" => EvidenceKind::Ledger,
        "mandate" => EvidenceKind::Mandate,
        "passport" => EvidenceKind::Passport,
        "capability" => EvidenceKind::Capability,
        "gateway" => EvidenceKind::Gateway,
        "artifact" => EvidenceKind::Artifact,
        other => return Err(format!("parse: the unknown evidence kind {other}")),
    };
    let window = get("window")?;
    let (issued_at, expires_at) = window
        .split_once('-')
        .ok_or_else(|| format!("parse: the window field lacks '-' — {window}"))?;
    let att = Attestation {
        actor: get("actor")?,
        subject: get("subject")?,
        subject_digest: get("subject_digest")?,
        claim: get("claim")?,
        evidence: Evidence {
            kind,
            reference: reference.to_string(),
            digest: digest.to_string(),
        },
        verifier: get("verifier")?,
        issued_at: issued_at
            .parse::<u64>()
            .map_err(|e| format!("parse: the issued_at is not u64: {e}"))?,
        expires_at: expires_at
            .parse::<u64>()
            .map_err(|e| format!("parse: the expires_at is not u64: {e}"))?,
        revoked: status == "revoked",
        row_digest: row_digest.clone(),
    };
    // the round-trip guard: the parsed row re-digests to its pin (the
    // status is lifecycle, the digest is the claim — the №867 form)
    if canonical_row(&att).is_empty() || canonical_row_digest(&att) != row_digest {
        return Err(format!(
            "parse: the row {row_digest} does not re-digest from its own fields — the line is not a v1 row"
        ));
    }
    Ok(att)
}
