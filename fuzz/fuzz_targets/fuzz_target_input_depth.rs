#![no_main]
// №958 (the audit 12.11 M-1 — gh#1925): the input-depth fuzz target.
// Arbitrary-deep inputs must NEVER abort the process: the parser
// pre-scan refuses the pathological shapes with the clean
// `[INPUT_TOO_DEEP]` error, and the DepthGuard bounds the walks. A
// crash (SIGABRT / panic) on any input is the finding.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // The parse must return (Ok or Err) — never abort, never hang
        // beyond the fuzzer's own per-input budget.
        let _ = metalogos::parser::parse(s);
    }
});
