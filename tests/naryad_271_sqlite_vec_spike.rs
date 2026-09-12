// ── Наряд №271 (SPIKE): smoke-тест sqlite-vec поверх rusqlite 0.40 bundled ──
//
// Проверяет канонический контракт интеграции без feature `load_extension`:
//   1. Статическая регистрация sqlite3_vec_init через sqlite3_auto_extension.
//   2. CREATE VIRTUAL TABLE ... USING vec0(... distance_metric=cosine).
//   3. KNN-запрос MATCH ... AND k = N — совпадение топ-1 с полным сканом.
//
// Запуск: cargo test --features vec --test naryad_271_sqlite_vec_spike
// В main НЕ мержится (только документ спайка).

#![cfg(feature = "vec")]

use rusqlite::ffi::sqlite3_auto_extension;
use std::sync::Once;

const DIM: usize = 384;

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

fn unit_vec(seed: u64) -> Vec<f32> {
    // murmur3-финализатор сида: без него соседние сиды (42 и 43 при `| 1`)
    // схлопываются в одно состояние генератора и дают векторы-дубликаты
    // (найдено диагностикой спайка — vec0 при этом считает корректно).
    let mut x = seed;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    x |= 1;
    let mut v: Vec<f32> = (0..DIM)
        .map(|_| {
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f32 / 16_777_216.0) - 1.0
        })
        .collect();
    let norm: f32 = v.iter().map(|a| a * a).sum::<f32>().sqrt();
    for a in &mut v {
        *a /= norm;
    }
    v
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

#[test]
fn naryad_271_sqlite_vec_smoke() {
    register();
    let conn = rusqlite::Connection::open_in_memory().expect("open db");

    let version: String = conn
        .query_row("SELECT vec_version()", [], |r| r.get(0))
        .expect("vec_version() — расширение не загрузилось");
    eprintln!("[naryad-271] sqlite-vec version = {version}");

    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[{DIM}] distance_metric=cosine);"
    ))
    .expect("create vec0 table with cosine metric");

    let n = 1000usize;
    let vectors: Vec<Vec<f32>> = (1..=n).map(|i| unit_vec(i as u64)).collect();
    conn.execute_batch("BEGIN;").unwrap();
    {
        let mut stmt = conn
            .prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?1, ?2)")
            .unwrap();
        for (i, v) in vectors.iter().enumerate() {
            stmt.execute(rusqlite::params![i as i64 + 1, to_blob(v)]).unwrap();
        }
    }
    conn.execute_batch("COMMIT;").unwrap();

    // Запрос — вектор seed=42, топ-1 должен совпасть с полным скалярным сканом.
    let query = unit_vec(42);
    let mut best = (0usize, f32::MIN);
    for (i, v) in vectors.iter().enumerate() {
        let dot: f32 = query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
        if dot > best.1 {
            best = (i, dot);
        }
    }

    let mut stmt = conn
        .prepare("SELECT rowid, distance FROM vec_items WHERE embedding MATCH ?1 AND k = 5")
        .expect("prepare knn");
    let rows: Vec<(i64, f64)> = stmt
        .query_map(rusqlite::params![to_blob(&query)], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 5, "KNN должен вернуть k=5 строк");
    assert!(
        rows.windows(2).all(|w| w[0].1 <= w[1].1),
        "результаты не отсортированы по distance"
    );
    assert_eq!(
        rows[0].0 as usize,
        best.0 + 1,
        "top-1 vec0 ({}) != top-1 полного скана ({})",
        rows[0].0,
        best.0 + 1
    );
    eprintln!(
        "[naryad-271] top-1 rowid={} sim={:.6} — совпал с полным сканом",
        rows[0].0,
        1.0 - rows[0].1
    );
    eprintln!("NARYAD-271 SMOKE PASS: sqlite-vec {version}, dim={DIM}, n={n}, cosine-metric, KNN OK");
}
