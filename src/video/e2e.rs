#![cfg(feature = "video")]
// ── E2E «озвученная сцена» (Наряд №309, A.5; ADR-0151) ───────────────
//
// Cross-pillar composition E2E: reference frame → I2V video → frame
// interpolation → extension → av_mux with a real AudioId (Voice pillar
// registry) → video_export with manifest-by-construction. Runs on the №310
// tiny seeded tensors, seed-deterministic, CI-safe (CPU, milliseconds).
//
// Two layers are covered:
// 1. Library layer with a LOCAL registry — full byte-determinism
//    (fixed timestamps, no global state).
// 2. Builtin layer through the GLOBAL VIDEO/VOICE registries — the real
//    `Value`-level path the interpreter takes.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use crate::interpreter::Value;
    use crate::video::i2v::REF_PIXELS_LEN;
    use crate::video::interp::{extend_video_artifact, frame_interp_artifact};
    use crate::video::mux::{export_video, mux_av, MLGVAV_MAGIC, MLGV_MAGIC};
    use crate::video::{VideoArtifact, VideoKind, VideoRegistry, VIDEO_REGISTRY};
    use crate::voice::{AudioArtifact, VOICE_REGISTRY};

    /// Deterministic reference frame (the "first frame" of the scene).
    fn scene_ref_frame() -> Vec<f32> {
        crate::nn::attention::generate_uniform_f32(2026, REF_PIXELS_LEN, -1.0, 1.0)
    }

    /// Deterministic PCM WAV audio (the "voice-over").
    fn scene_wav() -> Vec<u8> {
        let sample_rate: u32 = 8000;
        let n_samples: usize = 4000; // 0.5 s
        let mut data = Vec::with_capacity(n_samples * 2);
        for i in 0..n_samples {
            let phase = (i % 200) as i32;
            let tri = if phase < 100 {
                -1024 + phase * 20
            } else {
                1024 - (phase - 100) * 20
            };
            data.extend_from_slice(&(tri as i16).to_le_bytes());
        }
        let data_len = data.len() as u32;
        let mut wav = Vec::with_capacity(44 + data.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&data);
        wav
    }

    /// The full «озвученная сцена» pipeline on an explicit registry —
    /// byte-deterministic (fixed timestamp, local registry, fixed audio
    /// id + bytes — the global Voice registry is exercised separately in
    /// the builtin-layer test).
    fn voiced_scene(reg: &mut VideoRegistry, audio_id: u32) -> (VideoArtifact, Vec<u8>) {
        // 1. Frame → video (I2V, first-frame anchor).
        let frame = scene_ref_frame();
        let rendered = crate::video::i2v::render(
            "wan-2.2-ti2v-5b",
            "озвученная сцена: герой открывает дверь",
            Some(&frame),
            None,
            0,
        )
        .unwrap();
        // 2. Frame interpolation (RIFE-class, 2x).
        let interp = frame_interp_artifact(&rendered, 2, 0).unwrap();
        // 3. Clip extension (+2 latent frames anchored on the last one).
        let extended = extend_video_artifact(&interp, 2, 0).unwrap();
        let vid = reg.insert_artifact(extended);
        // 4. Mux with the Voice pillar's AudioId (real PCM WAV bytes).
        let video_artifact = {
            let r = reg.get_artifact(vid).unwrap();
            r.clone()
        };
        let muxed = mux_av(&video_artifact, audio_id, &scene_wav(), 0).unwrap();
        // 5. Export with manifest by construction.
        let path = std::env::temp_dir().join("mlogos_n309_e2e_voiced_scene.mlgv");
        let exported = export_video(&muxed, &path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        (muxed, exported)
    }

    #[test]
    fn e2e_voiced_scene_library_layer() {
        let mut reg = VideoRegistry::new();
        let (muxed, exported) = voiced_scene(&mut reg, 7);

        // Provenance chain of the muxed artifact: AvMux kind, AudioId wired,
        // source provenance recorded, fps carried through.
        let m = muxed.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::AvMux);
        assert!(
            m.audio_ref.is_some(),
            "AudioId must be wired into the manifest"
        );
        assert!(
            m.source_sha.is_some(),
            "the provenance chain must reach the source"
        );
        assert_eq!(m.fps, 8);

        // Sidecar structure: magic + JSON header with frame-aligned
        // timestamps + both payloads.
        let b = &muxed.video_bytes;
        assert_eq!(&b[0..7], MLGVAV_MAGIC);
        let json_len = u32::from_le_bytes([b[7], b[8], b[9], b[10]]) as usize;
        let header: serde_json::Value = serde_json::from_slice(&b[11..11 + json_len]).unwrap();
        // T_lat: 2 → interp 2x → 3 → extend +2 → 5 latent frames
        // → 10 decoded frames; timestamps 0, 1/8, ..., 9/8.
        assert_eq!(header["frames"], 10);
        assert_eq!(header["timestamps"].as_array().unwrap().len(), 10);
        let ts = header["timestamps"].as_array().unwrap();
        assert!((ts[9].as_f64().unwrap() - 9.0 / 8.0).abs() < 1e-9);

        // Export container: magic + manifest + watermark + payload.
        assert_eq!(&exported[0..5], MLGV_MAGIC);
        let mj_len =
            u32::from_le_bytes([exported[5], exported[6], exported[7], exported[8]]) as usize;
        let manifest_json: serde_json::Value =
            serde_json::from_slice(&exported[9..9 + mj_len]).unwrap();
        assert_eq!(manifest_json["kind"], "av_mux");
        assert!(
            manifest_json["ref_hash"].is_string(),
            "the reference frame hash must survive the whole chain into the export"
        );
        assert!(manifest_json["audio_ref"].is_u64());
        assert_eq!(&exported[9 + mj_len + 16..], &muxed.video_bytes[..]);
    }

    #[test]
    fn e2e_voiced_scene_is_seed_deterministic() {
        // Same seed + same inputs ⇒ byte-identical pipeline output
        // (two independent local registries, fixed timestamps, fixed audio id).
        let mut reg1 = VideoRegistry::new();
        let mut reg2 = VideoRegistry::new();
        let (a, export_a) = voiced_scene(&mut reg1, 7);
        let (b, export_b) = voiced_scene(&mut reg2, 7);
        assert_eq!(
            a.video_bytes, b.video_bytes,
            "sidecar bytes must be identical"
        );
        assert_eq!(export_a, export_b, "export bytes must be identical");
        assert_eq!(
            a.manifest.as_ref().unwrap().video_sha,
            b.manifest.as_ref().unwrap().video_sha
        );
        assert_eq!(
            a.manifest.as_ref().unwrap().audio_ref,
            b.manifest.as_ref().unwrap().audio_ref,
            "deterministic voice registry sequencing (both runs insert exactly one audio)"
        );
    }

    #[test]
    fn e2e_voiced_scene_builtin_layer() {
        // The real Value-level path: builtins resolving handles against the
        // global registries — exactly what the interpreter calls.
        let frame = Value::List(
            scene_ref_frame()
                .into_iter()
                .map(|p| Value::Float(p as f64))
                .collect(),
        );
        let vid = crate::video::builtin_video_render(&[
            Value::String("wan-2.2-ti2v-5b".to_string()),
            Value::String("озвученная сцена: builtin path".to_string()),
            frame,
        ])
        .unwrap();
        let interp_h = crate::video::builtin_frame_interp(&[vid, Value::Float(2.0)]).unwrap();
        let ext_h = crate::video::builtin_video_extend(&[interp_h, Value::Float(2.0)]).unwrap();

        // Real AudioId from the global Voice registry.
        let audio_id = VOICE_REGISTRY
            .lock()
            .unwrap()
            .insert_artifact(AudioArtifact {
                audio_bytes: scene_wav(),
                manifest: None,
            });
        let muxed_h = crate::video::builtin_av_mux(&[ext_h, Value::Audio(audio_id)]).unwrap();

        let muxed_id = match muxed_h {
            Value::Video(v) => v,
            other => panic!("expected Value::Video, got {}", other.type_name()),
        };
        let path = std::env::temp_dir().join("mlogos_n309_e2e_builtin.mlgv");
        let exported = crate::video::builtin_video_export(&[
            muxed_h,
            Value::String(path.to_string_lossy().to_string()),
        ])
        .unwrap();
        assert!(matches!(exported, Value::String(_)));
        assert!(path.exists(), "export must write the container file");

        // The global registry holds the muxed artifact with full provenance.
        let reg = VIDEO_REGISTRY.lock().unwrap();
        let artifact = reg.get_artifact(muxed_id).unwrap();
        let m = artifact.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::AvMux);
        assert_eq!(m.audio_ref, Some(audio_id.0));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn e2e_cross_pillar_three_pillars_composition() {
        // A.6 cross-pillar summary assertion: the pipeline composes all
        // three generative pillars' value types in one flow —
        // Vision-class frame (pixels) → Video (VideoId) ← Voice (AudioId).
        let frame = scene_ref_frame();
        let rendered = crate::video::i2v::render(
            "cogvideox-1.5-5b",
            "cross-pillar composition",
            Some(&frame),
            None,
            0,
        )
        .unwrap();
        let mut reg = VideoRegistry::new();
        let vid = reg.insert_artifact(rendered);
        let audio_id = VOICE_REGISTRY
            .lock()
            .unwrap()
            .insert_artifact(AudioArtifact {
                audio_bytes: scene_wav(),
                manifest: None,
            });
        let video_artifact = reg.get_artifact(vid).unwrap().clone();
        let muxed = mux_av(&video_artifact, audio_id.0, &scene_wav(), 0).unwrap();
        // One composed artifact carries both pillar handles by construction.
        assert_eq!(muxed.manifest.as_ref().unwrap().audio_ref, Some(audio_id.0));
        assert_eq!(muxed.manifest.as_ref().unwrap().kind, VideoKind::AvMux);
    }
}
