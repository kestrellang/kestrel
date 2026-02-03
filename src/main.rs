use clap::{Parser, Subcommand};
use kestrel_compiler::{Compilation, CompileError, TargetConfig};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "kestrel")]
#[command(about = "The Kestrel compiler", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Show semantic tree after analysis (use --tree=full for detailed output)
    #[arg(long, global = true, value_name = "MODE", num_args = 0..=1, default_missing_value = "summary")]
    tree: Option<String>,

    /// Show symbol table after analysis
    #[arg(long, global = true)]
    symbols: bool,

    /// Show execution graph after lowering
    #[arg(long = "execution-graph", visible_alias = "xgraph", global = true)]
    execution_graph: bool,

    /// Target triple for cross-compilation (e.g., x86_64-unknown-linux-gnu)
    #[arg(long, global = true)]
    target: Option<String>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to standard library (overrides default lang/std/)
    #[arg(long = "std", global = true, value_name = "PATH")]
    std_path: Option<String>,

    /// Disable the standard library
    #[arg(long = "no-std", global = true)]
    no_std: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Type-check source files
    Check {
        /// Source files to check
        files: Vec<String>,
    },
    /// Compile and run a program
    Run {
        /// Source files to run
        #[arg(required = true)]
        files: Vec<String>,
        /// Optimization level (0 = none, 1 = speed, 2 = speed+size)
        #[arg(
            short = 'O',
            long = "opt-level",
            value_name = "LEVEL",
            default_value = "0"
        )]
        opt_level: u8,
        /// Link with a library (can be repeated, use :libname.a for static libs)
        #[arg(short = 'l', long = "link", value_name = "LIBRARY")]
        libraries: Vec<String>,
        /// Add library search path (can be repeated)
        #[arg(short = 'L', long = "library-path", value_name = "PATH")]
        library_paths: Vec<String>,
        /// Link with a macOS framework (can be repeated)
        #[arg(long = "framework", value_name = "NAME")]
        frameworks: Vec<String>,
    },
    /// Build an executable
    Build {
        /// Source files to build
        #[arg(required = true)]
        files: Vec<String>,
        /// Output file path (defaults to input filename without extension)
        #[arg(short, long)]
        output: Option<String>,
        /// Optimization level (0 = none, 1 = speed, 2 = speed+size)
        #[arg(
            short = 'O',
            long = "opt-level",
            value_name = "LEVEL",
            default_value = "0"
        )]
        opt_level: u8,
        /// Link with a library (can be repeated, use :libname.a for static libs)
        #[arg(short = 'l', long = "link", value_name = "LIBRARY")]
        libraries: Vec<String>,
        /// Add library search path (can be repeated)
        #[arg(short = 'L', long = "library-path", value_name = "PATH")]
        library_paths: Vec<String>,
        /// Link with a macOS framework (can be repeated)
        #[arg(long = "framework", value_name = "NAME")]
        frameworks: Vec<String>,
    },
}

fn get_target_config(target: Option<&str>) -> Result<TargetConfig, ExitCode> {
    match target {
        Some(triple) => TargetConfig::from_triple(triple).map_err(|e| {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }),
        None => Ok(TargetConfig::host()),
    }
}

fn configure_stdlib(
    builder: kestrel_compiler::CompilationBuilder,
    std_path: Option<&str>,
    no_std: bool,
) -> kestrel_compiler::CompilationBuilder {
    if no_std {
        builder.without_std()
    } else if let Some(path) = std_path {
        builder.with_std_path(path)
    } else {
        builder
    }
}

fn add_source_files(
    mut builder: kestrel_compiler::CompilationBuilder,
    files: &[String],
    verbose: bool,
) -> Result<kestrel_compiler::CompilationBuilder, ExitCode> {
    for file in files {
        if verbose {
            eprintln!("  Reading {}", file);
        }
        // Use add_file to preserve path info for @fileconstant resolution
        builder = match builder.add_file(file) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: failed to read {}: {}", file, e);
                return Err(ExitCode::from(1));
            }
        };
    }
    Ok(builder)
}

fn build_codegen_options(
    opt_level: u8,
    libraries: Vec<String>,
    library_paths: Vec<String>,
    frameworks: Vec<String>,
) -> kestrel_compiler::CodegenOptions {
    kestrel_compiler::CodegenOptions {
        opt_level,
        libraries,
        library_paths,
        frameworks,
        ..Default::default()
    }
}

fn run_check(
    files: &[String],
    show_tree: Option<&str>,
    show_symbols: bool,
    show_execution_graph: bool,
    verbose: bool,
    std_path: Option<&str>,
    no_std: bool,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let builder = Compilation::builder();
    let builder = configure_stdlib(builder, std_path, no_std);
    let builder = match add_source_files(builder, files, verbose) {
        Ok(b) => b,
        Err(code) => return code,
    };

    if verbose {
        eprintln!("  Compiling...");
    }
    let compilation = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        },
    };

    if let Some(mode) = show_tree
        && let Some(model) = compilation.semantic_model()
    {
        model.print_semantic_model(mode == "full");
    }

    if show_symbols && let Some(model) = compilation.semantic_model() {
        model.print_model_symbols();
    }

    if show_execution_graph && let Some(model) = compilation.semantic_model() {
        let root = model.root();
        let result = kestrel_execution_graph_lowering::lower_module(model, root);
        for diag in &result.diagnostics {
            eprintln!("warning: {:?}", diag);
        }
        print!("{}", result.mir.display());
    }

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        ExitCode::from(1)
    } else {
        if verbose {
            eprintln!("  No errors found.");
        }
        ExitCode::SUCCESS
    }
}

fn run_program(
    files: &[String],
    target: Option<&str>,
    verbose: bool,
    opt_level: u8,
    libraries: Vec<String>,
    library_paths: Vec<String>,
    frameworks: Vec<String>,
    std_path: Option<&str>,
    no_std: bool,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let target_config = match get_target_config(target) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let builder = Compilation::builder();
    let builder = configure_stdlib(builder, std_path, no_std);
    let builder = match add_source_files(builder, files, verbose) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compilation = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        },
    };

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        return ExitCode::from(1);
    }

    if verbose {
        eprintln!("  Compiling and running...");
    }

    let options = build_codegen_options(opt_level, libraries, library_paths, frameworks);

    match compilation.run(&target_config, &options) {
        Ok(result) => {
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }
            ExitCode::from(result.exit_code as u8)
        },
        Err(CompileError::LoweringFailed(diagnostics)) => {
            compilation.diagnostics().emit_additional(&diagnostics).ok();
            ExitCode::from(1)
        },
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        },
    }
}

fn run_build(
    files: &[String],
    output: Option<&str>,
    target: Option<&str>,
    verbose: bool,
    opt_level: u8,
    libraries: Vec<String>,
    library_paths: Vec<String>,
    frameworks: Vec<String>,
    std_path: Option<&str>,
    no_std: bool,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let target_config = match get_target_config(target) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let builder = Compilation::builder();
    let builder = configure_stdlib(builder, std_path, no_std);
    let builder = match add_source_files(builder, files, verbose) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let output_path = match output {
        Some(path) => path.to_string(),
        None => {
            let path = Path::new(&files[0]);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            if cfg!(windows) {
                format!("{}.exe", stem)
            } else {
                stem.to_string()
            }
        },
    };

    let compilation = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        },
    };

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        return ExitCode::from(1);
    }

    if verbose {
        eprintln!("  Building {}...", output_path);
    }

    let options = build_codegen_options(opt_level, libraries, library_paths, frameworks);

    match compilation.build(&target_config, &options, Path::new(&output_path)) {
        Ok(()) => {
            if verbose {
                eprintln!("  Built successfully: {}", output_path);
            }
            ExitCode::SUCCESS
        },
        Err(CompileError::LoweringFailed(diagnostics)) => {
            compilation.diagnostics().emit_additional(&diagnostics).ok();
            ExitCode::from(1)
        },
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        },
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Check { files }) => run_check(
            &files,
            cli.tree.as_deref(),
            cli.symbols,
            cli.execution_graph,
            cli.verbose,
            cli.std_path.as_deref(),
            cli.no_std,
        ),
        Some(Commands::Run {
            files,
            opt_level,
            libraries,
            library_paths,
            frameworks,
        }) => run_program(
            &files,
            cli.target.as_deref(),
            cli.verbose,
            opt_level,
            libraries,
            library_paths,
            frameworks,
            cli.std_path.as_deref(),
            cli.no_std,
        ),
        Some(Commands::Build {
            files,
            output,
            opt_level,
            libraries,
            library_paths,
            frameworks,
        }) => run_build(
            &files,
            output.as_deref(),
            cli.target.as_deref(),
            cli.verbose,
            opt_level,
            libraries,
            library_paths,
            frameworks,
            cli.std_path.as_deref(),
            cli.no_std,
        ),
        None => {
            eprintln!("error: no command specified");
            eprintln!("Run 'kestrel --help' for usage.");
            ExitCode::from(1)
        },
    }
}
