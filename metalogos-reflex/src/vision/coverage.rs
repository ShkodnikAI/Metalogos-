//! The tensor-key coverage check (n232 Block 1) — its own leaf module
//! since №545 (b): the vision diffusion machinery (dit/vae/text_encoder)
//! moves to the metalogos-reflex crate and must NOT depend on the
//! staying weights infrastructure; the check is shared by both sides,
//! so it lives in its own dependency-free module (pure `&[String]` in,
//! `Result<(), String>` out — no candle, no Value).
//!
//! n234: accepts &[String] to support dynamic tensor names.

/// n232 Block 1: Check that the loaded tensor keys exactly match the expected set.
/// Returns Ok(()) if they match, Err with a descriptive message if there are
/// missing or extra keys.
/// n234: changed to accept &[String] to support dynamic tensor names (e.g.
/// "layers.0.attention.to_q.weight" generated from block index + schema).
pub fn check_tensor_coverage(expected: &[String], loaded: &[String]) -> Result<(), String> {
    let expected_set: std::collections::HashSet<&str> =
        expected.iter().map(|s| s.as_str()).collect();
    let loaded_set: std::collections::HashSet<&str> = loaded.iter().map(|s| s.as_str()).collect();

    let missing: Vec<&str> = expected_set.difference(&loaded_set).copied().collect();
    let extra: Vec<&str> = loaded_set.difference(&expected_set).copied().collect();

    if !missing.is_empty() || !extra.is_empty() {
        let mut msg = String::new();
        if !missing.is_empty() {
            msg.push_str(&format!(
                "Missing {} expected tensor(s): {:?}\n",
                missing.len(),
                &missing[..missing.len().min(10)]
            ));
        }
        if !extra.is_empty() {
            msg.push_str(&format!(
                "Extra {} unexpected tensor(s): {:?}\n",
                extra.len(),
                &extra[..extra.len().min(10)]
            ));
        }
        return Err(msg);
    }
    Ok(())
}
