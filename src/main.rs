use clap::{Parser, Subcommand};
use kestrel_lexer::lex;
use kestrel_parser::{parse_source_file, Parser as KestrelParser};
use kestrel_reporting::DiagnosticContext;
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

/// Parse a single file and add it to the builder
fn add_file(
    path: &str,
    builder: &mut kestrel_semantic_tree_builder::SemanticTreeBuilder,
    diagnostics: &mut DiagnosticContext,
    verbose: bool,
) -> bool {
    if verbose {
        eprintln!("  Parsing {}", path);
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path, e);
            return false;
        }
    };

    // Add file to diagnostics first to get file_id for spans
    let file_id = diagnostics.add_file(path.to_string(), content.clone());

    // Lex the entire file
    let tokens: Vec<_> = lex(&content, file_id)
        .filter_map(|t| t.ok())
        .map(|spanned| (spanned.value, spanned.span))
        .collect();

    // Parse the entire file
    let result = KestrelParser::parse(&content, tokens.into_iter(), parse_source_file);

    if !result.errors.is_empty() {
        for error in &result.errors {
            let span = error.span.clone().unwrap_or(kestrel_span::Span::from(0..1));
            let diagnostic = kestrel_reporting::Diagnostic::error()
                .with_message(&error.message)
                .with_labels(vec![kestrel_reporting::Label::primary(span.file_id, span.range())]);
            diagnostics.add_diagnostic(diagnostic);
        }
        return false;
    }

    // Add to semantic tree
    builder.add_file(path, &result.tree, &content, diagnostics, file_id);

    true
}

fn run_check(files: &[String], show_tree: bool, show_symbols: bool, verbose: bool) -> ExitCode {
    use kestrel_semantic_tree_builder::{SemanticTreeBuilder, SemanticBinder};

    if files.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(1);
    }

    let mut builder = SemanticTreeBuilder::new();
    let mut diagnostics = DiagnosticContext::new();
    let mut parse_ok = true;

    // Parse all files
    for file in files {
        if !add_file(file, &mut builder, &mut diagnostics, verbose) {
            parse_ok = false;
        }
    }

    // Build the semantic tree
    let tree = builder.build();

    // Run binding phase
    if verbose {
        eprintln!("  Running semantic analysis...");
    }
    let model = SemanticBinder::bind(tree, &mut diagnostics);

    // Show results
    if show_tree {
        println!("--- Semantic Tree ---");
        kestrel_semantic_tree_builder::print_semantic_model(&model);
        println!();
    }

    if show_symbols {
        println!("--- Symbol Table ---");
        kestrel_semantic_tree_builder::print_model_symbols(&model);
        println!();
    }

    // Emit diagnostics
    let has_errors = diagnostics.len() > 0 || !parse_ok;
    if has_errors {
        diagnostics.emit().ok();
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

        let tokens: Vec<_> = lex(&content, 0)  // Use file_id=0 for parse-only mode
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
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::expr::{ExprKind, LiteralValue};
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
    use kestrel_semantic_tree_builder::{SemanticTreeBuilder, SemanticBinder};
    use semantic_tree::symbol::Symbol;

    let mut builder = SemanticTreeBuilder::new();
    let mut diagnostics = DiagnosticContext::new();

    // Parse the file
    if !add_file(file, &mut builder, &mut diagnostics, verbose) {
        diagnostics.emit().ok();
        return ExitCode::from(1);
    }

    // Build the semantic tree
    let tree = builder.build();

    // Run binding phase
    let model = SemanticBinder::bind(tree, &mut diagnostics);

    // Check for errors
    if diagnostics.len() > 0 {
        diagnostics.emit().ok();
        return ExitCode::from(1);
    }

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
                let exec = behaviors.iter().find(|b| {
                    matches!(b.kind(), KestrelBehaviorKind::Executable)
                });

                if let Some(exec_behavior) = exec {
                    if let Some(eb) = exec_behavior.as_ref().downcast_ref::<ExecutableBehavior>() {
                        let body = eb.body();
                        let stmt_count = body.statements.len();

                        print!("{}func {}() ", prefix, name);

                        if stmt_count > 0 || body.yield_expr().is_some() {
                            println!("{{");
                            for stmt in &body.statements {
                                let stmt_str = format_statement(stmt);
                                println!("{}  {}", prefix, stmt_str);
                            }
                            if let Some(yield_expr) = body.yield_expr() {
                                let value_str = format_expr_value(yield_expr);
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
                use kestrel_semantic_tree::symbol::field::FieldSymbol;
                use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                if let Some(field) = symbol.as_ref().downcast_ref::<FieldSymbol>() {
                    let mutability = if field.is_mutable() { "var" } else { "let" };
                    // Get the resolved type from TypedBehavior
                    let behaviors = symbol.metadata().behaviors();
                    let ty = behaviors.iter()
                        .find(|b| matches!(b.kind(), KestrelBehaviorKind::Typed))
                        .and_then(|b| b.as_ref().downcast_ref::<TypedBehavior>())
                        .map(|t| format_type_simple(t.ty()))
                        .unwrap_or_else(|| "<unknown>".to_string());
                    println!("{}{} {}: {}", prefix, mutability, name, ty);
                }
            }
            _ => {}
        }
    }

    fn format_expr_value(expr: &kestrel_semantic_tree::expr::Expression) -> String {
        match &expr.kind {
            ExprKind::Literal(lit) => match lit {
                LiteralValue::Unit => "()".to_string(),
                LiteralValue::Integer(n) => n.to_string(),
                LiteralValue::Float(f) => f.to_string(),
                LiteralValue::String(s) => format!("\"{}\"", s),
                LiteralValue::Bool(b) => b.to_string(),
            },
            ExprKind::Array(elements) => {
                let items: Vec<_> = elements.iter().map(format_expr_value).collect();
                format!("[{}]", items.join(", "))
            }
            ExprKind::Tuple(elements) => {
                let items: Vec<_> = elements.iter().map(format_expr_value).collect();
                format!("({})", items.join(", "))
            }
            ExprKind::LocalRef(id) => format!("local_{}", id.0),
            ExprKind::SymbolRef(id) => format!("symbol_{:?}", id),
            ExprKind::OverloadedRef(_) => "overloaded".to_string(),
            ExprKind::Grouping(inner) => format!("({})", format_expr_value(inner)),
            ExprKind::FieldAccess { object, field } => {
                format!("{}.{}", format_expr_value(object), field)
            }
            ExprKind::TupleIndex { tuple, index } => {
                format!("{}.{}", format_expr_value(tuple), index)
            }
            ExprKind::MethodRef { receiver, method_name, .. } => {
                format!("{}.{}", format_expr_value(receiver), method_name)
            }
            ExprKind::Call { callee, arguments, .. } => {
                let args: Vec<String> = arguments.iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, format_expr_value(&a.value))
                        } else {
                            format_expr_value(&a.value)
                        }
                    })
                    .collect();
                format!("{}({})", format_expr_value(callee), args.join(", "))
            }
            ExprKind::PrimitiveMethodCall { receiver, method, arguments } => {
                let args: Vec<String> = arguments.iter()
                    .map(|a| format_expr_value(&a.value))
                    .collect();
                format!("{}.{}({})", format_expr_value(receiver), method.name(), args.join(", "))
            }
            ExprKind::ImplicitStructInit { struct_type, arguments } => {
                let args: Vec<String> = arguments.iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, format_expr_value(&a.value))
                        } else {
                            format_expr_value(&a.value)
                        }
                    })
                    .collect();
                format!("{}({})", format_type_simple(struct_type), args.join(", "))
            }
            ExprKind::TypeRef(id) => format!("type_{:?}", id),
            ExprKind::Assignment { target, value } => {
                format!("{} = {}", format_expr_value(target), format_expr_value(value))
            }
            ExprKind::If { condition, then_branch: _, then_value, else_branch } => {
                let then_str = then_value.as_ref()
                    .map(|v| format_expr_value(v))
                    .unwrap_or_else(|| "()".to_string());
                let else_str = if else_branch.is_some() { " else { ... }" } else { "" };
                format!("if {} {{ {} }}{}", format_expr_value(condition), then_str, else_str)
            }
            ExprKind::While { condition, .. } => {
                format!("while {} {{ ... }}", format_expr_value(condition))
            }
            ExprKind::Loop { .. } => {
                "loop { ... }".to_string()
            }
            ExprKind::Break { label, .. } => {
                if let Some(l) = label {
                    format!("break {}", l.name)
                } else {
                    "break".to_string()
                }
            }
            ExprKind::Continue { label, .. } => {
                if let Some(l) = label {
                    format!("continue {}", l.name)
                } else {
                    "continue".to_string()
                }
            }
            ExprKind::Return { value } => {
                if let Some(v) = value {
                    format!("return {}", format_expr_value(v))
                } else {
                    "return".to_string()
                }
            }
            ExprKind::TypeParameterRef(_) => "<type_param>".to_string(),
            ExprKind::Error => "<error>".to_string(),
        }
    }

    fn format_statement(stmt: &kestrel_semantic_tree::stmt::Statement) -> String {
        use kestrel_semantic_tree::stmt::StatementKind;
        use kestrel_semantic_tree::pattern::{PatternKind, Mutability};

        match &stmt.kind {
            StatementKind::Binding { pattern, value } => {
                let keyword = match &pattern.kind {
                    PatternKind::Local { mutability, .. } => {
                        if *mutability == Mutability::Mutable { "var" } else { "let" }
                    }
                    PatternKind::Error => "let",
                };
                let name = pattern.name().unwrap_or("<error>");
                let value_str = value.as_ref()
                    .map(|v| format!(" = {}", format_expr_value(v)))
                    .unwrap_or_default();
                format!("{} {}{};", keyword, name, value_str)
            }
            StatementKind::Expr(expr) => {
                format!("{};", format_expr_value(expr))
            }
        }
    }

    fn format_type_simple(ty: &kestrel_semantic_tree::ty::Ty) -> String {
        use kestrel_semantic_tree::ty::TyKind;
        match ty.kind() {
            TyKind::Unit => "()".to_string(),
            TyKind::Never => "!".to_string(),
            TyKind::Int(bits) => format!("{:?}", bits),
            TyKind::Float(bits) => format!("{:?}", bits),
            TyKind::Bool => "Bool".to_string(),
            TyKind::String => "String".to_string(),
            TyKind::Tuple(elements) => {
                let items: Vec<_> = elements.iter().map(format_type_simple).collect();
                format!("({})", items.join(", "))
            }
            TyKind::Array(elem) => format!("[{}]", format_type_simple(elem)),
            TyKind::Function { params, return_type } => {
                let params_str: Vec<_> = params.iter().map(format_type_simple).collect();
                format!("({}) -> {}", params_str.join(", "), format_type_simple(return_type))
            }
            TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
            TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
            TyKind::TypeParameter(param) => param.metadata().name().value.clone(),
            TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
            TyKind::AssociatedType { symbol, .. } => symbol.metadata().name().value.clone(),
            TyKind::SelfType => "Self".to_string(),
            TyKind::TypeVar(_) => "_".to_string(),
            TyKind::Error => "<error>".to_string(),
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
        Some(Commands::Run { file }) => {
            run_program(&file, cli.verbose)
        }
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
