//! Kestrel CLI driver (lib).
//!
//! `kestrel build` compiles source files into an executable.
//! `kestrel dump <kind>` prints compiler-internal representations for triage.
//!
//! Dump kinds today: tokens, cst, mir, cranelift, diagnostics.
//! Planned: ast, hir, types, asm (see `DumpKind`).

use clap::{Args, Parser, Subcommand, ValueEnum};
use kestrel_ast_builder::{Os as AstOs, TargetConfig as AstTargetConfig};
use kestrel_codegen::TargetConfig as CodegenTargetConfig;
use kestrel_codegen_cranelift::{self as cranelift_backend, CodegenOptions};
use kestrel_codegen_cranelift_2 as cranelift2_backend;
use kestrel_codegen_cranelift_3 as cranelift3_backend;
use kestrel_compiler::{Compiler, Severity};
use kestrel_compiler_driver::CompilerDriver;
use kestrel_mir_lower::lower_module;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run() -> Result<(), ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build(args) => build(&cli.globals, args),
        Command::Dump(args) => dump(&cli.globals, args),
    }
}

// ============================================================================
// CLI surface
// ============================================================================

#[derive(Parser)]
#[command(name = "kestrel", version, about = "The Kestrel compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    globals: Globals,
}

#[derive(Args)]
struct Globals {
    /// Target triple for cross-compilation (e.g., x86_64-unknown-linux-gnu).
    #[arg(long, global = true)]
    target: Option<String>,

    /// Verbose output.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to standard library (overrides default `lang/std/`).
    #[arg(long = "std", global = true, value_name = "PATH")]
    std_path: Option<String>,

    /// Disable the standard library.
    #[arg(long = "no-std", global = true)]
    no_std: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Build an executable from source files.
    Build(BuildArgs),
    /// Dump a compiler-internal representation to stdout.
    ///
    /// Diagnostics always go to stderr, so `kestrel dump mir f.ks > out.txt`
    /// captures the dump while errors still show in the terminal.
    Dump(DumpArgs),
}

#[derive(Args)]
struct BuildArgs {
    /// Source files (.ks) to compile.
    #[arg(required = true)]
    files: Vec<String>,

    /// Output executable path (defaults to input basename).
    #[arg(short, long)]
    output: Option<String>,

    /// Optimization level: 0 = none, 1 = speed, 2 = speed + size.
    #[arg(
        short = 'O',
        long = "opt-level",
        value_name = "LEVEL",
        default_value = "0"
    )]
    opt_level: u8,

    /// Link with a library (repeatable; use `:libname.a` for static).
    #[arg(short = 'l', long = "link", value_name = "LIBRARY")]
    libraries: Vec<String>,

    /// Add a library search path (repeatable).
    #[arg(short = 'L', long = "library-path", value_name = "PATH")]
    library_paths: Vec<String>,

    /// Link a macOS framework (repeatable).
    #[arg(long = "framework", value_name = "NAME")]
    frameworks: Vec<String>,

    /// Use the legacy MIR-1 codegen backend.
    #[arg(long = "mir-old")]
    mir_old: bool,

    /// Use the experimental MIR-3 (OSSA) codegen backend.
    #[arg(long = "mir3")]
    mir3: bool,
}

#[derive(Args)]
struct DumpArgs {
    /// Which representation to print.
    kind: DumpKind,

    /// Source files (.ks) to process.
    #[arg(required = true)]
    files: Vec<String>,
}

#[derive(ValueEnum, Clone, Copy)]
enum DumpKind {
    /// Token stream from the lexer.
    Tokens,
    /// Concrete syntax tree from the parser.
    Cst,
    /// MIR module after HIR lowering + all passes.
    Mir,
    /// MIR-2 module (new representation, pre-switchover).
    Mir2,
    /// Cranelift IR (CLIF) per function, pre-optimization.
    Cranelift,
    /// Cranelift IR via the new MIR-2 → mono → codegen-2 pipeline.
    Cranelift2,
    /// All accumulated diagnostics (lex, parse, infer, analyze).
    Diagnostics,
    // TODO: future dump kinds — add when display impls exist.
    // - `ast` — ECS entity tree after `build_declarations`. Needs a walker.
    // - `hir` — HIR bodies. `kestrel_hir::Body` has no pretty printer yet.
    // - `types` — per-body `TypedBody`. No display impl yet.
    // - `asm` — native assembly. Easiest path: shell out to `objdump -d` on
    //   `compile_to_object`'s bytes, or enable Cranelift's `disas` feature.
}

// ============================================================================
// MIR pipeline
// ============================================================================

fn lower_with_ownership(
    world: &kestrel_hecs::World,
    root: kestrel_hecs::Entity,
) -> kestrel_mir::MirModule {
    let mut mir = lower_module(world, root);
    kestrel_ownership::run(&mut mir);
    mir.with_all_passes()
}

// ============================================================================
// Build
// ============================================================================

fn build(globals: &Globals, args: BuildArgs) -> Result<(), ExitCode> {
    let (compiler, std_dir) = globals.load_compiler(&args.files)?;
    let driver = CompilerDriver::new(&compiler);
    driver.infer_all();
    let analyze_summary = driver.analyze_all();

    // Flush diagnostics before codegen — better to fail fast on type errors
    // than to surface a confusing MIR-lowering cascade downstream.
    driver.emit_diagnostics().ok();
    emit_analyze_errors(&compiler, &analyze_summary);
    if has_errors(&compiler) || analyze_summary.errors > 0 {
        return Err(ExitCode::FAILURE);
    }

    let output_path = args
        .output
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&args.files));

    if globals.verbose {
        eprintln!("  Building {}...", output_path.display());
    }

    let c_sources = collect_stdlib_c_sources(std_dir.as_deref());

    if args.mir3 {
        let options = cranelift3_backend::CodegenOptions {
            opt_level: args.opt_level,
            libraries: args.libraries,
            library_paths: args.library_paths,
            frameworks: args.frameworks,
            c_sources,
            ..Default::default()
        };
        let result = compiler.compile_and_link3(&output_path, &options);
        driver.emit_diagnostics().ok();
        result.map_err(|e| {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        })?;
    } else if args.mir_old {
        let mir = lower_with_ownership(compiler.world(), compiler.root());
        let target = globals.codegen_target()?;
        let options = CodegenOptions {
            opt_level: args.opt_level,
            libraries: args.libraries,
            library_paths: args.library_paths,
            frameworks: args.frameworks,
            c_sources,
            ..Default::default()
        };
        let result = cranelift_backend::compile_and_link(&mir, &target, &options, &output_path);
        driver.emit_diagnostics().ok();
        result.map_err(|e| {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        })?;
    } else {
        let options = cranelift2_backend::CodegenOptions {
            opt_level: args.opt_level,
            libraries: args.libraries,
            library_paths: args.library_paths,
            frameworks: args.frameworks,
            c_sources,
            ..Default::default()
        };
        let result = compiler.compile_and_link2(&output_path, &options);
        driver.emit_diagnostics().ok();
        result.map_err(|e| {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        })?;
    }

    if has_errors(&compiler) {
        return Err(ExitCode::FAILURE);
    }
    if globals.verbose {
        eprintln!("  Built successfully: {}", output_path.display());
    }
    Ok(())
}

// ============================================================================
// Dump
// ============================================================================

fn dump(globals: &Globals, args: DumpArgs) -> Result<(), ExitCode> {
    // Tokens and CST are per-file lex/parse — no stdlib, no inference.
    if matches!(args.kind, DumpKind::Tokens | DumpKind::Cst) {
        return dump_syntax(args.kind, &args.files, globals.verbose);
    }

    let (compiler, _std_dir) = globals.load_compiler(&args.files)?;
    let driver = CompilerDriver::new(&compiler);
    driver.infer_all();
    driver.analyze_all();

    match args.kind {
        DumpKind::Mir => {
            let mir = lower_with_ownership(compiler.world(), compiler.root());
            print!("{}", mir.display());
        },
        DumpKind::Mir2 => {
            // Always dump the MIR, even with verify errors — this is a debugging command
            let mir2 = compiler.lower_to_mir2_unchecked();
            print!("{}", mir2.display());
            driver.emit_diagnostics().ok();
        },
        DumpKind::Cranelift => {
            let mir = lower_with_ownership(compiler.world(), compiler.root());
            let target = globals.codegen_target()?;
            let options = CodegenOptions {
                emit_clif: true,
                ..Default::default()
            };
            match cranelift_backend::compile(&mir, &target, &options) {
                Ok(result) => {
                    for (name, clif) in &result.clif_text {
                        println!("; function: {}", name);
                        print!("{}", clif);
                        println!();
                    }
                },
                Err(e) => {
                    driver.emit_diagnostics().ok();
                    eprintln!("error: {}", e);
                    return Err(ExitCode::FAILURE);
                },
            }
        },
        DumpKind::Cranelift2 => {
            let mir2 = match compiler.lower_to_mir2() {
                Ok(m) => m,
                Err(_) => {
                    driver.emit_diagnostics().ok();
                    return Err(ExitCode::FAILURE);
                }
            };
            let target_mir2 = kestrel_mir_2::TargetConfig::host_64();
            match kestrel_mir_2::mono::monomorphize(mir2, &target_mir2) {
                Ok(mono) => {
                    let target = globals.codegen_target()?;
                    let options = cranelift2_backend::CodegenOptions {
                        emit_clif: true,
                        ..Default::default()
                    };
                    match cranelift2_backend::compile(&mono, &target, &options) {
                        Ok(result) => {
                            for (name, clif) in &result.clif_text {
                                println!("; function: {}", name);
                                print!("{}", clif);
                                println!();
                            }
                        }
                        Err(e) => {
                            driver.emit_diagnostics().ok();
                            eprintln!("error: {}", e);
                            return Err(ExitCode::FAILURE);
                        }
                    }
                }
                Err(errs) => {
                    for e in &errs {
                        eprintln!("mono error: {}", e);
                    }
                    return Err(ExitCode::FAILURE);
                }
            }
        },
        DumpKind::Diagnostics => {
            // Emitted below; no stdout output for this kind.
        },
        DumpKind::Tokens | DumpKind::Cst => unreachable!("handled above"),
    }

    driver.emit_diagnostics().ok();
    if has_errors(&compiler) {
        Err(ExitCode::FAILURE)
    } else {
        Ok(())
    }
}

fn dump_syntax(kind: DumpKind, files: &[String], verbose: bool) -> Result<(), ExitCode> {
    let mut compiler = Compiler::new();
    for file in files {
        if verbose {
            eprintln!("  Reading {}", file);
        }
        let source = std::fs::read_to_string(file).map_err(|e| {
            eprintln!("error: failed to read {}: {}", file, e);
            ExitCode::FAILURE
        })?;
        let entity = compiler.set_source(file, source);

        match kind {
            DumpKind::Tokens => {
                println!("; tokens: {}", file);
                for tok in compiler.lex(entity) {
                    println!("  {:?} @ {}..{}", tok.value, tok.span.start, tok.span.end);
                }
                println!();
            },
            DumpKind::Cst => {
                println!("; cst: {}", file);
                // SyntaxNode's Debug format is the tree-view CST representation.
                println!("{:#?}", compiler.parse(entity).tree);
            },
            _ => unreachable!("dump_syntax called with non-syntax kind"),
        }
    }

    CompilerDriver::new(&compiler).emit_diagnostics().ok();
    if has_errors(&compiler) {
        Err(ExitCode::FAILURE)
    } else {
        Ok(())
    }
}

// ============================================================================
// Globals helpers
// ============================================================================

impl Globals {
    /// Build a Compiler, loading stdlib (unless `--no-std`) and every input file.
    /// Returns the compiler and the stdlib directory (if loaded).
    fn load_compiler(&self, files: &[String]) -> Result<(Compiler, Option<PathBuf>), ExitCode> {
        let mut compiler = Compiler::new().with_target(self.ast_target());
        let mut std_dir_out = None;

        if !self.no_std {
            let std_dir = match self.std_path.as_deref() {
                Some(p) => PathBuf::from(p),
                None => default_std_path().map_err(|e| {
                    eprintln!("error: could not locate the Kestrel stdlib");
                    for (source, path) in &e.tried {
                        eprintln!("  tried {} -> {}", source, path.display());
                    }
                    eprintln!("hint: set KESTREL_STD or pass --std <path>");
                    ExitCode::FAILURE
                })?,
            };
            if self.verbose {
                eprintln!("  Loading stdlib from {}", std_dir.display());
            }
            compiler.load_dir(&std_dir);
            std_dir_out = Some(std_dir);
        }

        for file in files {
            if self.verbose {
                eprintln!("  Reading {}", file);
            }
            let source = std::fs::read_to_string(file).map_err(|e| {
                eprintln!("error: failed to read {}: {}", file, e);
                ExitCode::FAILURE
            })?;
            let entity = compiler.set_source(file, source);
            compiler.build(entity);
        }
        Ok((compiler, std_dir_out))
    }

    fn codegen_target(&self) -> Result<CodegenTargetConfig, ExitCode> {
        match self.target.as_deref() {
            Some(triple) => CodegenTargetConfig::from_triple(triple).map_err(|e| {
                eprintln!("error: {}", e);
                ExitCode::FAILURE
            }),
            None => Ok(CodegenTargetConfig::host()),
        }
    }

    /// Derive the ast-builder `TargetConfig` (for `@platform` conditionals) from
    /// the requested triple, falling back to host.
    fn ast_target(&self) -> AstTargetConfig {
        let Some(triple) = &self.target else {
            return AstTargetConfig::host();
        };
        let lower = triple.to_ascii_lowercase();
        let os = if lower.contains("darwin") || lower.contains("apple") {
            Some(AstOs::Darwin)
        } else if lower.contains("linux") {
            Some(AstOs::Linux)
        } else {
            None
        };
        AstTargetConfig { os }
    }
}

// ============================================================================
// Small helpers
// ============================================================================

struct StdLookupError {
    tried: Vec<(&'static str, PathBuf)>,
}

/// Locate the stdlib via, in priority order:
///   1. `KESTREL_STD` env var (matches kestrel-test-suite convention)
///   2. `<exe>/../lib/std` (jessup-installed toolchain layout)
///   3. `<CARGO_MANIFEST_DIR>/lang/std` baked at build time (in-repo dev)
///
/// Each candidate must `exists()` to win; otherwise we fall through and
/// surface every path we tried so the user can see what went wrong.
/// Order must stay fixed: explicit override > installed toolchain > repo dev.
fn default_std_path() -> Result<PathBuf, StdLookupError> {
    let mut tried = Vec::new();

    if let Some(p) = std::env::var_os("KESTREL_STD") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        tried.push(("KESTREL_STD", p));
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(p) = exe
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib/std"))
    {
        if p.exists() {
            return Ok(p);
        }
        tried.push(("exe-relative", p));
    }

    let baked = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lang/std");
    if baked.exists() {
        return Ok(baked);
    }
    tried.push(("CARGO_MANIFEST_DIR", baked));

    Err(StdLookupError { tried })
}

/// Collect .c files from the stdlib directory that need to be compiled and linked.
fn collect_stdlib_c_sources(std_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(std_dir) = std_dir else { return vec![] };
    let shim = std_dir.join("io/libc_shims.c");
    if shim.exists() { vec![shim] } else { vec![] }
}

fn default_output_path(files: &[String]) -> PathBuf {
    let stem = Path::new(&files[0])
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    if cfg!(windows) {
        PathBuf::from(format!("{}.exe", stem))
    } else {
        PathBuf::from(stem.as_ref())
    }
}

fn has_errors(compiler: &Compiler) -> bool {
    compiler
        .diagnostics()
        .iter()
        .any(|d| d.severity >= Severity::Error)
}

/// Emit analyzer diagnostics (E-codes) as codespan-style errors to stderr.
fn emit_analyze_errors(compiler: &Compiler, summary: &kestrel_compiler_driver::AnalyzeSummary) {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use kestrel_compiler::diagnostic::WorldFiles;

    let error_diags: Vec<_> = summary
        .diagnostics
        .iter()
        .filter(|d| d.severity == kestrel_analyze::Severity::Error)
        .collect();

    if error_diags.is_empty() {
        return;
    }

    let files = WorldFiles::from_world(compiler.world(), compiler.files());
    let codespan_diags: Vec<Diagnostic<usize>> = error_diags
        .iter()
        .map(|d| {
            let labels = d
                .labels
                .iter()
                .map(|l| {
                    let label = if l.is_primary {
                        Label::primary(l.span.file_id, l.span.range())
                    } else {
                        Label::secondary(l.span.file_id, l.span.range())
                    };
                    label.with_message(&l.message)
                })
                .collect();
            Diagnostic::error()
                .with_message(format!("{} [{}]", d.message, d.descriptor_id))
                .with_labels(labels)
                .with_notes(d.notes.clone())
        })
        .collect();

    kestrel_reporting::emit_all(&files, &codespan_diags).ok();
}
