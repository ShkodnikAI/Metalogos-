// ── Наряд №271 (SPIKE): sqlite-vec KNN vs текущий полный скан ────────
//
// Бенчмарк воспроизводит реальный путь SqliteStore::recall_top_k
// (src/memory_store.rs): SELECT всех строк → decode embedding BLOB
// (little-endian f32) → скалярный cosine_similarity → sort → truncate.
//
// Три измерения на каждом масштабе (10K / 100K векторов, dim=384, k=10):
//   1. full_scan_decode     — текущий путь: BLOB → Vec<f32> → cosine
//   2. full_scan_cosine_only — нижняя граница текущего пути без decode
//   3. sqlite_vec_knn       — vec0 virtual table, KNN через MATCH
//
// Вектора детерминированы (xorshift64*), нормированы — при единичной норме
// L2-ранжирование совпадает с cosine-ранжированием, поэтому корректность
// сверяется на smoke-тесте (tests/naryad_271_sqlite_vec_spike.rs).
//
// Файл пуст без feature `vec` (#![cfg]) — clippy --all-targets и
// сборки без фичи его не видят. В main НЕ мержится (только документ).

#![cfg(feature = "vec")]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use std::time::Instant;

const DIM: usize = 384;
const K: usize = 10;
const N_SMALL: usize = 10_000;
const N_LARGE: usize = 100_000;

// ── Детерминированный PRNG (xorshift64*), чтобы бенчмарк был воспроизводим
struct Xorshift(u64);

impl Xorshift {
    fn next_f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        ((v >> 40) as f32 / 16_777_216.0) - 1.0 // [-1, 1)
    }
}

/// Нормированный вектор (единичная L2-норма) — cosine == монотонное L2.
fn make_unit_vec(rng: &mut Xorshift) -> Vec<f32> {
    let mut v: Vec<f32> = (0..DIM).map(|_| rng.next_f32()).collect();
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

// Скалярный cosine — точная копия src/embeddings.rs::cosine_similarity.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na * nb)).clamp(-1.0, 1.0)
    }
}

// LE-сериализация — точная копия SqliteStore::embedding_to_blob.
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── sqlite-vec: регистрация расширения (статическая, без load_extension)
fn register_vec_extension() {
    use rusqlite::ffi::sqlite3_auto_extension;
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        // Каноничный паттерн sqlite-vec README для rusqlite:
        // sqlite3_vec_init регистрируется как auto-extension и вызывается
        // для каждого нового соединения. Feature `load_extension` НЕ нужна.
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

fn open_vec_db(n: usize) -> (rusqlite::Connection, Vec<f32>) {
    register_vec_extension();
    let conn = rusqlite::Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[{DIM}] distance_metric=cosine);"
    ))
    .expect("create vec0 table");

    let mut rng = Xorshift(0x4D45_5441_4C4F_474F); // "METALOGO"
    conn.execute_batch("BEGIN;").unwrap();
    let t0 = Instant::now();
    {
        let mut stmt = conn
            .prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?1, ?2)")
            .expect("prepare insert");
        for rowid in 1..=n as i64 {
            let v = make_unit_vec(&mut rng);
            stmt.execute(rusqlite::params![rowid, embedding_to_blob(&v)])
                .expect("insert vector");
        }
    }
    conn.execute_batch("COMMIT;").unwrap();
    eprintln!(
        "[naryad-271] insert {} x dim={} into vec0 (cosine): {:?} ({:.0}/s)",
        n,
        DIM,
        t0.elapsed(),
        n as f64 / t0.elapsed().as_secs_f64()
    );
    let query = make_unit_vec(&mut rng);
    (conn, query)
}

fn full_scan_decode(blobs: &[Vec<u8>], query: &[f32]) -> Vec<f32> {
    let mut scored: Vec<(usize, f32)> = blobs
        .iter()
        .enumerate()
        .map(|(i, blob)| {
            let e = blob_to_embedding(blob);
            (i, cosine_similarity(query, &e))
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(K);
    scored.into_iter().map(|(_, s)| s).collect()
}

fn full_scan_cosine_only(vectors: &[Vec<f32>], query: &[f32]) -> Vec<f32> {
    let mut scored: Vec<(usize, f32)> = vectors
        .iter()
        .enumerate()
        .map(|(i, e)| (i, cosine_similarity(query, e)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(K);
    scored.into_iter().map(|(_, s)| s).collect()
}

fn sqlite_vec_knn(conn: &rusqlite::Connection, query: &[f32]) -> Vec<f64> {
    let mut stmt = conn
        .prepare("SELECT rowid, distance FROM vec_items WHERE embedding MATCH ?1 AND k = ?2")
        .expect("prepare knn");
    let rows = stmt
        .query_map(rusqlite::params![embedding_to_blob(query), K], |r| {
            r.get::<_, f64>(1)
        })
        .expect("knn query");
    rows.map(|r| r.expect("row")).collect()
}

fn bench_scale(c: &mut Criterion, n: usize, group_prefix: &str) {
    // Общие данные для обоих полных сканов.
    let mut rng = Xorshift(0x4D45_5441_4C4F_474F);
    let vectors: Vec<Vec<f32>> = (0..n).map(|_| make_unit_vec(&mut rng)).collect();
    let blobs: Vec<Vec<u8>> = vectors.iter().map(|v| embedding_to_blob(v)).collect();
    let query = {
        let mut qrng = Xorshift(0xDEAD_BEEF_CAFE_0001);
        make_unit_vec(&mut qrng)
    };

    let (conn, vec_query) = open_vec_db(n);

    // Верификация топ-1: vec0 KNN должен совпасть с полным сканом
    // (нормированные вектора => ранжирования идентичны).
    let bf_top = full_scan_cosine_only(&vectors, &query);
    let knn_top = sqlite_vec_knn(&conn, &vec_query);
    assert_eq!(knn_top.len(), K, "vec0 KNN вернул не k строк");
    let bf_best = bf_top[0];
    let knn_best = 1.0 - knn_top[0]; // cosine distance -> similarity
    assert!(
        (bf_best - knn_best).abs() < 1e-4,
        "top-1 расходится: bf={bf_best}, vec={knn_best}"
    );
    eprintln!(
        "[naryad-271] top-1 verified at n={n}: bf={bf_best:.6}, vec0={knn_best:.6}"
    );

    let mut group = c.benchmark_group(group_prefix);
    group.throughput(Throughput::Elements(n as u64));

    group.bench_function("full_scan_decode", |b| {
        b.iter(|| full_scan_decode(black_box(&blobs), black_box(&query)))
    });
    group.bench_function("full_scan_cosine_only", |b| {
        b.iter(|| full_scan_cosine_only(black_box(&vectors), black_box(&query)))
    });
    group.bench_function("sqlite_vec_knn", |b| {
        b.iter(|| sqlite_vec_knn(black_box(&conn), black_box(&vec_query)))
    });

    group.finish();
}

fn bench_knn_10k(c: &mut Criterion) {
    bench_scale(c, N_SMALL, "naryad271_knn_10k");
}

fn bench_knn_100k(c: &mut Criterion) {
    bench_scale(c, N_LARGE, "naryad271_knn_100k");
}

criterion_group!(benches, bench_knn_10k, bench_knn_100k);
criterion_main!(benches);
