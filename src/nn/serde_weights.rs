//! Weight serialization — stub for Наряд №180 (persistence).
//!
//! The actual serialize/deserialize logic lives in each Layer's impl
//! (Layer::serialize_weights / deserialize_weights).
//! This module provides format-agnostic helpers and will be extended
//! in Наряд №180 to handle file I/O (save_model/load_model builtins).

/// Magic bytes for Reflex model files (Наряд №180).
pub const REFLEX_MAGIC: &[u8; 4] = b"RFLX";

/// Current format version.
pub const REFLEX_VERSION: u32 = 1;

/// Serialize a list of layer weight blobs into a single byte vector.
/// Format: [magic:4][version:4][num_layers:4]
///         then for each layer: [len:4][data:len]
pub fn serialize_model(layers: &[Vec<u8>]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(REFLEX_MAGIC);
    data.extend_from_slice(&REFLEX_VERSION.to_le_bytes());
    data.extend_from_slice(&(layers.len() as u32).to_le_bytes());
    for layer in layers {
        data.extend_from_slice(&(layer.len() as u32).to_le_bytes());
        data.extend_from_slice(layer);
    }
    data
}

/// Deserialize a model byte vector back into per-layer weight blobs.
#[allow(clippy::unwrap_used)]
pub fn deserialize_model(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if data.len() < 12 {
        return Err("model data too short".to_string());
    }
    if &data[0..4] != REFLEX_MAGIC {
        return Err("invalid magic bytes".to_string());
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != REFLEX_VERSION {
        return Err(format!("unsupported model version: {}", version));
    }
    let num_layers = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

    let mut offset = 12;
    let mut layers = Vec::with_capacity(num_layers);
    for _ in 0..num_layers {
        if offset + 4 > data.len() {
            return Err("unexpected end of model data".to_string());
        }
        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + len > data.len() {
            return Err("layer data exceeds buffer".to_string());
        }
        layers.push(data[offset..offset + len].to_vec());
        offset += len;
    }
    Ok(layers)
}
