//! Retained-representation diagnostic (naryad №415, path A per gh#527).
//!
//! Measures the RETAINED bytecode-side memory of a compiled program —
//! the class the Stage 5 re-gates №404/№410 flagged as the RSS blocker
//! (VM ~40.5–41.2 MB vs TW ~36.0–36.5 MB on CI; delta ≈ 4.3–4.6 MB).
//!
//! Sections measured:
//! 1. Inline `RegisterPattern(CompiledFn)` bodies in main_code (the
//!    compiler never fills `Program.patterns` — the table field is dead,
//!    so the ONLY copy lives inline) + the №402 shared-snapshot CLONE of
//!    the same bodies (a full duplicate on first load).
//! 2. Rules wire copy + sorted snapshot clone.
//! 3. deny_handlers / skill_indices / globals / schema_ddl + their
//!    snapshot clones.
//! 4. Route bodies (compiled exactly like the server does at startup).
//! 5. Total .mbc serialized size (a floor estimate of the wire mass).
//!
//! Usage: cargo run --example retained_diag -- benches/fixtures/production_workload.mlog

use metalogos::bytecode::{Instruction, Program};
use metalogos::compiler::Compiler;
use metalogos::parser;

fn instr_heap_units(code: &[Instruction]) -> usize {
    // A crude but consistent "payload mass" proxy: 1 unit per instruction
    // plus 1 per embedded String/Vec payload slot (MakeStruct, LoadGlobalByName,
    // Const etc. carry heap payloads; the proxy counts slots, not bytes).
    code.len() * 2
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "benches/fixtures/production_workload.mlog".into());
    let src = std::fs::read_to_string(&path).expect("read source");

    // Compile exactly like the server's VM startup path.
    let declarations = parser::parse(&src).expect("parse");
    let config_routes: Vec<_> = declarations
        .iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.routes.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut compiler = Compiler::new();
    let program = compiler.compile(declarations).expect("compile");
    let routes = compiler.compile_routes(&config_routes).expect("routes");

    // ── 1. Inline pattern bodies in main_code ──
    let mut inline_bodies = 0usize;
    let mut inline_instrs = 0usize;
    for instr in &program.main_code {
        if let Instruction::RegisterPattern(f) = instr {
            inline_bodies += 1;
            inline_instrs += f.code.len();
        }
    }
    // Serialized mass of one copy (bincode), summed per body.
    let inline_bytes: usize = program
        .main_code
        .iter()
        .filter_map(|i| match i {
            Instruction::RegisterPattern(f) => Some(
                bincode::serde::encode_to_vec(f, bincode::config::legacy()).unwrap_or_default()
                    .len(),
            ),
            _ => None,
        })
        .sum();

    println!("== patterns (inline in main_code) ==");
    println!(
        "  bodies: {} | instructions: {} | serialized: {} B ({} KiB)",
        inline_bodies,
        inline_instrs,
        inline_bytes,
        inline_bytes / 1024
    );
    println!("  Program.patterns table entries: {}  <-- naryad 415: the CANONICAL body store (was dead/always empty pre-415)", program.patterns.len());

    // ── 2. Rules ──
    let rules_bytes =
        bincode::serde::encode_to_vec(&program.rules, bincode::config::legacy()).unwrap_or_default();
    println!("== rules ==");
    println!(
        "  count: {} | serialized wire copy: {} B ({} KiB) | sorted snapshot = second copy",
        program.rules.len(),
        rules_bytes.len(),
        rules_bytes.len() / 1024
    );

    // ── 3. Small tables ──
    let dh =
        bincode::serde::encode_to_vec(&program.deny_handlers, bincode::config::legacy())
            .unwrap_or_default();
    let si =
        bincode::serde::encode_to_vec(&program.skill_indices, bincode::config::legacy())
            .unwrap_or_default();
    let gl =
        bincode::serde::encode_to_vec(&program.globals, bincode::config::legacy())
            .unwrap_or_default();
    let ddl =
        bincode::serde::encode_to_vec(&program.schema_ddl, bincode::config::legacy())
            .unwrap_or_default();
    let ln =
        bincode::serde::encode_to_vec(&program.learnables, bincode::config::legacy())
            .unwrap_or_default();
    println!("== small tables (serialized; each also cloned once into the snapshot) ==");
    println!(
        "  deny_handlers: {} entries, {} B | skill_indices: {} entries, {} B",
        program.deny_handlers.len(),
        dh.len(),
        program.skill_indices.len(),
        si.len()
    );
    println!(
        "  globals: {} names, {} B | schema_ddl: {} stmts, {} B | learnables: {} entries, {} B",
        program.globals.len(),
        gl.len(),
        program.schema_ddl.len(),
        ddl.len(),
        program.learnables.len(),
        ln.len()
    );

    // ── 4. Route bodies ──
    let route_bytes: usize = routes
        .iter()
        .map(|r| bincode::serde::encode_to_vec(&r.code, bincode::config::legacy()).unwrap_or_default().len())
        .sum();
    let route_instrs: usize = routes.iter().map(|r| r.code.len()).sum();
    println!("== routes (server startup compile) ==");
    println!(
        "  routes: {} | instructions: {} | serialized: {} B ({} KiB)",
        routes.len(),
        route_instrs,
        route_bytes,
        route_bytes / 1024
    );

    // ── 5. Whole-program wire size ──
    let mbc = program.serialize().unwrap_or_default();
    println!("== totals ==");
    println!("  .mbc serialized Program: {} B ({} KiB)", mbc.len(), mbc.len() / 1024);
    println!(
        "  Leftover inline+snapshot duplicate mass (0 after 415): ~{} KiB per Program",
        (inline_bytes * 2) / 1024
    );
    println!(
        "  instruction-slot proxy: main_code {} ({} from inline bodies), heap-units ~{}",
        program.main_code.len(),
        inline_instrs,
        instr_heap_units(&program.main_code)
    );
    let _ = std::mem::size_of::<Program>();
}

#[cfg(test)]
mod sizes {
    #[test]
    fn print_sizes() {
        println!("size_of Instruction = {}", std::mem::size_of::<metalogos::bytecode::Instruction>());
        println!("size_of CompiledFn  = {}", std::mem::size_of::<metalogos::bytecode::CompiledFn>());
        println!("size_of Value       = {}", std::mem::size_of::<metalogos::interpreter::Value>());
    }
}
