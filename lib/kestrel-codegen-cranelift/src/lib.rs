pub mod abi;
pub mod c_bridge;
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

use kestrel_codegen::TargetConfig;
use kestrel_mir::mono::MonoModule;

pub(crate) static LINK_COUNTER: AtomicU64 = AtomicU64::new(0);

pub use error::CodegenError;

#[derive(Clone, Default)]
pub struct CodegenOptions {
    pub opt_level: u8,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
    pub frameworks: Vec<String>,
    pub c_sources: Vec<std::path::PathBuf>,
    pub emit_clif: bool,
    /// `--overflow-checks on|off`; `None` derives it from `opt_level`.
    pub overflow_checks_override: Option<bool>,
}

impl CodegenOptions {
    /// Whether integer `+ - *` (and `-minValue`, `minValue / -1`, `pow`)
    /// trap on overflow. On for unoptimized builds, off for optimized ones,
    /// unless `overflow_checks_override` says otherwise
    /// (`docs/design/integer-overflow.md`, ruling O1).
    pub fn overflow_checks(&self) -> bool {
        self.overflow_checks_override.unwrap_or(self.opt_level == 0)
    }
}

pub struct CompilationResult {
    pub object_bytes: Vec<u8>,
    pub clif_text: Vec<(String, String)>,
}

pub fn compile(
    module: &MonoModule,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError> {
    let mut ctx = context::CodegenCtx::new(module, target, options)?;
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
    let obj_name = format!("kestrel_{}_{}.o", std::process::id(), id);
    let obj_path = tmp_dir.join(obj_name);
    std::fs::write(&obj_path, &result.object_bytes)?;

    link::link_executable(&obj_path, output_path.as_ref(), target, options)?;

    let _ = std::fs::remove_file(&obj_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::CodegenOptions;

    /// Overflow checks default on at -O0 and off above it; the flag wins.
    #[test]
    fn overflow_checks_follow_opt_level_unless_overridden() {
        let at = |opt_level, overflow_checks_override| {
            CodegenOptions {
                opt_level,
                overflow_checks_override,
                ..Default::default()
            }
            .overflow_checks()
        };
        assert!(at(0, None));
        assert!(!at(2, None));
        assert!(!at(0, Some(false)));
        assert!(at(2, Some(true)));
    }
}
