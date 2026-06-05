//! LLVM code generation backend for Kestrel (lib).
//!
//! Analogous to `kestrel-codegen-cranelift`: consumes a monomorphized MIR
//! `MonoModule` and produces a linkable object file (and optional textual LLVM
//! IR for inspection) via the LLVM C API, wrapped by `inkwell` (LLVM 18).
//!
//! Representation (typed-`ptr`; see `ty`, `abi`, `mem`, `inst`, `terminator`,
//! `func`): pointer-width scalars — addresses, aggregate references,
//! `Pointer`/`FuncThin` scalars, function pointers — are real LLVM `ptr` values,
//! and offset arithmetic is `getelementptr`. This preserves pointer provenance
//! so LLVM's alias analysis can devirtualize indirect calls, hoist loads (LICM),
//! and vectorize. The only `int<->ptr` conversions are the
//! `Op::PtrToAddress`/`Op::PtrFromAddress` boundaries.

pub mod abi;
pub mod context;
pub mod error;
pub mod func;
pub mod imm;
pub mod inst;
pub mod link;
pub mod mem;
pub mod terminator;
pub mod ty;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use inkwell::context::Context;
use kestrel_codegen::TargetConfig;
use kestrel_mir::mono::MonoModule;

pub(crate) static LINK_COUNTER: AtomicU64 = AtomicU64::new(0);

pub use error::CodegenError;

/// Options controlling code generation and linking. Field-compatible with the
/// Cranelift backend's `CodegenOptions`, except `emit_ir`/`ir_text` replace the
/// Cranelift-specific `emit_clif`/`clif_text` (the dumped text is LLVM IR).
#[derive(Clone, Default)]
pub struct CodegenOptions {
    pub opt_level: u8,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
    pub frameworks: Vec<String>,
    pub c_sources: Vec<std::path::PathBuf>,
    /// Emit textual LLVM IR (per function) into `CompilationResult::ir_text`.
    pub emit_ir: bool,
}

pub struct CompilationResult {
    pub object_bytes: Vec<u8>,
    /// `(function name, textual LLVM IR)` pairs, populated when `emit_ir` is set.
    pub ir_text: Vec<(String, String)>,
}

pub fn compile(
    module: &MonoModule,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    // The LLVM `Context` owns every type/value/module created from it and must
    // outlive them. It lives for the duration of `compile`; `CompilationResult`
    // holds only owned data (object bytes + IR strings), so it safely outlives
    // the context.
    let cx = Context::create();
    let mut ctx = context::CodegenCtx::new(&cx, module, target, options)?;
    ctx.compile_all()?;
    ctx.finish()
}

pub fn compile_and_link(
    module: &MonoModule,
    target: &TargetConfig,
    options: &CodegenOptions,
    output_path: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let result = compile(module, target, options)?;

    let tmp_dir = std::env::temp_dir();
    let id = LINK_COUNTER.fetch_add(1, Ordering::Relaxed);
    let obj_name = format!("kestrel_llvm_{}_{}.o", std::process::id(), id);
    let obj_path = tmp_dir.join(obj_name);
    std::fs::write(&obj_path, &result.object_bytes)?;

    link::link_executable(&obj_path, output_path.as_ref(), target, options)?;

    let _ = std::fs::remove_file(&obj_path);
    Ok(())
}
