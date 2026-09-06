// ── Наряд №184 Contract 2: SwiGLU forward — numerical correctness ──
//
// Per the naryad spec:
//   "численная корректность против независимого NumPy-референса"
//
// ## Reference numbers
//
// The expected output is computed by an independent NumPy script
// (kept in /home/z/my-project/scripts/naryad_184_numpy_reference.py,
// also embedded below). The script reproduces the same SwiGLU algorithm:
//   SwiGLU(x) = (silu(x @ W_gate) * (x @ W_up)) @ W_down
// where silu(z) = z * sigmoid(z), and weights are generated via the
// project's xorshift64 PRNG (same algorithm as attention.rs).
//
// ```python
// import numpy as np
// # xorshift64 (matches src/builtins/math.rs's algorithm)
// def make_rng(seed):
//     state = (seed ^ 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
//     if state == 0: state = 0x9E3779B97F4A7C15
//     def rand_u01():
//         nonlocal state
//         state ^= (state << 13) & 0xFFFFFFFFFFFFFFFF
//         state ^= state >> 7
//         state ^= (state << 17) & 0xFFFFFFFFFFFFFFFF
//         return (state >> 11) / (1 << 53)
//     return rand_u01
// def gen_uniform(seed, n, lo, up):
//     r = make_rng(seed)
//     return np.array([(r() * (up - lo) + lo) for _ in range(n)], dtype=np.float32)
// dim = 8; ff_dim = 16; seq = 3; seed = 42
// bound = 1.0 / np.sqrt(dim)
// weights = gen_uniform(seed, 3 * dim * ff_dim, -bound, bound)
// w_gate = weights[:dim*ff_dim].reshape(dim, ff_dim)
// w_up = weights[dim*ff_dim:2*dim*ff_dim].reshape(dim, ff_dim)
// w_down = weights[2*dim*ff_dim:3*dim*ff_dim].reshape(ff_dim, dim)
// x = np.arange(seq * dim, dtype=np.float32).reshape(seq, dim) * 0.1
// g = x.astype(np.float64) @ w_gate.astype(np.float64)
// u = x.astype(np.float64) @ w_up.astype(np.float64)
// silu_g = g * (1.0 / (1.0 + np.exp(-g)))
// h = silu_g * u
// out = (h @ w_down.astype(np.float64)).astype(np.float32)
// ```
//
// Output (24 values, [3, 8] flattened row-major):
//   [0.020007595, -0.013434383, 0.0137327975, 0.023506023,
//    -0.067236364, -0.023171866, 0.0016555763, 0.033985775,
//     0.0029476737, -0.19392473, -0.037608083, -0.049880154,
//    -0.40928888, -0.1555851, -0.005764028, 0.18303192,
//    -0.11405373, -0.70707244, -0.22771959, -0.4031396,
//    -1.0679955, -0.41016898, -0.006430898, 0.46283302]

#![cfg(feature = "candle")]

use metalogos::nn::sequence_layer::SequenceLayer;
use metalogos::nn::swiglu::SwiGlu;

#[test]
fn swiglu_forward_matches_numpy_reference() {
    let dim = 8;
    let ff_dim = 16;
    let seq_len = 3;
    let seed = 42;

    let layer = SwiGlu::new(dim, ff_dim, seed).expect("swiglu build");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let output = layer.forward(&input).expect("forward");
    let output_vec: Vec<f32> = output
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    let expected: &[f32] = &[
        0.020007595,
        -0.013434383,
        0.0137327975,
        0.023506023,
        -0.067236364,
        -0.023171866,
        0.0016555763,
        0.033985775,
        0.0029476737,
        -0.19392473,
        -0.037608083,
        -0.049880154,
        -0.40928888,
        -0.1555851,
        -0.005764028,
        0.18303192,
        -0.11405373,
        -0.70707244,
        -0.22771959,
        -0.4031396,
        -1.0679955,
        -0.41016898,
        -0.006430898,
        0.46283302,
    ];

    assert_eq!(output_vec.len(), expected.len(), "output length mismatch");

    let mut max_diff = 0.0f32;
    for (i, (actual, expected)) in output_vec.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - expected).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-4,
            "swiglu output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ swiglu forward matches NumPy reference (max diff = {:.2e}, tolerance = 1e-4)",
        max_diff
    );
}

#[test]
fn swiglu_forward_deterministic_same_seed() {
    let dim = 16;
    let ff_dim = 32;
    let seq_len = 4;

    let l1 = SwiGlu::new(dim, ff_dim, 99).expect("swiglu build");
    let l2 = SwiGlu::new(dim, ff_dim, 99).expect("swiglu build (same seed)");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.05).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input")
        .to_dtype(candle_core::DType::F32)
        .expect("dtype");

    let o1 = l1.forward(&input).expect("forward 1");
    let o2 = l2.forward(&input).expect("forward 2");

    let v1: Vec<f32> = o1.flatten_all().expect("flatten").to_vec1().expect("v1");
    let v2: Vec<f32> = o2.flatten_all().expect("flatten").to_vec1().expect("v2");

    assert_eq!(v1.len(), v2.len());
    let max_diff = v1
        .iter()
        .zip(v2.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff == 0.0,
        "same seed → bitwise-identical output, got max diff = {:.2e}",
        max_diff
    );
    println!(
        "✓ swiglu determinism: bitwise-identical ({} elements)",
        v1.len()
    );
}

#[test]
fn swiglu_output_shape_preserved() {
    // Output dim == input dim (residual stream preserved).
    let dim = 16;
    let ff_dim = 64;
    let seq_len = 4;
    let layer = SwiGlu::new(dim, ff_dim, 1).expect("swiglu build");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.01).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input")
        .to_dtype(candle_core::DType::F32)
        .expect("dtype");

    let output = layer.forward(&input).expect("forward");
    let out_dims = output.dims();
    assert_eq!(out_dims, &[seq_len, dim]);
    println!("✓ swiglu output shape: {:?} (residual preserved)", out_dims);
}
