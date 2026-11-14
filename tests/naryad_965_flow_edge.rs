// ── Naryad №965 (issue #1953, the B7 line — the owner decision 13–14.11,
// P0, [security][compiler]): the flow edge carries the step-return label.
//
// The flow OUTPUT is the implicit print of the last step's value; the
// label of that value was lost on every `->` edge (the audit's row B7 —
// five of the seven №956 delivery triples). The fix lives in the label
// lattice's flow arm (`semantic::sink_clearance_violations` → the
// `Declaration::Flow` walk): the step is the flow's call site — the
// №950 hook (б) facts re-decide with the ACTUAL edge label, the step's
// return label (the №950 (а) summary) joins into the outgoing edge, and
// the output edge is gated as the print channel it is (the frozen
// `PII_EGRESS_OUTPUT` class, no new check_id — §12.4).
//
// THE AXIS IS THE NAMED OWNER CHOICE: confidentiality only. The arm
// reads `seen.conf` and never `seen.integrity` — an UNTRUSTED-public
// flow output (model/remote data ending a flow — the `p100_mcp_echo`
// idiom) passes BY DESIGN; the integrity half is the №966 row (measured,
// not guessed).
//
// MUTATION CONTRACTS (the «сделано, когда» pins):
// - b07: delete the `Declaration::Flow` arm from
//   `sink_clearance_violations` and `n965_b07_flow_output_refuses` goes
//   green-to-red.
// - the axis: change the Flow arm to read `seen.integrity` too (gate
//   both axes) and `n965_axis_choice_confidentiality_only` goes
//   green-to-red.
// - the receipt: remove the №964 receipt table from the lattice hook and
//   `n965_receipt_stays_public_through_the_edge` goes green-to-red.

fn check_refuses(src: &str) -> bool {
    match metalogos::check_program(src) {
        Ok(result) => {
            let rendered = result
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            rendered.contains("PII_EGRESS_OUTPUT")
        }
        Err(_) => false,
    }
}

/// The b07 shape: the flow step returns the source value; the OUTPUT
/// refuses. The mutation target of the whole naryad.
#[test]
fn n965_b07_flow_output_refuses() {
    let src = "pattern P(x: String) -> String { return env(\"SP\") }\nflow Main { input: String = \"x\" -> P -> output }";
    assert!(
        check_refuses(src),
        "the flow output edge must refuse with PII_EGRESS_OUTPUT (№965)"
    );
}

/// THE AXIS CHOICE (the owner decision 13–14.11, in code as a named
/// choice): an UNTRUSTED-PUBLIC flow output passes — model/remote data
/// ending a flow is the language's primary idiom. Gating the integrity
/// axis here is exactly what №966 is measuring; this pin makes the
/// «one axis» decision enforceable.
#[test]
fn n965_axis_choice_confidentiality_only() {
    // call_llm output: (public, untrusted) — the flow output of it is
    // legal TODAY (the integrity half is №966's row).
    let src = "pattern P(x: String) -> String { return call_llm(\"summarize\", x) }\nflow Main { input: String = \"doc\" -> P -> output }";
    match metalogos::check_program(src) {
        Ok(result) => {
            let rendered = result
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !rendered.contains("PII_EGRESS_OUTPUT"),
                "the confidentiality axis must not refuse an untrusted-PUBLIC \
                 flow output (the axis choice; got: {rendered})"
            );
        }
        Err(e) => panic!("the axis-choice program must parse: {e}"),
    }
}

/// The №964 receipt contract rides the edge: the quarantine id is a
/// receipt, not the payload — the flow ending on `quarantine_write`'s
/// receipt stays public (the w1_consent_revoke shape).
#[test]
fn n965_receipt_stays_public_through_the_edge() {
    let src = "pattern R(x: String) -> String { return quarantine_write(\"q\", \"gdpr revoked\") }\nflow Main { input: String = \"tick\" -> R -> output }";
    match metalogos::check_program(src) {
        Ok(result) => {
            let rendered = result
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !rendered.contains("PII_EGRESS_OUTPUT"),
                "the quarantine receipt must ride the flow edge as public \
                 (the №964 contract; got: {rendered})"
            );
        }
        Err(e) => panic!("the receipt program must parse: {e}"),
    }
}

/// The step's param→sink facts re-decide at the EDGE with the actual
/// incoming label: a flow whose SOURCE is secret, piped into a step whose
/// body writes to a file, refuses at the flow line with the inner sink's
/// own class (the №950 hook (б) machinery applied to the pipeline).
#[test]
fn n965_step_param_sink_facts_redecide_at_the_edge() {
    let src = "pattern W(u: String) -> String { write_file(\"/tmp/n965_probe.txt\", u) return \"ok\" }\nflow Main { input: String = env(\"SP\") -> W -> output }";
    assert!(
        check_refuses(src),
        "the flow source's private label must reach the step's inner sink \
         via the edge (the hook (б) re-decision at the pipeline)"
    );
}

/// The legitimate shape stays green: a flow ending on a literal-returning
/// step never refuses (the corpus must not move).
#[test]
fn n965_public_flow_output_stays_green() {
    let src = "pattern P(x: String) -> String { return \"ok: \" + x }\nflow Main { input: String = \"job\" -> P -> output }";
    match metalogos::check_program(src) {
        Ok(result) => {
            let rendered = result
                .errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !rendered.contains("PII_EGRESS_OUTPUT"),
                "a literal flow output must never refuse (got: {rendered})"
            );
        }
        Err(e) => panic!("the public program must parse: {e}"),
    }
}

// The semantic re-export check: the flow arm lives in the label lattice's
// walk — the module surface is unchanged (the №385 vocabulary frozen).
#[test]
fn n965_vocabulary_stays_frozen() {
    // The class string is the print channel's frozen class — no new
    // check_id anywhere in the naryad.
    let src = "pattern P(x: String) -> String { return env(\"SP\") }\nflow Main { input: String = \"x\" -> P -> output }";
    let result = metalogos::check_program(src).expect("parses");
    let rendered = result
        .errors
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.contains("[PII_EGRESS_OUTPUT]"),
        "the refusal carries the frozen class (got: {rendered})"
    );
}
