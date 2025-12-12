use clap::{Parser, Subcommand};
use kestrel_lexer::lex;
use kestrel_parser::{Parser as KestrelParser, parse_source_file};
use std::fs;
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

    /// Show semantic tree after analysis
    #[arg(long, global = true)]
    tree: bool,

    /// Show symbol table after analysis
    #[arg(long, global = true)]
    symbols: bool,

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
    /// Compile and run a program (shows semantic analysis results)
    Run {
        /// Source file to run
        file: String,
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

fn run_check(files: &[String], show_tree: bool, show_symbols: bool, verbose: bool) -> ExitCode {
    use kestrel_compiler::Compilation;

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
    if show_tree {
        println!("--- Semantic Tree ---");
        if let Some(model) = compilation.semantic_model() {
            model.print_semantic_model();
            println!();
        }
    }

    if show_symbols {
        println!("--- Symbol Table ---");
        if let Some(model) = compilation.semantic_model() {
            model.print_model_symbols();
            println!();
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

        let tokens: Vec<_> = lex(&content, 0) // Use file_id=0 for parse-only mode
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = KestrelParser::parse(&content, tokens.into_iter(), parse_source_file);

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

fn run_program(file: &str, verbose: bool) -> ExitCode {
    use kestrel_compiler::Compilation;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::expr::{ExprKind, LiteralValue};
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
    use semantic_tree::symbol::Symbol;

    if verbose {
        eprintln!("  Reading {}", file);
    }
    let Some(content) = read_source(file) else {
        return ExitCode::from(1);
    };

    let compilation = Compilation::builder()
        .add_source(file.to_string(), content)
        .build();

    if compilation.has_errors() {
        compilation.diagnostics().emit().ok();
        return ExitCode::from(1);
    }

    let Some(model) = compilation.semantic_model() else {
        eprintln!("error: no semantic model produced");
        return ExitCode::from(1);
    };

    println!("=== Compiled {} ===\n", file);

    // Find and display all functions with bodies
    fn visit_symbol(
        symbol: &std::sync::Arc<dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>>,
        indent: usize,
    ) {
        let prefix = "  ".repeat(indent);
        let name = &symbol.metadata().name().value;
        let kind = symbol.metadata().kind();

        match kind {
            KestrelSymbolKind::Function => {
                // Check for ExecutableBehavior
                let behaviors = symbol.metadata().behaviors();
                let exec = behaviors
                    .iter()
                    .find(|b| matches!(b.kind(), KestrelBehaviorKind::Executable));

                if let Some(exec_behavior) = exec {
                    if let Some(eb) = exec_behavior.as_ref().downcast_ref::<ExecutableBehavior>() {
                        let body = eb.body();
                        let stmt_count = body.statements.len();

                        print!("{}func {}() ", prefix, name);

                        if stmt_count > 0 || body.yield_expr().is_some() {
                            println!("{{");
                            for stmt in &body.statements {
                                let stmt_str = stmt.debug_compact();
                                println!("{}  {}", prefix, stmt_str);
                            }
                            if let Some(yield_expr) = body.yield_expr() {
                                let value_str = yield_expr.debug_compact();
                                println!("{}  -> {}", prefix, value_str);
                            }
                            println!("{}}}", prefix);
                        } else {
                            println!("{{ }}");
                        }
                    }
                } else {
                    // Protocol method or abstract function
                    println!("{}func {}() [no body]", prefix, name);
                }
            }
            KestrelSymbolKind::Struct => {
                println!("{}struct {} {{", prefix, name);
                for child in symbol.metadata().children() {
                    visit_symbol(&child, indent + 1);
                }
                println!("{}}}", prefix);
            }
            KestrelSymbolKind::Protocol => {
                println!("{}protocol {} {{", prefix, name);
                for child in symbol.metadata().children() {
                    visit_symbol(&child, indent + 1);
                }
                println!("{}}}", prefix);
            }
            KestrelSymbolKind::Module => {
                println!("{}module {} {{", prefix, name);
                for child in symbol.metadata().children() {
                    visit_symbol(&child, indent + 1);
                }
                println!("{}}}", prefix);
            }
            KestrelSymbolKind::SourceFile => {
                // SourceFile is a container - visit children directly
                for child in symbol.metadata().children() {
                    visit_symbol(&child, indent);
                }
            }
            KestrelSymbolKind::Field => {
                use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                use kestrel_semantic_tree::symbol::field::FieldSymbol;
                if let Some(field) = symbol.as_ref().downcast_ref::<FieldSymbol>() {
                    let mutability = if field.is_mutable() { "var" } else { "let" };
                    // Get the resolved type from TypedBehavior
                    let behaviors = symbol.metadata().behaviors();
                    let ty = behaviors
                        .iter()
                        .find(|b| matches!(b.kind(), KestrelBehaviorKind::Typed))
                        .and_then(|b| b.as_ref().downcast_ref::<TypedBehavior>())
                        .map(|t| t.ty().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    println!("{}{} {}: {}", prefix, mutability, name, ty);
                }
            }
            _ => {}
        }
    }

    // Visit from root
    for child in model.root().metadata().children() {
        visit_symbol(&child, 0);
    }

    println!("\n=== Success ===");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Check { files }) => {
            // Use subcommand files only (cli.files is redundant due to global arg)
            run_check(&files, cli.tree, cli.symbols, cli.verbose)
        }
        Some(Commands::Parse { files }) => {
            // Use subcommand files only
            run_parse(&files, cli.tree)
        }
        Some(Commands::Run { file }) => run_program(&file, cli.verbose),
        None => {
            // No subcommand: use global files
            if cli.files.is_empty() {
                eprintln!("error: no input files");
                eprintln!("Run 'kestrel --help' for usage.");
                ExitCode::from(1)
            } else {
                run_check(&cli.files, cli.tree, cli.symbols, cli.verbose)
            }
        }
    }
}
