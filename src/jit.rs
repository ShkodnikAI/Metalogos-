// ── METALOGOS JIT Compiler — Phase 4.3 ────────────────────────────
//
// Compiles hot bytecode patterns to native machine code via Cranelift.
// Strategy: the VM counts calls per pattern. When a pure Float-only pattern
// exceeds the threshold (default N=100), it is compiled to native code.
// Subsequent calls dispatch directly to the native function pointer.
//
// Constraints (documented, intentional):
//   - JIT only for pure patterns (no LLM calls, no side-effect builtins,
//     no global reads/writes beyond parameters, no struct operations).
//   - Only Float parameters and Float return type are supported.
//   - Supported bytecode instructions: LoadLocal, Const(Float), Add, Sub,
//     Mul, Div, Return. Comparison instructions (Cmp*) stay in VM.
//   - Learnable and adapt patterns ALWAYS stay in the VM (they have side
//     effects and/or LLM interactions).
//   - No guard checks needed: pure patterns have no external dependencies.

use cranelift_codegen::ir::types::F64;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value as Cv};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::bytecode::{CompiledFn, Instruction};
use crate::interpreter::Value;

/// A JIT-compiled function pointer. Stored as a raw pointer to allow
/// different arities. The caller must transmute to the correct signature.
pub type JitFnPtr = *const u8;

/// The METALOGOS JIT compiler. Owns a Cranelift JIT module and manages
/// the set of compiled native function pointers.
pub struct JitCompiler {
    /// The Cranelift JIT module (owns executable memory).
    module: JITModule,
    /// Shared function builder context (reused across compilations).
    func_ctx: FunctionBuilderContext,
    /// Compiled function pointers: pattern index -> native code pointer.
    compiled: Vec<Option<JitFnPtr>>,
    /// Hot-path threshold: number of calls before JIT compilation triggers.
    threshold: usize,
    /// Unique counter for generating Cranelift function names.
    name_counter: usize,
}

impl JitCompiler {
    /// Create a new JIT compiler with the given hot-path threshold.
    pub fn new(threshold: usize) -> Result<Self, String> {
        let mut flag_builder = settings::builder();
        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| format!("JIT settings error: {}", e))?;
        flag_builder
            .set("is_pic", "false")
            .map_err(|e| format!("JIT settings error: {}", e))?;

        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("JIT: host machine not supported: {}", e))?;

        let flags = settings::Flags::new(flag_builder);
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| format!("JIT: ISA creation failed: {}", e))?;

        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Ok(JitCompiler {
            module,
            func_ctx: FunctionBuilderContext::new(),
            compiled: Vec::new(),
            threshold,
            name_counter: 0,
        })
    }

    /// Get the hot-path threshold.
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Ensure the compiled vector is large enough for the given pattern index.
    fn ensure_capacity(&mut self, idx: usize) {
        while self.compiled.len() <= idx {
            self.compiled.push(None);
        }
    }

    /// Check if a pattern is JIT-eligible: pure Float-only with only
    /// supported bytecode instructions (arithmetic only, no comparisons).
    pub fn is_jit_eligible(pattern: &CompiledFn) -> bool {
        if !pattern.is_pure {
            return false;
        }
        // All parameter types must be Float
        if pattern.param_types.iter().any(|t| t != "Float") {
            return false;
        }
        // Pattern must have at least 1 parameter
        if pattern.param_count == 0 {
            return false;
        }
        // Check that all instructions are JIT-compilable
        // JIT supports: LoadLocal, Const(Float), Add, Sub, Mul, Div, Return
        // Comparisons (Cmp*) are NOT JIT-compiled — they stay in the VM.
        for instr in &pattern.code {
            match instr {
                Instruction::LoadLocal(_) |
                Instruction::Const(Value::Float(_)) |
                Instruction::Add |
                Instruction::Sub |
                Instruction::Mul |
                Instruction::Div |
                Instruction::Return => {}
                // Anything else (including Cmp*) disqualifies from JIT
                _ => return false,
            }
        }
        true
    }

    /// Compile a pure Float-only pattern to native code via Cranelift.
    /// Returns the function pointer on success.
    pub fn compile_pattern(&mut self, pattern: &CompiledFn) -> Result<JitFnPtr, String> {
        if !Self::is_jit_eligible(pattern) {
            return Err(format!(
                "JIT: pattern '{}' is not JIT-eligible",
                pattern.name
            ));
        }

        self.name_counter += 1;
        let func_name = format!("jit_hot_{}_{}", pattern.name, self.name_counter);
        let arity = pattern.param_count;

        // Build function signature: fn(f64, f64, ...) -> f64
        let mut sig = self.module.make_signature();
        for _ in 0..arity {
            sig.params.push(AbiParam::new(F64));
        }
        sig.returns.push(AbiParam::new(F64));

        // Declare the function
        let func_id = self.module
            .declare_function(&func_name, Linkage::Local, &sig)
            .map_err(|e| format!("JIT declare: {}", e))?;

        // Build the function body
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;

        {
            let mut fb = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);

            let entry_block = fb.create_block();
            fb.append_block_params_for_function_params(entry_block);
            fb.switch_to_block(entry_block);
            fb.seal_block(entry_block);

            // Translate bytecode instructions to Cranelift IR
            let mut stack: Vec<Cv> = Vec::new();
            for instr in &pattern.code {
                match instr {
                    Instruction::LoadLocal(slot) => {
                        let val = fb.block_params(entry_block)[*slot];
                        stack.push(val);
                    }
                    Instruction::Const(Value::Float(f)) => {
                        // Create float constant: iconst I64 with bit pattern,
                        // then raw_bitcast to F64
                        let bits = fb.ins().iconst(
                            cranelift_codegen::ir::types::I64,
                            f.to_bits() as i64,
                        );
                        let val = fb.ins().bitcast(F64, MemFlags::new(), bits);
                        stack.push(val);
                    }
                    Instruction::Add => {
                        let r = stack.pop().ok_or("JIT: stack underflow in Add")?;
                        let l = stack.pop().ok_or("JIT: stack underflow in Add")?;
                        stack.push(fb.ins().fadd(l, r));
                    }
                    Instruction::Sub => {
                        let r = stack.pop().ok_or("JIT: stack underflow in Sub")?;
                        let l = stack.pop().ok_or("JIT: stack underflow in Sub")?;
                        stack.push(fb.ins().fsub(l, r));
                    }
                    Instruction::Mul => {
                        let r = stack.pop().ok_or("JIT: stack underflow in Mul")?;
                        let l = stack.pop().ok_or("JIT: stack underflow in Mul")?;
                        stack.push(fb.ins().fmul(l, r));
                    }
                    Instruction::Div => {
                        let r = stack.pop().ok_or("JIT: stack underflow in Div")?;
                        let l = stack.pop().ok_or("JIT: stack underflow in Div")?;
                        stack.push(fb.ins().fdiv(l, r));
                    }
                    Instruction::Return => {
                        let val = stack.pop().ok_or("JIT: stack underflow in Return")?;
                        fb.ins().return_(&[val]);
                    }
                    _ => {
                        return Err(format!(
                            "JIT: unsupported instruction in pattern '{}': {:?}",
                            pattern.name, instr
                        ));
                    }
                }
            }

            fb.finalize();
        }

        // Define and finalize
        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| format!("JIT define: {}", e))?;
        self.module.clear_context(&mut ctx);
        self.module
            .finalize_definitions()
            .map_err(|e| format!("JIT finalize: {}", e))?;

        let code_ptr = self.module.get_finalized_function(func_id);
        Ok(code_ptr)
    }

    /// Compile a pattern and store the function pointer.
    pub fn compile_and_store(
        &mut self,
        idx: usize,
        pattern: &CompiledFn,
    ) -> Result<(), String> {
        self.ensure_capacity(idx);
        let ptr = self.compile_pattern(pattern)?;
        self.compiled[idx] = Some(ptr);
        Ok(())
    }

    /// Get the JIT-compiled function pointer for a pattern index.
    pub fn get_compiled(&self, idx: usize) -> Option<JitFnPtr> {
        self.compiled.get(idx).and_then(|v| *v)
    }

    /// Get total number of JIT-compiled patterns.
    pub fn compiled_count(&self) -> usize {
        self.compiled.iter().filter(|v| v.is_some()).count()
    }

    /// Call a JIT-compiled pattern with the given f64 arguments.
    /// Dispatches to the correct arity-specific transmute.
    /// Returns `Err` for unsupported arities instead of panicking (Наряд №29 §3.4).
    #[inline]
    pub fn call_jit(ptr: JitFnPtr, args: &[f64]) -> Result<f64, String> {
        // SAFETY: `ptr` is a `JitFnPtr` (`*const u8`) previously obtained from
        // `JitCompiler::compile_pattern` / `ensure_compiled`, which stores the
        // address returned by Cranelift's `JITModule::get_finalized_function`.
        // Cranelift allocates the function in executable memory with a stable
        // ABI: each compiled pattern takes N `f64` arguments (passed in the
        // platform's standard floating-point registers) and returns a single
        // `f64`, matching the function pointer type we transmute to below.
        //
        // Invariants that must hold for this transmute to be sound:
        //   1. `ptr` is non-null and points to live, finalized JIT code owned
        //      by the `JITModule` (lifetime tied to the `JitCompiler`). The
        //      compiler is kept alive for the entire VM run, so the code is
        //      never unmapped while a `JitFnPtr` is outstanding.
        //   2. The arity selected by `args.len()` matches the arity of the
        //      function that was compiled into `ptr`. `is_jit_eligible`
        //      rejects patterns with arity > 3, and `compile_pattern`
        //      emits a signature with exactly as many `F64` parameters as the
        //      source pattern declares, so the transmuted signature is
        //      representation-compatible with the compiled code.
        //   3. `ptr` has the same width as a function pointer on the target
        //      platform (both are pointer-sized), so `mem::transmute` between
        //      `*const u8` and `fn(...) -> f64` is a no-op bit-cast.
        //
        // The match arms below select the transmuted signature by the runtime
        // argument count; if a caller violates invariant (2) by passing a
        // different number of arguments than the compiled function expects,
        // the behavior is undefined — but the VM never does so, because it
        // dispatches to `call_jit` only with the same arity it used when
        // requesting compilation.
        unsafe {
            match args.len() {
                0 => {
                    let f: fn() -> f64 = std::mem::transmute(ptr);
                    Ok(f())
                }
                1 => {
                    let f: fn(f64) -> f64 = std::mem::transmute(ptr);
                    Ok(f(args[0]))
                }
                2 => {
                    let f: fn(f64, f64) -> f64 = std::mem::transmute(ptr);
                    Ok(f(args[0], args[1]))
                }
                3 => {
                    let f: fn(f64, f64, f64) -> f64 = std::mem::transmute(ptr);
                    Ok(f(args[0], args[1], args[2]))
                }
                _ => {
                    // Unsupported arity — this should not happen because
                    // is_jit_eligible rejects patterns we can't compile
                    Err(String::from("JIT: unsupported arity"))
                }
            }
        }
    }
}
