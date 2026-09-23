//! Backend registry (Наряд №333, ADR-0163) — classes, SHA-pin contract,
//! license classes and the distribution gate data.
//!
//! ## ADR-0163 — the SSOT for what a backend IS
//!
//! One static table (`BACKEND_REGISTRY`), spec!-style: every entry is a
//! plain struct literal statically visible to the audit (the №316
//! classification precedent). Programs reference backends by identifier;
//! the license gate matches those identifiers at string-literal positions
//! (the `MODEL_WEIGHTS_UNSAFE` literal-URL precedent) plus the
//! `vision { model: … }` declaration field.
//!
//! ## The SHA-pin boundary is a TYPE, not a placeholder
//!
//! `ShaPin::Pinned(hash)` = the expected SHA-256 of the weights artifact,
//! verified at fetch/load time (the №334 contract). `ShaPin::PendingNo334`
//! = the weights artifact is NOT vendored in-tree (real-weights runs are
//! PARKED by hardware, №294) — there is NO hash to state, and fabricating
//! one would be a lie. №334 replaces every `PendingNo334` with a real pin
//! and refuses to LOAD `PendingNo334` entries.
//!
//! ## License classes are claims with named bases
//!
//! `Osi` / `NonOsi` / `Restrictive`, each entry carrying `license_note`
//! naming the license and its basis. Legal fine-reading of specific
//! licenses is OUT of scope (the issue's loud boundary): classes only.
//! Unverified licenses are **restrictive by default-deny** — the
//! `MODEL_WEIGHTS_UNSAFE` allowlist posture.

/// The §7.6 MDL model classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendClass {
    Stt,
    Tts,
    Omni,
    VisionUnderstanding,
    Llm,
    /// Optical character recognition — text extraction FROM an image
    /// (Naryad №407, wave 4.5). Reading/understanding, NOT generation:
    /// no Art. 50 synthetic marking on the output (the №331/№332
    /// provenance discipline applies unchanged).
    Ocr,
    /// Video comprehension — answering questions about a video segment
    /// (Naryad №408, wave 4.5). Reading/understanding, NOT generation:
    /// no Art. 50 synthetic marking on the output (the №331/№332
    /// provenance discipline applies unchanged); real inference is
    /// PARKED by hardware (№294) — the mock-first call surface is live.
    VideoUnderstanding,
    /// The embodied sim contour (Naryad №355, registry В5 — Phase 5
    /// «Embodied, sim-only», ADR-0159). In-tree deterministic
    /// simulators and mock device records — NO external weights
    /// artifact exists, so the №334 pin contract does not apply (the
    /// entries declare PendingNo334 honestly and are never loaded
    /// through the weights loader). The contour is sim-only: no real
    /// hardware path (the №294 hardware gate + the GPU-budget gate).
    EmbodiedSim,
}

impl BackendClass {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendClass::Stt => "stt",
            BackendClass::Tts => "tts",
            BackendClass::Omni => "omni",
            BackendClass::VisionUnderstanding => "vision-understanding",
            BackendClass::Llm => "llm",
            BackendClass::Ocr => "ocr",
            BackendClass::VideoUnderstanding => "video-understanding",
            BackendClass::EmbodiedSim => "embodied-sim",
        }
    }

    /// Parse a §7.6 class word (the `backend_select` ladder's class
    /// argument, №336). Unknown words are loud at both check time
    /// (semantic companion, ADR-0165 §2.4) and run time.
    pub fn parse(word: &str) -> Option<BackendClass> {
        match word {
            "stt" => Some(BackendClass::Stt),
            "tts" => Some(BackendClass::Tts),
            "omni" => Some(BackendClass::Omni),
            "vision-understanding" => Some(BackendClass::VisionUnderstanding),
            "llm" => Some(BackendClass::Llm),
            "ocr" => Some(BackendClass::Ocr),
            "video-understanding" => Some(BackendClass::VideoUnderstanding),
            "embodied-sim" => Some(BackendClass::EmbodiedSim),
            _ => None,
        }
    }
}

/// License class for distribution governance (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseClass {
    /// OSI-approved license (MIT, Apache-2.0, …) — distributable by default.
    Osi,
    /// Source-available but NOT OSI-approved (e.g. NVIDIA Open Model
    /// License) — forbidden in the distribution profile.
    NonOsi,
    /// Restrictive / unverified — forbidden by default-deny until a
    /// license record lands.
    Restrictive,
}

impl LicenseClass {
    pub fn as_str(self) -> &'static str {
        match self {
            LicenseClass::Osi => "osi",
            LicenseClass::NonOsi => "non-osi",
            LicenseClass::Restrictive => "restrictive",
        }
    }
}

/// The SHA-pin contract (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaPin {
    /// Expected SHA-256 of the weights artifact (hex, lowercase),
    /// verified at fetch/load time — the №334 contract.
    Pinned(&'static str),
    /// Weights not vendored in-tree (PARKED №294) — no hash to state.
    /// №334 replaces this variant with a real pin and REFUSES to load
    /// it. Loud, typed, grep-visible: never a fabricated hash.
    PendingNo334,
}

impl ShaPin {
    /// Report form (`backend_list` / audit messages).
    pub fn as_str(self) -> &'static str {
        match self {
            ShaPin::Pinned(h) => h,
            ShaPin::PendingNo334 => "pending (naryad №334 sha-pin path)",
        }
    }
}

/// One backend registry entry (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendEntry {
    /// Program-visible backend identifier.
    pub name: &'static str,
    pub class: BackendClass,
    /// Weights artifact identifier — the gate's matching key.
    pub weights_id: &'static str,
    pub pin: ShaPin,
    pub license: LicenseClass,
    /// The license and its classification basis (one line).
    pub license_note: &'static str,
}

/// The backend SSOT (ADR-0163 §2.1). Seed entries — the models the tree
/// and plan §15 already name; the list is reported loudly in the naryad
/// report. New backends (№334+) join HERE, never bypass the registry.
pub const BACKEND_REGISTRY: &[BackendEntry] = &[
    BackendEntry {
        name: "chatterbox",
        class: BackendClass::Tts,
        weights_id: "chatterbox-multilingual-v3",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Chatterbox (Resemble AI) — MIT (osi)",
    },
    BackendEntry {
        name: "kokoro",
        class: BackendClass::Tts,
        weights_id: "koko-ro-82m",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Kokoro-82M — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "z-image-turbo",
        class: BackendClass::VisionUnderstanding,
        weights_id: "z-image-turbo",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Z-Image Turbo (Tongyi) — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "molmoact2",
        class: BackendClass::VisionUnderstanding,
        weights_id: "molmoact2",
        // №334: real pin — HF LFS oid (sha256) of the primary shard of
        // allenai/MolmoAct2, fetched via the HF tree API 2026-09-16 (the
        // full per-file manifest is WEIGHTS_SOURCES below — the loader
        // verifies EVERY shard, not only the primary one).
        pin: ShaPin::Pinned("512674fc34842123fd4405fc72143bf8d48ee71165b0dc59e666132ca9447dc9"),
        license: LicenseClass::Osi,
        license_note: "MolmoAct2 (AllenAI lineage) — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "wall-oss",
        class: BackendClass::Omni,
        weights_id: "wall-oss-0.5",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Restrictive,
        license_note: "license NOT verified in-tree — restrictive by default-deny (allowlist posture, ADR-0163 §2.1) until a license record lands",
    },
    BackendEntry {
        name: "nemotron-omni",
        class: BackendClass::Omni,
        weights_id: "nemotron-3-nano-omni-30b-a3b",
        // №334: real pin — HF LFS oid (sha256) of the primary shard of
        // nvidia/Nemotron-3-Nano-Omni-30B-A3B-Reasoning-BF16, fetched via
        // the HF tree API 2026-09-16 (full per-file manifest below).
        pin: ShaPin::Pinned("de952574c9189925ad15f8cf164184117b6e5eec2d8b7f092e1268c1f0872244"),
        license: LicenseClass::NonOsi,
        license_note: "Nemotron — NVIDIA Open Model License (non-osi; the MDL-3 test case)",
    },
    // №334: the STT class joins the registry — the canon ASR wedge
    // (single-file artifact, the whole pin IS the artifact hash).
    BackendEntry {
        name: "whisper-turbo",
        class: BackendClass::Stt,
        weights_id: "whisper-large-v3-turbo",
        pin: ShaPin::Pinned("542566a422ae4f3fd23f1ba11add198fca01bbf82e66e6a2857b3f608b1eb9d1"),
        license: LicenseClass::Osi,
        license_note: "Whisper large-v3-turbo (OpenAI) — MIT (osi)",
    },
    // №407 (wave 4.5): the OCR class joins the registry — the canon
    // printed-text wedge (single-artifact manifest, the whisper-pin
    // pattern №334: the registry pin IS the primary artifact's hash).
    BackendEntry {
        name: "trocr-printed",
        class: BackendClass::Ocr,
        weights_id: "trocr-base-printed",
        // №407: real pin — HF LFS oid (sha256) of the primary artifact
        // of microsoft/trocr-base-printed, fetched via the HF tree API
        // 2026-09-20 (the full per-file manifest is WEIGHTS_SOURCES
        // below — the loader verifies every manifest file).
        pin: ShaPin::Pinned("1cf4a6eedab26afaaf505f1c7f73d9634944924dbd1ed049d569db98039cd596"),
        license: LicenseClass::Osi,
        license_note: "TrOCR base-printed (Microsoft) — MIT (osi)",
    },
    // №408 (wave 4.5): the video-understanding class joins the registry —
    // the canon video-comprehension wedge. Pins are REAL HF LFS oids
    // (sha256) of the primary shard of each repo, fetched via the HF tree
    // API 2026-09-20 (full per-shard manifests in WEIGHTS_SOURCES below —
    // the loader verifies EVERY shard). License classification follows
    // the ACTUAL repo declaration (ADR-0163 §2.1): all three declare
    // Apache-2.0 in the HF card metadata (cardData + tags); the naryad's
    // "InternVL3 (MIT)" assumption was STALE — the card declares
    // apache-2.0 and the repo ships no separate LICENSE text, recorded
    // here per the actual declaration, not the assumption.
    BackendEntry {
        name: "qwen25-vl-7b",
        class: BackendClass::VideoUnderstanding,
        weights_id: "qwen2.5-vl-7b-instruct",
        pin: ShaPin::Pinned("e97b877e47fde53a6c6e77aafb36e58e91ee9d95c4a3eeac6f1b5c0e6a1c986e"),
        license: LicenseClass::Osi,
        license_note: "Qwen2.5-VL-7B-Instruct (Qwen) — Apache-2.0 (osi; HF cardData + tags)",
    },
    BackendEntry {
        name: "llava-video-7b",
        class: BackendClass::VideoUnderstanding,
        weights_id: "llava-video-7b-qwen2",
        pin: ShaPin::Pinned("2625213dd97a944180a7ba6776501709f6094e9c07295101518d69ca8ccfb5ad"),
        license: LicenseClass::Osi,
        license_note: "LLaVA-Video-7B-Qwen2 (lmms-lab) — Apache-2.0 (osi; HF cardData + tags)",
    },
    BackendEntry {
        name: "internvl3-8b",
        class: BackendClass::VideoUnderstanding,
        weights_id: "internvl3-8b",
        pin: ShaPin::Pinned("7ea1f92eaae35cb927e7c7b0f87568ccc046a2446aa68066dfdccb7ebbe0c7f0"),
        license: LicenseClass::Osi,
        license_note: "InternVL3-8B (OpenGVLab) — Apache-2.0 per the HF card declaration (osi; cardData + tags; the naryad's MIT assumption was stale — no separate LICENSE text ships in the repo)",
    },
    // №355 (wave 10, registry В5 — Phase 5 «Embodied, sim-only»,
    // ADR-0159): the embodied-sim class joins the registry — the sim/
    // mock device records the contour runs on. Honest entries: the
    // simulators are IN-TREE deterministic code — no external weights
    // artifact exists, so there is NOTHING to fetch or pin
    // (PendingNo334 declared loudly; never routed through the weights
    // loader), and the governing license is the repository's own
    // (MIT OR Apache-2.0 — both osi). The contour is sim-only: no real
    // hardware path exists (№294 + the GPU-budget gate, ADR-0159 §3.1).
    BackendEntry {
        name: "embodied-sim-kinematic",
        class: BackendClass::EmbodiedSim,
        weights_id: "in-tree-kinematics-sim",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "in-tree deterministic kinematics simulator — no external weights artifact exists (nothing to fetch or pin; the №334 loader never routes sim records); governed by the repository license (MIT OR Apache-2.0 — osi)",
    },
    BackendEntry {
        name: "embodied-mock-device",
        class: BackendClass::EmbodiedSim,
        weights_id: "in-tree-mock-device",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "in-tree mock device record (trace-recording contour, no kinematics) — no external weights artifact exists (nothing to fetch or pin; the №334 loader never routes sim records); governed by the repository license (MIT OR Apache-2.0 — osi)",
    },
];

/// Find a registry entry by weights identifier (exact, case-sensitive —
/// the identifiers are lowercase canon; gate matches are case-insensitive
/// and go through `find_by_weights_id_ci`).
pub fn find_by_weights_id(id: &str) -> Option<&'static BackendEntry> {
    BACKEND_REGISTRY.iter().find(|e| e.weights_id == id)
}

/// Case-insensitive weights-id match (the gate's key: a literal in ANY
/// casing names the same weights).
pub fn find_by_weights_id_ci(id: &str) -> Option<&'static BackendEntry> {
    let lower = id.to_lowercase();
    BACKEND_REGISTRY.iter().find(|e| e.weights_id == lower)
}

/// Find by the program-visible backend name.
pub fn find_by_name(name: &str) -> Option<&'static BackendEntry> {
    BACKEND_REGISTRY.iter().find(|e| e.name == name)
}

// ── №334: the per-file weights manifest (the SHA-pin path) ───────────
//
// The registry pin names the PRIMARY artifact; the loader verifies EVERY
// file against the manifest below. Values are REAL: HF LFS oid == SHA-256
// of the file (the №212 manifest discipline), fetched from the HF tree
// API on 2026-09-16; provenance (repo + file) is in every path. A pin or
// manifest entry without a hash NEVER happens — `validate_weights_source`
// refuses it loudly, and a fabricated hash would be a lie (ADR-0163 §2.1).

/// One file of a weights manifest: the repo-relative path, the expected
/// SHA-256 (hex, lowercase — HF LFS oid), and the expected byte count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightsFile {
    pub path: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
}

/// The full weights source of one backend: the HF repo id (canonical,
/// case as on HF) plus every file the loader must verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightsSource {
    /// The Hugging Face repo id, e.g. "openai/whisper-large-v3-turbo".
    pub repo: &'static str,
    /// The revision the pins were verified against (never floating).
    pub revision: &'static str,
    pub files: &'static [WeightsFile],
}

/// The manifest SSOT, keyed by weights_id (the registry's artifact id).
/// Only the №334-scoped backends (STT, omni, vision-understanding) and
/// the №407 OCR canon carry sources; TTS/z-image/wall-oss stay
/// PendingNo334 — hashes for gated or unfilled-manifest artifacts are
/// not invented, they are fetched.
pub const WEIGHTS_SOURCES: &[(&str, WeightsSource)] = &[
    (
        "trocr-base-printed",
        WeightsSource {
            repo: "microsoft/trocr-base-printed",
            revision: "main",
            files: &[WeightsFile {
                path: "model.safetensors",
                sha256: "1cf4a6eedab26afaaf505f1c7f73d9634944924dbd1ed049d569db98039cd596",
                bytes: 1_333_384_464,
            }],
        },
    ),
    (
        "qwen2.5-vl-7b-instruct",
        WeightsSource {
            repo: "Qwen/Qwen2.5-VL-7B-Instruct",
            revision: "main",
            files: &[
                WeightsFile {
                    path: "model-00001-of-00005.safetensors",
                    sha256: "e97b877e47fde53a6c6e77aafb36e58e91ee9d95c4a3eeac6f1b5c0e6a1c986e",
                    bytes: 3_900_233_256,
                },
                WeightsFile {
                    path: "model-00002-of-00005.safetensors",
                    sha256: "a9a300a43b4724eee2abe7c18ceb26768d0ab011eb0cad19d9bfd2476a24d024",
                    bytes: 3_864_726_320,
                },
                WeightsFile {
                    path: "model-00003-of-00005.safetensors",
                    sha256: "111223d173e00bbee81cba1216fad28668df3476706b7fd26f4d5b50f8b3a507",
                    bytes: 3_864_726_424,
                },
                WeightsFile {
                    path: "model-00004-of-00005.safetensors",
                    sha256: "ef47f634fa57d46ee134edcc09f34085a47da1e16c12a2abe0d67118be6d72ed",
                    bytes: 3_864_733_680,
                },
                WeightsFile {
                    path: "model-00005-of-00005.safetensors",
                    sha256: "0c859795ad3a627a9b95bcb762e059d5b768a4a36fdd4affeff269d93fdecc67",
                    bytes: 1_089_994_880,
                },
            ],
        },
    ),
    (
        "llava-video-7b-qwen2",
        WeightsSource {
            repo: "lmms-lab/LLaVA-Video-7B-Qwen2",
            revision: "main",
            files: &[
                WeightsFile {
                    path: "model-00001-of-00004.safetensors",
                    sha256: "2625213dd97a944180a7ba6776501709f6094e9c07295101518d69ca8ccfb5ad",
                    bytes: 4_877_668_032,
                },
                WeightsFile {
                    path: "model-00002-of-00004.safetensors",
                    sha256: "bc4fadb1419522b41c58d36f46e29d6f0cba5fcb35b74c0d1887692852f3d98b",
                    bytes: 4_932_751_008,
                },
                WeightsFile {
                    path: "model-00003-of-00004.safetensors",
                    sha256: "94aca0e44c71e4640b40cdbadd995a234937168c72c552e0d2d13036ca96cc68",
                    bytes: 4_994_571_904,
                },
                WeightsFile {
                    path: "model-00004-of-00004.safetensors",
                    sha256: "3cbb8d7cfb44f868dd672a11b677dadcbde5428dc744a23f4393508aaeb1f22d",
                    bytes: 1_255_812_224,
                },
            ],
        },
    ),
    (
        "internvl3-8b",
        WeightsSource {
            repo: "OpenGVLab/InternVL3-8B",
            revision: "main",
            files: &[
                WeightsFile {
                    path: "model-00001-of-00004.safetensors",
                    sha256: "7ea1f92eaae35cb927e7c7b0f87568ccc046a2446aa68066dfdccb7ebbe0c7f0",
                    bytes: 4_991_123_960,
                },
                WeightsFile {
                    path: "model-00002-of-00004.safetensors",
                    sha256: "05f9f1bf63d946d2963bd68c9d1ee7f93bdc5fc05a9b39a3fb91734d5ef4362d",
                    bytes: 4_958_443_072,
                },
                WeightsFile {
                    path: "model-00003-of-00004.safetensors",
                    sha256: "904aa0f2fbdbf504cd7bce1a8ec5633ee4e3e1769b48c5b3e08369a23875aaa1",
                    bytes: 4_796_984_024,
                },
                WeightsFile {
                    path: "model-00004-of-00004.safetensors",
                    sha256: "50e048a88254db95e0aa397a4e4012a353eaf650a18d7b291e1522d16e83d389",
                    bytes: 1_142_280_864,
                },
            ],
        },
    ),
    (
        "whisper-large-v3-turbo",
        WeightsSource {
            repo: "openai/whisper-large-v3-turbo",
            revision: "main",
            files: &[WeightsFile {
                path: "model.safetensors",
                sha256: "542566a422ae4f3fd23f1ba11add198fca01bbf82e66e6a2857b3f608b1eb9d1",
                bytes: 1_617_824_864,
            }],
        },
    ),
    (
        "nemotron-3-nano-omni-30b-a3b",
        WeightsSource {
            repo: "nvidia/Nemotron-3-Nano-Omni-30B-A3B-Reasoning-BF16",
            revision: "main",
            files: &[
                WeightsFile {
                    path: "model-00001-of-00017.safetensors",
                    sha256: "de952574c9189925ad15f8cf164184117b6e5eec2d8b7f092e1268c1f0872244",
                    bytes: 3_996_912_760,
                },
                WeightsFile {
                    path: "model-00002-of-00017.safetensors",
                    sha256: "5b2707cbe0e3bea9f67f06980184848ecad2fdadcf80cbb94d0b2d9a859a9943",
                    bytes: 3_999_538_784,
                },
                WeightsFile {
                    path: "model-00003-of-00017.safetensors",
                    sha256: "f315ecba2cd3d0233ecef2b02b84c38fde609775ccf32c2f255094033318b038",
                    bytes: 3_994_808_632,
                },
                WeightsFile {
                    path: "model-00004-of-00017.safetensors",
                    sha256: "253fbbe376a8224f46b4e2c7c479e75b753d37077c014d27f04f4f84014e5bd3",
                    bytes: 3_999_538_936,
                },
                WeightsFile {
                    path: "model-00005-of-00017.safetensors",
                    sha256: "3c025df3ffeaf0dac38b816522a5cb740b8e79c46bd27b886b5bc370c87c242b",
                    bytes: 3_994_809_040,
                },
                WeightsFile {
                    path: "model-00006-of-00017.safetensors",
                    sha256: "24a7ba72f08e647c2c78b0264ad4bc4f78e3d70e405e5dda50c059333ebdb3d2",
                    bytes: 3_999_539_160,
                },
                WeightsFile {
                    path: "model-00007-of-00017.safetensors",
                    sha256: "071fd79ae9d1d05da92101b89929ff169d98ea72cadc504b42781740a704a4db",
                    bytes: 3_994_809_064,
                },
                WeightsFile {
                    path: "model-00008-of-00017.safetensors",
                    sha256: "307f0422e80c02b7d44bce7cee627959a9c7a49f8240b4f8aafa6ec608f51587",
                    bytes: 3_999_539_152,
                },
                WeightsFile {
                    path: "model-00009-of-00017.safetensors",
                    sha256: "b07c93f94210b721c60c470e305bff1a50fbe285878c28cf2abe610a5ce76fe9",
                    bytes: 3_994_809_088,
                },
                WeightsFile {
                    path: "model-00010-of-00017.safetensors",
                    sha256: "4eb2ae5f5c918f25aa5009627fda26ffd29cfd0bec216690904002a38bed1011",
                    bytes: 3_999_539_128,
                },
                WeightsFile {
                    path: "model-00011-of-00017.safetensors",
                    sha256: "aa28bbab86804af65b12776b67ec54ddd7857dfca439599e0c1dcd1d819c8d1b",
                    bytes: 3_982_766_872,
                },
                WeightsFile {
                    path: "model-00012-of-00017.safetensors",
                    sha256: "1804bf9755ee431cf7bd65773a4d0640b89ba76efc58c57de63eb8ab98a05dca",
                    bytes: 3_991_625_352,
                },
                WeightsFile {
                    path: "model-00013-of-00017.safetensors",
                    sha256: "78c70bebd7a0fcde5eb78f6a08a2d6da923d933bfc53853946e207fa75f1a271",
                    bytes: 3_970_314_024,
                },
                WeightsFile {
                    path: "model-00014-of-00017.safetensors",
                    sha256: "8261526b09413eacc1807f92b886df0bcc056dc27015abd156913aed66c8782b",
                    bytes: 3_994_100_200,
                },
                WeightsFile {
                    path: "model-00015-of-00017.safetensors",
                    sha256: "442006b0a48b3652770f5bf95e6677ebff512990736587073ac53cb6cc1e7bc6",
                    bytes: 3_999_539_368,
                },
                WeightsFile {
                    path: "model-00016-of-00017.safetensors",
                    sha256: "bc699ab1095fdccad1ec8bfd798128ab29dad3cc3e82e04dd39470a5daf58840",
                    bytes: 3_997_900_128,
                },
                WeightsFile {
                    path: "model-00017-of-00017.safetensors",
                    sha256: "d34d8b0e21f53e5ea6196339e826c62dc6ef6ef2a598d4149fc1b5cbe699d367",
                    bytes: 2_122_218_848,
                },
            ],
        },
    ),
    (
        "molmoact2",
        WeightsSource {
            repo: "allenai/MolmoAct2",
            revision: "main",
            files: &[
                WeightsFile {
                    path: "model-00001-of-00005.safetensors",
                    sha256: "512674fc34842123fd4405fc72143bf8d48ee71165b0dc59e666132ca9447dc9",
                    bytes: 4_919_324_120,
                },
                WeightsFile {
                    path: "model-00002-of-00005.safetensors",
                    sha256: "d3c335f3291604d25e2092e0a22441752949745050c894d18537394f8f66ac84",
                    bytes: 4_844_690_992,
                },
                WeightsFile {
                    path: "model-00003-of-00005.safetensors",
                    sha256: "198e7aeebcd24150db62e0af096d81c7054b57491edff01179f71b1ef8f2e2fe",
                    bytes: 4_844_691_024,
                },
                WeightsFile {
                    path: "model-00004-of-00005.safetensors",
                    sha256: "a81faa0f56099dd27590c1088e73b0a84e9fad71a322a90b89eb31dfd283d278",
                    bytes: 4_877_619_536,
                },
                WeightsFile {
                    path: "model-00005-of-00005.safetensors",
                    sha256: "6b2eee6db4ad12f8b78fc3b0143aa4bd2510f477cdb2e736c355c41d26850afe",
                    bytes: 2_282_630_240,
                },
            ],
        },
    ),
];

/// Find the weights source manifest for a weights identifier.
pub fn weights_source(weights_id: &str) -> Option<&'static WeightsSource> {
    WEIGHTS_SOURCES
        .iter()
        .find(|(id, _)| *id == weights_id)
        .map(|(_, s)| s)
}

/// Loud manifest validation (№334): EVERY file must carry a non-empty
/// SHA-256 pin and a positive byte count, and the paths must be unique
/// and repo-relative (no absolute paths, no `..`). A manifest without a
/// hash is a refusal, never a silent fallback (the weights.rs posture).
pub fn validate_weights_source(source: &WeightsSource) -> Result<(), String> {
    if source.repo.is_empty() || source.repo.starts_with('/') || source.repo.contains("..") {
        return Err(format!(
            "weights manifest '{}': repo id must be a canonical HF repo id",
            source.repo
        ));
    }
    if source.files.is_empty() {
        return Err(format!(
            "weights manifest '{}' is EMPTY — a manifest without files cannot \
             pin anything (silent fallback forbidden)",
            source.repo
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for f in source.files {
        let sha_ok = f.sha256.len() == 64 && f.sha256.chars().all(|c| c.is_ascii_hexdigit());
        if !sha_ok {
            return Err(format!(
                "weights manifest '{}': file '{}' has no valid pinned SHA-256 \
                 (expected 64 hex chars) — refusing to plan or fetch",
                source.repo, f.path
            ));
        }
        if f.bytes == 0 {
            return Err(format!(
                "weights manifest '{}': file '{}' declares 0 bytes — refusing",
                source.repo, f.path
            ));
        }
        if f.path.starts_with('/') || f.path.contains("..") {
            return Err(format!(
                "weights manifest '{}': file path '{}' must be repo-relative",
                source.repo, f.path
            ));
        }
        if seen.contains(&f.path) {
            return Err(format!(
                "weights manifest '{}': duplicate file path '{}'",
                source.repo, f.path
            ));
        }
        seen.push(f.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_complete() {
        for e in BACKEND_REGISTRY {
            assert!(!e.name.is_empty(), "{}: name", e.weights_id);
            assert!(!e.weights_id.is_empty(), "{}: weights_id", e.name);
            assert!(
                e.weights_id == e.weights_id.to_lowercase(),
                "{}: weights_id must be lowercase canon",
                e.name
            );
            assert!(
                !e.license_note.trim().is_empty(),
                "{}: license note is the classification basis",
                e.name
            );
            // Pins: either a real hex pin (64 lowercase hex chars) or the
            // typed PendingNo334 boundary — never an invented hash.
            match e.pin {
                ShaPin::PendingNo334 => {}
                ShaPin::Pinned(h) => {
                    assert_eq!(h.len(), 64, "{}: pin is sha256 hex", e.name);
                    assert!(
                        h.chars().all(|c| c.is_ascii_hexdigit()),
                        "{}: pin is hex",
                        e.name
                    );
                }
            }
        }
    }

    #[test]
    fn seed_license_classes_match_the_reported_basis() {
        // The MDL-3 test case (issue #460): Nemotron is not OSI-approved.
        let n = find_by_weights_id_ci("Nemotron-3-Nano-Omni-30B-A3B").expect("nemotron entry");
        assert_eq!(n.license, LicenseClass::NonOsi);
        // The default-deny posture: an unverified license is restrictive.
        let w = find_by_weights_id("wall-oss-0.5").expect("wall entry");
        assert_eq!(w.license, LicenseClass::Restrictive);
        // Known osi seeds.
        for id in [
            "chatterbox-multilingual-v3",
            "koko-ro-82m",
            "z-image-turbo",
            "molmoact2",
        ] {
            let e = find_by_weights_id(id).expect(id);
            assert_eq!(e.license, LicenseClass::Osi, "{}", id);
        }
    }

    #[test]
    fn classes_cover_the_mdl_vocabulary() {
        // At least one entry per §7.6 class that the tree already names.
        let has = |c: BackendClass| BACKEND_REGISTRY.iter().any(|e| e.class == c);
        assert!(has(BackendClass::Tts));
        assert!(has(BackendClass::VisionUnderstanding));
        assert!(has(BackendClass::Omni));
        // STT/LLM entries land with №334 (the SHA-pin path) — recorded
        // loudly in the naryad report, not stubbed here.
    }

    #[test]
    fn no_duplicates() {
        for (i, a) in BACKEND_REGISTRY.iter().enumerate() {
            for b in &BACKEND_REGISTRY[i + 1..] {
                assert_ne!(a.weights_id, b.weights_id, "weights_id uniqueness");
                assert_ne!(a.name, b.name, "name uniqueness");
            }
        }
    }
}
