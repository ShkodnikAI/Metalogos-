// ── Наряд №190 Contract 1: stacked transformer_blocks get different weights ──
//
// Per the naryad spec:
//   "два блока должны получить РАЗНЫЕ веса (детерминированно из общего
//    seed, но не идентичные копии друг друга — иначе стек математически
//    эквивалентен одному блоку с удвоенной глубиной без реальной выгоды).
//    Если текущая реализация даёт двум слоям одинаковые веса — это баг,
//    найденный этим нарядом, не свойство, которое нужно сохранить."
//
// Bug found + fixed in this naryad:
//   TrainableAttention registered Var's under fixed names "attn_w_q",
//   "attn_w_k", etc. in VarMap. A stack of 2 transformer_blocks would
//   overwrite each other — the second block's Var's would replace the
//   first's, and backward() would only see one set of gradients.
//
// Fix: added `prefix` parameter (Наряд №190) — each layer registers
// under "{prefix}_w_q" etc., where prefix = "block{i}" for transformer_block
// or "layer{i}" for attention.
//
// This test verifies the fix: builds 2 transformer_blocks, extracts
// their weight Var's from VarMap, and asserts they are DIFFERENT.

#![cfg(feature = "candle")]

use metalogos::nn::trainable_transformer_block::TrainableTransformerBlock;

use candle_nn::VarMap;

#[test]
fn stacked_blocks_have_different_weights() {
    let heads = 2;
    let dim = 8;
    let ff_dim = 16;
    let seed = 42;

    let var_map = VarMap::new();

    // Build two blocks with different prefixes.
    let _block0 = TrainableTransformerBlock::new(heads, dim, ff_dim, seed, &var_map, "block0")
        .expect("block0 build");
    let _block1 = TrainableTransformerBlock::new(
        heads,
        dim,
        ff_dim,
        seed.wrapping_add(1),
        &var_map,
        "block1",
    )
    .expect("block1 build");

    // Extract Var's from VarMap by name.
    let data = var_map.data().lock().unwrap();

    // Both blocks should have registered their weights under unique names.
    let block0_q = data
        .get("block0_attn_w_q")
        .expect("block0_attn_w_q missing");
    let block1_q = data
        .get("block1_attn_w_q")
        .expect("block1_attn_w_q missing");

    // Extract the weight values as Vec<f32>.
    let v0: Vec<f32> = block0_q
        .as_tensor()
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");
    let v1: Vec<f32> = block1_q
        .as_tensor()
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    assert_eq!(v0.len(), v1.len(), "weight vecs same length");

    // THE KEY ASSERTION: weights must be DIFFERENT (not identical copies).
    // If they were identical, the stack would be mathematically equivalent
    // to one block with doubled depth — no real capacity gain.
    let mut differences = 0;
    for (a, b) in v0.iter().zip(v1.iter()) {
        if (a - b).abs() > 1e-6 {
            differences += 1;
        }
    }
    assert!(
        differences > 0,
        "block0 and block1 have IDENTICAL weights — bug: stack has no real capacity gain"
    );
    println!(
        "✓ stacked blocks have different weights ({} of {} elements differ)",
        differences,
        v0.len()
    );
}

#[test]
fn stacked_blocks_unique_varmap_names() {
    // Verify that the VarMap contains entries for BOTH blocks — not just
    // the last one (which was the bug before the prefix fix).
    let var_map = VarMap::new();

    let _block0 = TrainableTransformerBlock::new(2, 8, 16, 42, &var_map, "block0").expect("block0");
    let _block1 = TrainableTransformerBlock::new(2, 8, 16, 43, &var_map, "block1").expect("block1");

    let data = var_map.data().lock().unwrap();

    // Both blocks' weights should be present in VarMap.
    assert!(
        data.contains_key("block0_attn_w_q"),
        "block0 weights missing"
    );
    assert!(
        data.contains_key("block1_attn_w_q"),
        "block1 weights missing"
    );
    assert!(
        data.contains_key("block0_attn_w_k"),
        "block0 K weights missing"
    );
    assert!(
        data.contains_key("block1_attn_w_k"),
        "block1 K weights missing"
    );

    // Total Var count: 2 blocks × 4 weights each (Q, K, V, O) = 8
    assert_eq!(
        data.len(),
        8,
        "expected 8 Vars in VarMap (2 blocks × 4 each), got {}",
        data.len()
    );

    println!(
        "✓ VarMap contains {} unique entries for 2 stacked blocks",
        data.len()
    );
}
