// ── tests/naryad_529_registry_bounds.rs ─────────────────────────────────
// №529 (P1, reliability/hardening; the consolidated audit 30.09 Д-5+N-6):
// the bounded registries grow the SECOND circuit — the total BYTE ceiling
// next to the №515 count ceiling, one deterministic victim order (the
// lowest id / least-recently-touched seq), loud [REGISTRY_EVICTION] lines
// + public metrics getters (the serve-report primitive).
//   T1: VIDEO count circuit — 1000 small artifacts → the store sits at
//       VIDEO_ARTIFACTS_MAX, the eviction order is deterministic.
//   T2: VIDEO byte circuit — a few LARGE artifacts overflow the byte cap
//       long before the count cap; the eviction counters move.
//   T3: VOICE artifacts byte circuit (the same shape).
//   T4: VOICE voiceprints byte circuit (embedding-accounted).
//   T5: THE HONEST ALLOWANCE — a single artifact larger than the byte
//       cap is admitted into an EMPTY store (the cap bounds accumulation,
//       not one legit artifact); the NEXT insert evicts it.
// The metrics getters are process-global counters (the house pattern:
// №517/№527's env-pinning lock) — the metrics-sensitive tests share one
// static lock so the deltas are deterministic.
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;

static METRICS_LOCK: Mutex<()> = Mutex::new(());

fn small_video(sz: usize) -> metalogos::video::VideoArtifact {
    metalogos::video::VideoArtifact {
        video_bytes: vec![0u8; sz],
        manifest: None,
        latent: None,
    }
}

fn small_audio(sz: usize) -> metalogos::voice::AudioArtifact {
    metalogos::voice::AudioArtifact {
        audio_bytes: vec![0u8; sz],
        manifest: None,
    }
}

fn big_print(mb_f32: usize) -> metalogos::voice::Voiceprint {
    metalogos::voice::Voiceprint {
        embedding: vec![0.0f32; mb_f32 * 1024 * 1024 / 4],
        model_id: "koko-ro-82m".into(),
    }
}

// ── T1: the VIDEO count circuit holds at 1000 small inserts ────────────

#[test]
fn n529_video_count_circuit_1000_small() {
    let _g = METRICS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut reg = metalogos::video::VideoRegistry::new();
    let mut first_id = None;
    for _ in 0..1000 {
        let id = reg.insert_artifact(small_video(8));
        if first_id.is_none() {
            first_id = Some(id);
        }
    }
    let (before_c, _) = metalogos::video::video_registry_eviction_metrics();
    let _ = before_c; // the counters are shared; the bound is the observable
    assert_eq!(
        reg.len(),
        metalogos::video::VIDEO_ARTIFACTS_MAX,
        "the count circuit sits at the cap"
    );
    assert!(
        reg.get_artifact(first_id.unwrap()).is_none(),
        "the first (oldest id) artifact was evicted deterministically"
    );
}

// ── T2: the VIDEO byte circuit bites before the count circuit ──────────

#[test]
fn n529_video_byte_circuit_large_artifacts() {
    let _g = METRICS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 64 MiB cap: 10 artifacts × 8 MiB = 80 MiB — the byte cap must evict
    // while the count (≤10) never reaches VIDEO_ARTIFACTS_MAX (64).
    let (c0, b0) = metalogos::video::video_registry_eviction_metrics();
    let mut reg = metalogos::video::VideoRegistry::new();
    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(reg.insert_artifact(small_video(8 * 1024 * 1024)));
    }
    let (c1, b1) = metalogos::video::video_registry_eviction_metrics();
    assert!(
        reg.len() < 10,
        "the byte circuit evicted at least one large artifact, len={}",
        reg.len()
    );
    assert!(c1 - c0 >= 1, "the eviction counter moved: {} -> {}", c0, c1);
    assert!(b1 - b0 >= 1, "the evicted-bytes counter moved");
    assert!(
        reg.get_artifact(ids[0]).is_none(),
        "the FIRST large artifact is the deterministic victim (lowest id)"
    );
    assert!(
        reg.get_artifact(*ids.last().unwrap()).is_some(),
        "the newest artifact survives"
    );
}

// ── T3: the VOICE artifacts byte circuit ────────────────────────────────

#[test]
fn n529_voice_artifact_byte_circuit() {
    let _g = METRICS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 32 MiB cap: 8 artifacts × 5 MiB = 40 MiB — the byte circuit bites.
    let (c0, _) = metalogos::voice::voice_registry_eviction_metrics();
    let mut reg = metalogos::voice::VoiceRegistry::new();
    let mut ids = Vec::new();
    for _ in 0..8 {
        ids.push(reg.insert_artifact(small_audio(5 * 1024 * 1024)));
    }
    let (c1, _) = metalogos::voice::voice_registry_eviction_metrics();
    assert!(reg.len() < 8, "the byte circuit evicted, len={}", reg.len());
    assert!(c1 > c0, "the eviction counter moved: {} -> {}", c0, c1);
    assert!(
        reg.get_artifact(ids[0]).is_none(),
        "the oldest is the victim"
    );
    assert!(reg.get_artifact(*ids.last().unwrap()).is_some());
}

// ── T4: the VOICE voiceprints byte circuit (embedding-accounted) ────────

#[test]
fn n529_voiceprint_byte_circuit() {
    let _g = METRICS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 32 MiB cap: 10 prints × 4 MiB embeddings = 40 MiB accounted.
    let (c0, _) = metalogos::voice::voice_registry_eviction_metrics();
    let mut reg = metalogos::voice::VoiceRegistry::new();
    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(reg.insert_voiceprint(big_print(4)));
    }
    let (c1, _) = metalogos::voice::voice_registry_eviction_metrics();
    assert!(
        reg.len() < 10,
        "the byte circuit evicted voiceprints, len={}",
        reg.len()
    );
    assert!(c1 > c0, "the eviction counter moved: {} -> {}", c0, c1);
    assert!(
        reg.get_voiceprint(ids[0]).is_none(),
        "the oldest print is gone"
    );
    assert!(reg.get_voiceprint(*ids.last().unwrap()).is_some());
}

// ── T5: the honest single-giant allowance into an EMPTY store ───────────

#[test]
fn n529_single_giant_artifact_admitted_into_empty_store() {
    let _g = METRICS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // One 70 MiB artifact exceeds VIDEO_ARTIFACTS_MAX_BYTES alone: the
    // store admits it (empty-store allowance) and the NEXT insert evicts
    // it deterministically (the cap bounds accumulation, not one artifact).
    let mut reg = metalogos::video::VideoRegistry::new();
    let giant = reg.insert_artifact(small_video(70 * 1024 * 1024));
    assert_eq!(reg.len(), 1, "the giant artifact is admitted alone");
    let second = reg.insert_artifact(small_video(1));
    assert!(
        reg.get_artifact(giant).is_none(),
        "the giant became the victim of the next insert"
    );
    assert!(
        reg.get_artifact(second).is_some(),
        "the small artifact lives"
    );
    assert_eq!(reg.len(), 1);
}
