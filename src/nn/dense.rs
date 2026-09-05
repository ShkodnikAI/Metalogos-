//! Dense (fully-connected) layer — first Layer implementation.
//!
//! Forward pass: y = activation(x · W + b)
//! where W is [input_size × output_size] and b is [output_size].
//!
//! Weight initialization: Xavier/Glorot uniform — deterministic when
//! given the same seed (uses the xorshift64 PRNG from Наряд №177).

use crate::interpreter::Value;
use crate::nn::activation::ActivationKind;
use crate::nn::layer::Layer;

/// Dense layer: y = activation(x · W + b)
pub struct Dense {
    weights: Vec<Vec<f64>>, // [output_size][input_size]
    bias: Vec<f64>,         // [output_size]
    activation: ActivationKind,
    input_dim: usize,
    output_dim: usize,
}

impl Dense {
    /// Create a new Dense layer with deterministic weight initialization.
    /// Uses Xavier/Glorot uniform: weights ~ U(-limit, limit)
    /// where limit = sqrt(6 / (input_dim + output_dim)).
    pub fn new(input_dim: usize, output_dim: usize, activation: ActivationKind, seed: u64) -> Self {
        let limit = (6.0 / (input_dim + output_dim) as f64).sqrt();
        let mut rng = XorShift64::new(seed);

        let weights: Vec<Vec<f64>> = (0..output_dim)
            .map(|_| {
                (0..input_dim)
                    .map(|_| {
                        // Map [0,1) → [-limit, limit]
                        (rng.next_float() * 2.0 - 1.0) * limit
                    })
                    .collect()
            })
            .collect();

        let bias: Vec<f64> = (0..output_dim).map(|_| 0.0).collect();

        Self {
            weights,
            bias,
            activation,
            input_dim,
            output_dim,
        }
    }

    /// Create a Dense layer with explicit weights (for testing).
    pub fn with_weights(
        weights: Vec<Vec<f64>>,
        bias: Vec<f64>,
        activation: ActivationKind,
    ) -> Self {
        let output_dim = weights.len();
        let input_dim = if output_dim > 0 { weights[0].len() } else { 0 };
        Self {
            weights,
            bias,
            activation,
            input_dim,
            output_dim,
        }
    }
}

impl Layer for Dense {
    fn forward(&self, input: &[f64]) -> Vec<f64> {
        debug_assert_eq!(
            input.len(),
            self.input_dim,
            "Dense::forward: input len {} != input_dim {}",
            input.len(),
            self.input_dim
        );

        let mut output = vec![0.0; self.output_dim];

        // y = x · W + b (W is transposed: weights[out][in])
        for (o, w_row) in self.weights.iter().enumerate() {
            let mut sum = self.bias[o];
            for (i, &x) in input.iter().enumerate() {
                sum += x * w_row[i];
            }
            output[o] = sum;
        }

        // Apply activation (reuses math.rs implementations)
        self.activation.apply(&mut output);

        output
    }

    fn input_size(&self) -> usize {
        self.input_dim
    }

    fn output_size(&self) -> usize {
        self.output_dim
    }

    fn name(&self) -> &str {
        "dense"
    }

    fn serialize_weights(&self) -> Vec<u8> {
        // Simple format: [input_dim:u32][output_dim:u32][activation:u8]
        // followed by weights (f64 LE) and bias (f64 LE).
        let mut data = Vec::new();
        data.extend_from_slice(&(self.input_dim as u32).to_le_bytes());
        data.extend_from_slice(&(self.output_dim as u32).to_le_bytes());
        data.push(self.activation as u8);
        for row in &self.weights {
            for &w in row {
                data.extend_from_slice(&w.to_le_bytes());
            }
        }
        for &b in &self.bias {
            data.extend_from_slice(&b.to_le_bytes());
        }
        data
    }

    #[allow(clippy::unwrap_used)]
    fn deserialize_weights(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() < 9 {
            return Err("dense: serialized data too short".to_string());
        }
        let input_dim = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let output_dim = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let activation_byte = data[8];

        let activation = match activation_byte {
            0 => ActivationKind::None,
            1 => ActivationKind::Relu,
            2 => ActivationKind::Sigmoid,
            3 => ActivationKind::Tanh,
            4 => ActivationKind::Softmax,
            _ => {
                return Err(format!(
                    "dense: unknown activation byte: {}",
                    activation_byte
                ))
            }
        };

        let expected_len = 9 + input_dim * output_dim * 8 + output_dim * 8;
        if data.len() < expected_len {
            return Err(format!(
                "dense: expected {} bytes, got {}",
                expected_len,
                data.len()
            ));
        }

        self.input_dim = input_dim;
        self.output_dim = output_dim;
        self.activation = activation;

        let mut offset = 9;
        self.weights = Vec::with_capacity(output_dim);
        for _ in 0..output_dim {
            let mut row = Vec::with_capacity(input_dim);
            for _ in 0..input_dim {
                let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
                row.push(f64::from_le_bytes(bytes));
                offset += 8;
            }
            self.weights.push(row);
        }

        self.bias = Vec::with_capacity(output_dim);
        for _ in 0..output_dim {
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            self.bias.push(f64::from_le_bytes(bytes));
            offset += 8;
        }

        Ok(())
    }
}

/// Build function for LAYER_REGISTRY.
/// Args: [units: Float, activation: String]
pub fn build_dense(args: &[Value], seed: u64) -> Result<Box<dyn Layer>, String> {
    let units = match args.first() {
        Some(Value::Float(f)) => *f as usize,
        Some(other) => {
            return Err(format!(
                "dense: 'units' must be Float, got {}",
                other.type_name()
            ))
        }
        None => return Err("dense: requires 'units' parameter".to_string()),
    };

    let activation_str = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        Some(other) => {
            return Err(format!(
                "dense: 'activation' must be String, got {}",
                other.type_name()
            ))
        }
        None => "none",
    };

    let activation = ActivationKind::parse_kind(activation_str)?;

    // input_dim is not known at build time — it's determined by the
    // previous layer in the stack. For now, we use a placeholder (0)
    // and the actual dim is set when the model is assembled.
    // The Dense layer created here has 0 input_dim — it needs to be
    // rebuilt with the correct input_dim when layers are chained.
    // This is handled in ReflexModel::build (Наряд №178 Block 3).
    Ok(Box::new(Dense::new(0, units, activation, seed)))
}

// ── xorshift64 PRNG — same algorithm as math.rs (Наряд №177) ──────────
// Duplicated here (not imported) because math.rs stores state in
// thread_local, and we need an instance-based PRNG for deterministic
// layer init within a single model.

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let state = seed ^ 0x9E3779B97F4A7C15;
        Self {
            state: if state == 0 {
                0x9E3779B97F4A7C15
            } else {
                state
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_float(&mut self) -> f64 {
        let bits = self.next_u64();
        let mantissa = bits >> 11; // top 53 bits
        (mantissa as f64) / ((1u64 << 53) as f64)
    }
}
