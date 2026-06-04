//! Kestrel CLI driver (lib).
//!
//! `kestrel build` compiles source files into an executable.
//! `kestrel dump <kind>` prints compiler-internal representations for triage.
//!
//! Dump kinds today: tokens, cst, mir, cranelift, diagnostics.
//!
//! For `mir`, `--stage <s>` (`-s`) selects which pipeline stage to print
//! (`--list-stages` lists them; `-s all` prints every stage). Default is `verify`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use kestrel_ast_builder::{Os as AstOs, TargetConfig as AstTargetConfig};
use kestrel_codegen::TargetConfig as CodegenTargetConfig;
use kestrel_codegen_cranelift as cranelift_backend;
use kestrel_compiler::{Compiler, Severity};
use kestrel_compiler_driver::CompilerDriver;
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
}

#[derive(Args)]
struct DumpArgs {
    /// Which representation to print.
    kind: DumpKind,

    /// Source files (.ks) to process.
    #[arg(required_unless_present = "list_stages")]
    files: Vec<String>,

    /// Filter output to functions whose name contains this substring.
    #[arg(long = "function", short = 'f')]
    function_filter: Option<String>,

    /// For `mir`/`mir`: which pipeline stage to print (see `--list-stages`).
    /// Defaults to `verify`. `all` prints every stage.
    #[arg(long = "stage", short = 's', value_enum)]
    stage: Option<DumpStage>,

    /// List the MIR pipeline stages (for `--stage`) and exit.
    #[arg(long = "list-stages")]
    list_stages: bool,
}

/// Stage selector for `kestrel dump mir -s <stage>`. Variants map 1:1 onto
/// [`kestrel_mir::passes::Stage`]; `All` is the meta-stage that prints them
/// all. clap derives the kebab spellings (`DropFix` → `drop-fix`), which match
/// `Stage::name()`.
#[derive(ValueEnum, Clone, Copy)]
enum DumpStage {
    Raw,
    DropFix,
    Thunk,
    DropShim,
    CloneShim,
    Layout,
    Verify,
    Mono,
    CopyProp,
    Expand,
    All,
}

impl DumpStage {
    /// The corresponding pipeline stage, or `None` for `All`.
    fn to_mir(self) -> Option<kestrel_mir::passes::Stage> {
        use kestrel_mir::passes::Stage;
        Some(match self {
            DumpStage::Raw => Stage::Raw,
            DumpStage::DropFix => Stage::DropFix,
            DumpStage::Thunk => Stage::Thunk,
            DumpStage::DropShim => Stage::DropShim,
            DumpStage::CloneShim => Stage::CloneShim,
            DumpStage::Layout => Stage::Layout,
            DumpStage::Verify => Stage::Verify,
            DumpStage::Mono => Stage::Mono,
            DumpStage::CopyProp => Stage::CopyProp,
            DumpStage::Expand => Stage::Expand,
            DumpStage::All => return None,
        })
    }
}

#[derive(ValueEnum, Clone, Copy)]
enum DumpKind {
    /// Token stream from the lexer.
    Tokens,
    /// Concrete syntax tree from the parser.
    Cst,
    /// MIR (OSSA) module.
    Mir,
    /// Cranelift IR via the MIR (OSSA) pipeline.
    Cranelift,
    /// All accumulated diagnostics (lex, parse, infer, analyze).
    Diagnostics,
}

// ============================================================================
// Build
// ============================================================================

fn build(globals: &Globals, args: BuildArgs) -> Result<(), ExitCode> {
    let (compiler, std_dir) = globals.load_compiler(&args.files)?;
    let driver = CompilerDriver::new(&compiler);
    driver.infer_all();
    // `build` produces an executable, so analysis enforces the `@main`
    // entry-point requirement (E618).
    let analyze_summary = driver.analyze_all(true);

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

    let options = cranelift_backend::CodegenOptions {
        opt_level: args.opt_level,
        libraries: args.libraries,
        library_paths: args.library_paths,
        frameworks: args.frameworks,
        c_sources,
        ..Default::default()
    };
    let result = compiler.compile_and_link(&output_path, &options);
    driver.emit_diagnostics().ok();
    result.map_err(|e| {
        eprintln!("error: {}", e);
        ExitCode::FAILURE
    })?;

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
    // `--list-stages`: print the MIR pipeline stages and exit (no files needed).
    if args.list_stages {
        print_stage_list();
        return Ok(());
    }
    // `--stage` only makes sense for the MIR dump.
    if args.stage.is_some() && !matches!(args.kind, DumpKind::Mir) {
        eprintln!("error: --stage is only valid with `mir` (alias `mir`)");
        return Err(ExitCode::FAILURE);
    }

    // Tokens and CST are per-file lex/parse — no stdlib, no inference.
    if matches!(args.kind, DumpKind::Tokens | DumpKind::Cst) {
        return dump_syntax(args.kind, &args.files, globals.verbose);
    }

    let (compiler, _std_dir) = globals.load_compiler(&args.files)?;
    let driver = CompilerDriver::new(&compiler);
    driver.infer_all();
    // `dump` is not producing a binary — don't require a `@main`.
    driver.analyze_all(false);

    match args.kind {
        DumpKind::Mir => {
            dump_mir(&compiler, args.stage, args.function_filter.as_deref())?;
        },
        DumpKind::Cranelift => {
            let mir = compiler.lower_to_mir().map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            })?;
            let mono = compiler.monomorphize_mir(mir).map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            })?;
            let target = globals.codegen_target()?;
            let options = cranelift_backend::CodegenOptions {
                emit_clif: true,
                ..Default::default()
            };
            match cranelift_backend::compile(&mono, &target, &options) {
                Ok(result) => {
                    for (name, clif) in &result.clif_text {
                        println!("; function: {name}");
                        print!("{clif}");
                        println!();
                    }
                }
                Err(e) => {
                    driver.emit_diagnostics().ok();
                    eprintln!("error: {e}");
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

/// Print the MIR module at the requested `stage` (default `verify`).
///
/// `verify` (and the default) preserves the historical abort-on-verify-error
/// behavior. Every other stage is best-effort: it prints whatever the stage
/// produced and surfaces any verify errors as stderr warnings. The only hard
/// failure off the default path is a monomorphization error (no module to show).
fn dump_mir(
    compiler: &Compiler,
    stage: Option<DumpStage>,
    filter: Option<&str>,
) -> Result<(), ExitCode> {
    use kestrel_mir::passes::Stage;

    match stage.unwrap_or(DumpStage::Verify).to_mir() {
        // Default / explicit `verify`: keep aborting on verify error.
        Some(Stage::Verify) => {
            let mir = compiler.lower_to_mir().map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            })?;
            print_mir(&mir, filter);
        },
        // Pre-mono intermediate stages (raw..layout): best-effort, no verify.
        Some(s) if s.is_pre_mono() => {
            let (mir, _errors) = compiler.lower_to_mir_stage(s);
            print_mir(&mir, filter);
        },
        // Post-mono stages (mono / copy-prop / expand): best-effort; only a hard
        // monomorphization failure aborts (there'd be no module to print).
        Some(s) => {
            let mir = compiler.lower_to_mir().map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            })?;
            let (mono, mono_errors) = compiler.monomorphize_mir_until(mir, s).map_err(|e| {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            })?;
            for e in &mono_errors {
                eprintln!("warning: mono-verify: {}", e.message);
            }
            print_mono(&mono, filter);
        },
        // `--stage all`: every stage, best-effort, never aborts.
        None => dump_all_mir_stages(compiler, filter),
    }
    Ok(())
}

/// Print every pipeline stage, each under a `=== <stage> ===` header. Fully
/// best-effort — re-runs lowering per stage and never aborts.
fn dump_all_mir_stages(compiler: &Compiler, filter: Option<&str>) {
    use kestrel_mir::passes::Stage;

    for s in Stage::ORDER.into_iter().filter(|s| s.is_pre_mono()) {
        println!("=== {} ===", s.name());
        let (mir, _errors) = compiler.lower_to_mir_stage(s);
        print_mir(&mir, filter);
        println!();
    }
    for s in Stage::ORDER.into_iter().filter(|s| s.is_post_mono()) {
        println!("=== {} ===", s.name());
        // Post-mono needs a fully-lowered pre-mono module; re-lower best-effort.
        let (mir, _) = compiler.lower_to_mir_stage(Stage::Verify);
        match compiler.monomorphize_mir_until(mir, s) {
            Ok((mono, mono_errors)) => {
                for e in &mono_errors {
                    eprintln!("warning: mono-verify: {}", e.message);
                }
                print_mono(&mono, filter);
            },
            Err(e) => println!("; <unavailable: {e}>"),
        }
        println!();
    }
}

fn print_mir(mir: &kestrel_mir::MirModule, filter: Option<&str>) {
    match filter {
        Some(f) => print!("{}", kestrel_mir::display::display_module_filtered(mir, f)),
        None => print!("{}", kestrel_mir::display::display_module(mir)),
    }
}

fn print_mono(mono: &kestrel_mir::mono::MonoModule, filter: Option<&str>) {
    match filter {
        Some(f) => print!("{}", kestrel_mir::display::display_mono_module_filtered(mono, f)),
        None => print!("{}", kestrel_mir::display::display_mono_module(mono)),
    }
}

/// Print the ordered MIR pipeline stages for `--list-stages`.
fn print_stage_list() {
    use kestrel_mir::passes::Stage;
    println!("MIR (OSSA) pipeline stages, in order:");
    for s in Stage::ORDER {
        if s == Stage::Verify {
            println!("  {} (default)", s.name());
        } else {
            println!("  {}", s.name());
        }
    }
    println!("  all (every stage, with `=== <stage> ===` headers)");
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
