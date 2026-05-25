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

use kestrel_codegen::TargetConfig;
use kestrel_mir_3::mono::MonoModule;

pub use error::CodegenError;

#[derive(Clone)]
pub struct CodegenOptions {
    pub opt_level: u8,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
    pub frameworks: Vec<String>,
    pub c_sources: Vec<std::path::PathBuf>,
    pub emit_clif: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            opt_level: 0,
            libraries: Vec::new(),
            library_paths: Vec::new(),
            frameworks: Vec::new(),
            c_sources: Vec::new(),
            emit_clif: false,
        }
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
    let obj_name = format!("kestrel_{}.o", std::process::id());
    let obj_path = tmp_dir.join(obj_name);
    std::fs::write(&obj_path, &result.object_bytes)?;

    link::link_executable(&obj_path, output_path.as_ref(), target, options)?;

    let _ = std::fs::remove_file(&obj_path);
    Ok(())
}
