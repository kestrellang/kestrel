use clap::{Parser, Subcommand};
use kestrel_compiler::{Compilation, CompileError, TargetConfig};
use kestrel_lexer::lex;
use kestrel_parser::{Parser as KestrelParser, parse_source_file};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "kestrel")]
#[command(about = "The Kestrel compiler", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Source files to process
    #[arg(global = true)]
    files: Vec<String>,

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
}

#[derive(Subcommand)]
enum Commands {
    /// Type-check source files
    Check {
        /// Source files to check
        files: Vec<String>,
    },
    /// Parse source files and show syntax tree
    Parse {
        /// Source files to parse
        files: Vec<String>,
    },
    /// Compile and run a program
    Run {
        /// Source files to run
        #[arg(required = true)]
        files: Vec<String>,
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
        /// Source file to build
        file: String,
        /// Source files to process
        #[arg(value_name = "FILES")]
        files: Vec<String>,
        /// Output file path (defaults to input filename without extension)
        #[arg(short, long)]
        output: Option<String>,
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

fn read_source(path: &str) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path, e);
            None
        }
    }
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

fn run_check(
    files: &[String],
    show_tree: Option<&str>,
    show_symbols: bool,
    show_execution_graph: bool,
    verbose: bool,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let mut builder = Compilation::builder();
    let mut io_ok = true;

    for file in files {
        if verbose {
            eprintln!("  Reading {}", file);
        }
        let Some(content) = read_source(file) else {
            io_ok = false;
            continue;
        };
        builder = builder.add_source(file.clone(), content);
    }

    if verbose {
        eprintln!("  Compiling...");
    }
    let compilation = builder.build();

    // Show results
    if let Some(mode) = show_tree {
        if let Some(model) = compilation.semantic_model() {
            let full = mode == "full";
            model.print_semantic_model(full);
        }
    }

    if show_symbols {
        if let Some(model) = compilation.semantic_model() {
            model.print_model_symbols();
        }
    }

    // Lower to execution graph and display
    if show_execution_graph {
        if let Some(model) = compilation.semantic_model() {
            let root = model.root();
            let result = kestrel_execution_graph_lowering::lower_module(model, &root);

            // Show lowering diagnostics if any
            if !result.diagnostics.is_empty() {
                for diag in &result.diagnostics {
                    eprintln!("warning: {:?}", diag);
                }
            }

            // Display the execution graph
            print!("{}", result.mir.display());
        }
    }

    // Emit diagnostics
    let has_errors = !io_ok || compilation.has_errors();
    if has_errors {
        compilation.diagnostics().emit().ok();
        ExitCode::from(1)
    } else {
        if verbose {
            eprintln!("  No errors found.");
        }
        ExitCode::SUCCESS
    }
}

fn run_parse(files: &[String], show_tree: bool) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let mut has_errors = false;

    for file in files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: cannot read '{}': {}", file, e);
                has_errors = true;
                continue;
            }
        };

        let file_id = 0; // Use file_id=0 for parse-only mode
        let tokens: Vec<_> = lex(&content, file_id)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = KestrelParser::parse(&content, tokens.into_iter(), parse_source_file, file_id);

        println!("=== {} ===", file);

        if !result.errors.is_empty() {
            has_errors = true;
            for error in &result.errors {
                println!("error: {}", error.message);
            }
        } else {
            println!("Parsed successfully.");
        }

        if show_tree {
            println!("\n{:#?}", result.tree);
        }

        println!();
    }

    if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_program(
    files: &[String],
    target: Option<&str>,
    verbose: bool,
    libraries: Vec<String>,
    library_paths: Vec<String>,
    frameworks: Vec<String>,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let target_config = match get_target_config(target) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let mut builder = Compilation::builder();
    let mut io_ok = true;

    for file in files {
        if verbose {
            eprintln!("  Reading {}", file);
        }
        let Some(content) = read_source(file) else {
            io_ok = false;
            continue;
        };
        builder = builder.add_source(file.clone(), content);
    }

    if !io_ok {
        return ExitCode::from(1);
    }

    let compilation = builder.build();

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        return ExitCode::from(1);
    }

    if verbose {
        eprintln!("  Compiling and running...");
    }

    let options = kestrel_compiler::CodegenOptions {
        libraries,
        library_paths,
        frameworks,
        ..Default::default()
    };

    match compilation.run(&target_config, &options) {
        Ok(result) => {
            // Print stdout/stderr
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
            }
            ExitCode::from(result.exit_code as u8)
        }
        Err(CompileError::LoweringFailed(diagnostics)) => {
            // Emit lowering diagnostics with full source context
            compilation.diagnostics().emit_additional(&diagnostics).ok();
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run_build(
    files: &[String],
    output: Option<&str>,
    target: Option<&str>,
    verbose: bool,
    libraries: Vec<String>,
    library_paths: Vec<String>,
    frameworks: Vec<String>,
) -> ExitCode {
    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let target_config = match get_target_config(target) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let mut builder = Compilation::builder();
    let mut io_ok = true;

    for file in files {
        if verbose {
            eprintln!("  Reading {}", file);
        }
        let Some(content) = read_source(file) else {
            io_ok = false;
            continue;
        };
        builder = builder.add_source(file.clone(), content);
    }

    if !io_ok {
        return ExitCode::from(1);
    }

    // Determine output path (from first file if not specified)
    let output_path = match output {
        Some(path) => path.to_string(),
        None => {
            // Strip extension from first input file
            let path = Path::new(&files[0]);
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            if cfg!(windows) {
                format!("{}.exe", stem)
            } else {
                stem.to_string()
            }
        }
    };

    let compilation = builder.build();

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        return ExitCode::from(1);
    }

    if verbose {
        eprintln!("  Building {}...", output_path);
    }

    let options = kestrel_compiler::CodegenOptions {
        libraries,
        library_paths,
        frameworks,
        ..Default::default()
    };
    match compilation.build(&target_config, &options, Path::new(&output_path)) {
        Ok(()) => {
            if verbose {
                eprintln!("  Built successfully: {}", output_path);
            }
            ExitCode::SUCCESS
        }
        Err(CompileError::LoweringFailed(diagnostics)) => {
            // Emit lowering diagnostics with full source context
            compilation.diagnostics().emit_additional(&diagnostics).ok();
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
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
        ),
        Some(Commands::Parse { files }) => run_parse(&files, cli.tree.is_some()),
        Some(Commands::Run {
            files,
            libraries,
            library_paths,
            frameworks,
        }) => run_program(
            &files,
            cli.target.as_deref(),
            cli.verbose,
            libraries,
            library_paths,
            frameworks,
        ),
        Some(Commands::Build {
            file,
            files,
            output,
            libraries,
            library_paths,
            frameworks,
        }) => {
            // Combine main file with additional files
            let mut all_files = vec![file];
            all_files.extend(files);
            run_build(
                &all_files,
                output.as_deref(),
                cli.target.as_deref(),
                cli.verbose,
                libraries,
                library_paths,
                frameworks,
            )
        }
        None => {
            // No subcommand: use global files
            if cli.files.is_empty() {
                eprintln!("error: no input files");
                eprintln!("Run 'kestrel --help' for usage.");
                ExitCode::from(1)
            } else {
                run_check(
                    &cli.files,
                    cli.tree.as_deref(),
                    cli.symbols,
                    cli.execution_graph,
                    cli.verbose,
                )
            }
        }
    }
}
