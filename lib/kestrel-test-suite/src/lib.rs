//! Kestrel Test Suite
//!
//! A fluent test API for testing the Kestrel compiler.
//!
//! # Example
//!
//! ```
//! use kestrel_test_suite::*;
//!
//! fn test_struct() {
//!     Test::new("module Test\nstruct Foo {}")
//!         .expect(Compiles)
//!         .expect(Symbol::new("Foo").is(SymbolKind::Struct));
//! }
//! ```
//!
//! # Test Prelude
//!
//! By default, tests include a prelude module with builtin protocols (`Copyable`, `Cloneable`).
//! Tests can import these with `import Prelude` or `import Prelude.(Copyable, Cloneable)`.
//!
//! ```
//! use kestrel_test_suite::*;
//!
//! // Uses prelude (default)
//! fn test_with_prelude() {
//!     Test::new("module Test\nimport Prelude\nstruct Handle: not Copyable {}")
//!         .expect(Compiles);
//! }
//!
//! // Opt-out for tests that define their own builtin protocols
//! fn test_without_prelude() {
//!     Test::new("module Test\n@builtin(.Copyable)\nprotocol Copyable {}")
//!         .without_prelude()
//!         .expect(Compiles);
//! }
//! ```
//!
//! # Symbol Path Matching
//!
//! Symbols can be found by simple name or by dot-separated path:
//!
//! ```no_run
//! use kestrel_test_suite::*;
//!
//! // Simple name lookup (finds first match anywhere in tree)
//! Symbol::new("Inner");
//!
//! // Path-based lookup (finds Outer, then Inner within it)
//! Symbol::new("Outer.Inner");
//!
//! // Deeply nested paths
//! Symbol::new("Module.Struct.Method");
//! ```
//!
//! # Available Behaviors
//!
//! ```no_run
//! use kestrel_test_suite::*;
//!
//! Symbol::new("Foo")
//!     .is(SymbolKind::Struct)
//!     .has(Behavior::Visibility(Visibility::Public))
//!     .has(Behavior::TypeParamCount(2))
//!     .has(Behavior::IsGeneric(true))
//!     .has(Behavior::FieldCount(3))
//!     .has(Behavior::IsStatic(false))
//!     .has(Behavior::HasBody(true))
//!     .has(Behavior::ParameterCount(2))
//!     .has(Behavior::ConformanceCount(1));
//! ```

pub mod mir;

use std::cell::OnceCell;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Prelude source containing builtin protocols.
///
/// This module is automatically included in tests (unless `.without_prelude()` is called)
/// and provides the `Copyable` and `Cloneable` protocols that tests can import.
pub const PRELUDE_SOURCE: (&str, &str) = (
    "prelude.ks",
    r#"module Prelude

@builtin(.Copyable)
public protocol Copyable {}

@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}

@builtin(.FFISafe)
public protocol FFISafe {}

@builtin(.ControlFlowEnum)
public enum ControlFlow[C, B] {
    case Continue(C)
    case Break(B)
}

@builtin(.TryableProtocol)
public protocol Tryable {
    type Output
    type Early

    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}

@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}

@builtin(.BooleanConditional)
public protocol BooleanConditional {
    func asBool() -> lang.i1
}

@builtin(.Matchable)
public protocol Matchable {
    func matches(other: Self) -> lang.i1
}

// Arithmetic operator protocols
@builtin(.AddOperatorProtocol)
public protocol AddOperatorProtocol {
    @builtin(.AddOperatorMethod)
    func add(rhs: Self) -> Self
}

@builtin(.SubtractOperatorProtocol)
public protocol SubtractOperatorProtocol {
    @builtin(.SubtractOperatorMethod)
    func subtract(rhs: Self) -> Self
}

@builtin(.MultiplyOperatorProtocol)
public protocol MultiplyOperatorProtocol {
    @builtin(.MultiplyOperatorMethod)
    func multiply(rhs: Self) -> Self
}

@builtin(.DivideOperatorProtocol)
public protocol DivideOperatorProtocol {
    @builtin(.DivideOperatorMethod)
    func divide(rhs: Self) -> Self
}

@builtin(.ModuloOperatorProtocol)
public protocol ModuloOperatorProtocol {
    @builtin(.ModuloOperatorMethod)
    func modulo(rhs: Self) -> Self
}

@builtin(.NegateOperatorProtocol)
public protocol NegateOperatorProtocol {
    @builtin(.NegateOperatorMethod)
    func negate() -> Self
}

// Comparison operator protocols
@builtin(.EqualsOperatorProtocol)
public protocol EqualsOperatorProtocol {
    @builtin(.EqualsOperatorMethod)
    func equals(rhs: Self) -> lang.i1
}

@builtin(.NotEqualsOperatorProtocol)
public protocol NotEqualsOperatorProtocol {
    @builtin(.NotEqualsOperatorMethod)
    func notEquals(rhs: Self) -> lang.i1
}

@builtin(.LessThanOperatorProtocol)
public protocol LessThanOperatorProtocol {
    @builtin(.LessThanOperatorMethod)
    func lessThan(rhs: Self) -> lang.i1
}

@builtin(.GreaterThanOperatorProtocol)
public protocol GreaterThanOperatorProtocol {
    @builtin(.GreaterThanOperatorMethod)
    func greaterThan(rhs: Self) -> lang.i1
}

@builtin(.LessOrEqualOperatorProtocol)
public protocol LessOrEqualOperatorProtocol {
    @builtin(.LessOrEqualOperatorMethod)
    func lessThanOrEqual(rhs: Self) -> lang.i1
}

@builtin(.GreaterOrEqualOperatorProtocol)
public protocol GreaterOrEqualOperatorProtocol {
    @builtin(.GreaterOrEqualOperatorMethod)
    func greaterThanOrEqual(rhs: Self) -> lang.i1
}

// Bitwise operator protocols
@builtin(.BitwiseAndOperatorProtocol)
public protocol BitwiseAndOperatorProtocol {
    @builtin(.BitwiseAndOperatorMethod)
    func bitwiseAnd(rhs: Self) -> Self
}

@builtin(.BitwiseOrOperatorProtocol)
public protocol BitwiseOrOperatorProtocol {
    @builtin(.BitwiseOrOperatorMethod)
    func bitwiseOr(rhs: Self) -> Self
}

@builtin(.BitwiseXorOperatorProtocol)
public protocol BitwiseXorOperatorProtocol {
    @builtin(.BitwiseXorOperatorMethod)
    func bitwiseXor(rhs: Self) -> Self
}

@builtin(.ShiftLeftOperatorProtocol)
public protocol ShiftLeftOperatorProtocol {
    @builtin(.ShiftLeftOperatorMethod)
    func shiftLeft(rhs: Self) -> Self
}

@builtin(.ShiftRightOperatorProtocol)
public protocol ShiftRightOperatorProtocol {
    @builtin(.ShiftRightOperatorMethod)
    func shiftRight(rhs: Self) -> Self
}

@builtin(.BitwiseNotOperatorProtocol)
public protocol BitwiseNotOperatorProtocol {
    @builtin(.BitwiseNotOperatorMethod)
    func bitwiseNot() -> Self
}

// Logical operator protocols
@builtin(.LogicalNotOperatorProtocol)
public protocol LogicalNotOperatorProtocol {
    @builtin(.LogicalNotOperatorMethod)
    func logicalNot() -> lang.i1
}

// Literal protocols
@builtin(.ExpressibleByIntLiteral)
public protocol ExpressibleByIntegerLiteral {
    init(intLiteral value: lang.i64)
}

@builtin(.ExpressibleByFloatLiteral)
public protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: lang.f64)
}

@builtin(.ExpressibleByStringLiteral)
public protocol ExpressibleByStringLiteral {
    init(stringLiteral value: lang.str)
}

@builtin(.ExpressibleByBoolLiteral)
public protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: lang.i1)
}

@builtin(.ExpressibleByNilLiteral)
public protocol ExpressibleByNilLiteral {
    init(nilLiteral value: ())
}
"#,
);
use std::sync::Arc;

use kestrel_lexer::lex;
use kestrel_parser::{Parser, parse_source_file};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::callable::ReceiverKind;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::copy_semantics::CopySemanticsBehavior;
use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::visibility::Visibility as SemanticVisibility;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree_binder::{SemanticBinder, SemanticModel};
use kestrel_semantic_tree_builder::SemanticModelBuilder;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol as SymbolTrait;

// Re-export commonly used types
pub use kestrel_semantic_tree::behavior::callable::ReceiverKind as Receiver;
pub use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind as SymbolKind;

/// Load the standard library from disk.
///
/// Searches for the stdlib in the following locations (in order):
/// 1. `KESTREL_STD_PATH` environment variable
/// 2. `lang/std` relative to the Cargo manifest directory (for tests)
/// 3. `lang/std` relative to current directory (development)
/// 4. `lib/std` relative to executable (installed)
fn load_stdlib() -> Result<Vec<(String, String)>, String> {
    use kestrel_compiler::stdlib::{StdLib, StdLibConfig};
    use std::path::PathBuf;

    // First try the CARGO_MANIFEST_DIR-based path (for tests)
    // We need to go up from lib/kestrel-test-suite to the project root
    let manifest_path = option_env!("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()))
        .map(|p| p.join("lang/std"));

    let config = if let Some(path) = manifest_path {
        if path.exists() {
            StdLibConfig::default().with_path(path)
        } else {
            StdLibConfig::default()
        }
    } else {
        StdLibConfig::default()
    };

    match StdLib::load(&config) {
        Ok(Some(stdlib)) => Ok(stdlib.sources),
        Ok(None) => Ok(Vec::new()), // Stdlib disabled
        Err(e) => Err(e.to_string()),
    }
}

/// Visibility levels for test expectations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Fileprivate,
    Internal,
    Public,
}

/// Result of running a compiled program.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Exit code of the program.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

/// Test context containing compilation results
pub struct TestContext {
    pub semantic_model: SemanticModel,
    pub diagnostics: DiagnosticContext,
    pub has_errors: bool,
    /// Lazily computed MIR lowering result
    mir_result: OnceCell<kestrel_execution_graph_lowering::LoweringResult>,
    /// Lazily computed run result
    run_result: OnceCell<Result<RunResult, String>>,
}

impl TestContext {
    /// Get the MIR lowering result, computing it lazily if needed.
    pub fn mir(&self) -> &kestrel_execution_graph_lowering::LoweringResult {
        self.mir_result.get_or_init(|| {
            let root = self.semantic_model.root();
            kestrel_execution_graph_lowering::lower_module(&self.semantic_model, &root)
        })
    }

    /// Get the run result, compiling and running the program lazily if needed.
    pub fn run_result(&self) -> Result<&RunResult, String> {
        self.run_result
            .get_or_init(|| self.compile_and_run())
            .as_ref()
            .map_err(|e| e.clone())
    }

    /// Compile to executable and run, capturing the output.
    fn compile_and_run(&self) -> Result<RunResult, String> {
        // Get MIR (this already handles errors)
        let mir_result = self.mir();
        if !mir_result.diagnostics.is_empty() {
            return Err(format!(
                "MIR lowering failed: {:?}",
                mir_result.diagnostics
            ));
        }

        // Create temp directory
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let temp_dir = std::env::temp_dir().join(format!(
            "kestrel_test_run_{}_{:?}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::thread::current().id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        // Determine executable name
        let exe_name = if cfg!(windows) { "test.exe" } else { "test" };
        let exe_path = temp_dir.join(exe_name);

        // Clone MIR for codegen (we need mutable access)
        let mut mir = mir_result.mir.clone();

        // Compile and link
        use kestrel_codegen::TargetConfig;
        use kestrel_codegen_cranelift::{CodegenOptions, compile_and_link};

        let target = TargetConfig::host();
        let options = CodegenOptions::default();

        if let Err(e) = compile_and_link(&mut mir, &target, &options, &exe_path) {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!("Codegen failed: {}", e));
        }

        // Run the executable
        let result = match Command::new(&exe_path).output() {
            Ok(output) => RunResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Failed to run executable: {}", e));
            }
        };

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(result)
    }
}

/// A test case that can be run against the Kestrel compiler
pub struct Test {
    files: Vec<(String, String)>,
    context: Option<TestContext>,
    /// Whether to include the prelude module with builtin protocols.
    /// Default is `true`. Use `.without_prelude()` to opt out.
    include_prelude: bool,
    /// Whether to include the standard library.
    /// Default is `false`. Use `.with_stdlib()` to enable.
    include_stdlib: bool,
}

impl Test {
    /// Create a new test from a single source string.
    ///
    /// By default, includes the prelude module with builtin protocols.
    /// Use `.without_prelude()` to opt out.
    pub fn new(source: &str) -> Self {
        Test {
            files: vec![("test.ks".to_string(), source.to_string())],
            context: None,
            include_prelude: true,
            include_stdlib: false,
        }
    }

    /// Create a test from multiple source files.
    ///
    /// By default, includes the prelude module with builtin protocols.
    /// Use `.without_prelude()` to opt out.
    pub fn with_files(files: &[(&str, &str)]) -> Self {
        Test {
            files: files
                .iter()
                .map(|(name, content)| (name.to_string(), content.to_string()))
                .collect(),
            context: None,
            include_prelude: true,
            include_stdlib: false,
        }
    }

    /// Include the prelude module with builtin protocols (default behavior).
    ///
    /// The prelude provides `Copyable` and `Cloneable` protocols that can be
    /// imported with `import Prelude` or `import Prelude.(Copyable, Cloneable)`.
    pub fn with_prelude(mut self) -> Self {
        self.include_prelude = true;
        self
    }

    /// Exclude the prelude module.
    ///
    /// Use this for tests that define their own builtin protocols or
    /// specifically test prelude-related behavior.
    pub fn without_prelude(mut self) -> Self {
        self.include_prelude = false;
        self
    }

    /// Include the standard library from `lang/std/`.
    ///
    /// This loads the actual stdlib files and enables std auto-import.
    /// Use this for tests that need access to stdlib types like `Int64`, `String`, etc.
    pub fn with_stdlib(mut self) -> Self {
        self.include_stdlib = true;
        self
    }

    /// Exclude the standard library (default behavior).
    pub fn without_stdlib(mut self) -> Self {
        self.include_stdlib = false;
        self
    }

    /// Compile the test files and store the result
    fn compile(&mut self) {
        if self.context.is_some() {
            return; // Already compiled
        }

        let mut builder = SemanticModelBuilder::new();
        let mut diagnostics = DiagnosticContext::new();
        let mut has_parse_errors = false;

        // Load stdlib files if enabled
        let stdlib_files: Vec<(String, String)> = if self.include_stdlib {
            builder.enable_std_auto_import();
            match load_stdlib() {
                Ok(files) => files,
                Err(e) => {
                    // Create context with error
                    let diagnostic = kestrel_reporting::Diagnostic::error()
                        .with_message(&format!("Failed to load stdlib: {}", e));
                    diagnostics.add_diagnostic(diagnostic);
                    self.context = Some(TestContext {
                        semantic_model: builder.build(),
                        diagnostics,
                        has_errors: true,
                        mir_result: OnceCell::new(),
                        run_result: OnceCell::new(),
                    });
                    return;
                }
            }
        } else {
            Vec::new()
        };

        // Collect all files to compile (stdlib first if enabled, then prelude if not using stdlib, then test files)
        // Note: When using stdlib, we don't include the prelude since stdlib has its own protocol definitions
        let mut all_files: Vec<(&str, &str)> = Vec::new();
        for (name, content) in &stdlib_files {
            all_files.push((name.as_str(), content.as_str()));
        }
        if self.include_prelude && !self.include_stdlib {
            all_files.push((PRELUDE_SOURCE.0, PRELUDE_SOURCE.1));
        }
        for (name, content) in &self.files {
            all_files.push((name.as_str(), content.as_str()));
        }

        // Parse and add all files
        for (file_name, content) in all_files {
            let file_id = diagnostics.add_file(file_name.to_string(), content.to_string());
            let tokens: Vec<_> = lex(content, file_id)
                .filter_map(|t| t.ok())
                .map(|spanned| (spanned.value, spanned.span))
                .collect();

            let result = Parser::parse(content, tokens.into_iter(), parse_source_file, file_id);

            if !result.errors.is_empty() {
                has_parse_errors = true;
                // Add parse errors to diagnostics
                for error in &result.errors {
                    let span = error.span.clone().unwrap_or(Span::new(0, 0..1));
                    let diagnostic = kestrel_reporting::Diagnostic::error()
                        .with_message(&error.message)
                        .with_labels(vec![kestrel_reporting::Label::primary(
                            file_id,
                            span.range(),
                        )]);
                    diagnostics.add_diagnostic(diagnostic);
                }
            }

            builder.add_file(file_name, &result.tree, content, file_id, &mut diagnostics);
        }

        // Build the semantic model (lowering)
        let model = builder.build();

        // Run binding phase on the built model
        let model = SemanticBinder::bind(model, &mut diagnostics);

        // Run analyzers (during migration we mirror builder validations here)
        {
            use kestrel_semantic_analyzers::{
                AnalysisContext, Analyzer, default_analyzers, run_all,
            };
            let mut owned = default_analyzers();
            let mut analyzers: Vec<&mut dyn Analyzer> = Vec::new();
            for a in owned.iter_mut() {
                analyzers.push(a.as_mut());
            }
            let mut ctx = AnalysisContext::new(&model, &mut diagnostics);
            run_all(&mut analyzers, &model, &mut ctx);
        }

        let has_errors = has_parse_errors || diagnostics.has_errors();

        self.context = Some(TestContext {
            semantic_model: model,
            diagnostics,
            has_errors,
            mir_result: OnceCell::new(),
            run_result: OnceCell::new(),
        });
    }

    /// Apply an expectation to this test
    pub fn expect<E: Expectable>(mut self, expectation: E) -> Self {
        self.compile();
        let ctx = self.context.as_ref().unwrap();
        if let Err(e) = expectation.check(ctx) {
            // Emit diagnostics for context
            if !ctx.diagnostics.is_empty() {
                eprintln!("\n--- Compiler Diagnostics ---");
                ctx.diagnostics.emit().ok();
            }
            panic!("Expectation failed: {}", e);
        }
        self
    }
}

/// Trait for test expectations
pub trait Expectable {
    fn check(&self, ctx: &TestContext) -> Result<(), String>;
}

/// Expects compilation to succeed with no errors
pub struct Compiles;

impl Expectable for Compiles {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            Err(format!(
                "Expected compilation to succeed, but got {} error(s)",
                ctx.diagnostics.len()
            ))
        } else {
            Ok(())
        }
    }
}

/// Expects compilation to fail with an error containing a specific message
pub struct HasError(pub &'static str);

impl Expectable for HasError {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if !ctx.has_errors {
            return Err("Expected compilation to fail with an error, but it succeeded".to_string());
        }

        // Check if any error contains the expected message
        let has_matching_error = ctx
            .diagnostics
            .diagnostics()
            .iter()
            .any(|diag| diag.message.contains(self.0));

        if has_matching_error {
            Ok(())
        } else {
            let actual_errors: Vec<_> = ctx
                .diagnostics
                .diagnostics()
                .iter()
                .map(|d| d.message.as_str())
                .collect();
            Err(format!(
                "Expected an error containing '{}', but got: {:?}",
                self.0, actual_errors
            ))
        }
    }
}

/// Expects compilation to fail with exactly N errors
pub struct HasErrorCount(pub usize);

impl Expectable for HasErrorCount {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        let actual = ctx.diagnostics.len();
        if actual == self.0 {
            Ok(())
        } else {
            Err(format!("Expected {} error(s), but got {}", self.0, actual))
        }
    }
}

/// Expects compilation to fail (with any error)
pub struct Fails;

impl Expectable for Fails {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            Ok(())
        } else {
            Err("Expected compilation to fail, but it succeeded".to_string())
        }
    }
}

/// Expects a warning containing a specific message
pub struct HasWarning(pub &'static str);

impl Expectable for HasWarning {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        use kestrel_reporting::Severity;

        // Check if any warning contains the expected message
        let has_matching_warning = ctx
            .diagnostics
            .diagnostics()
            .iter()
            .any(|diag| diag.severity == Severity::Warning && diag.message.contains(self.0));

        if has_matching_warning {
            Ok(())
        } else {
            let warnings: Vec<_> = ctx
                .diagnostics
                .diagnostics()
                .iter()
                .filter(|d| d.severity == Severity::Warning)
                .map(|d| d.message.as_str())
                .collect();
            if warnings.is_empty() {
                Err(format!(
                    "Expected a warning containing '{}', but there were no warnings",
                    self.0
                ))
            } else {
                Err(format!(
                    "Expected a warning containing '{}', but got: {:?}",
                    self.0, warnings
                ))
            }
        }
    }
}

/// Expects no warnings
pub struct NoWarnings;

impl Expectable for NoWarnings {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        use kestrel_reporting::Severity;

        let warnings: Vec<_> = ctx
            .diagnostics
            .diagnostics()
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();

        if warnings.is_empty() {
            Ok(())
        } else {
            let warning_messages: Vec<_> = warnings.iter().map(|d| d.message.as_str()).collect();
            Err(format!(
                "Expected no warnings, but got {}: {:?}",
                warnings.len(),
                warning_messages
            ))
        }
    }
}

// ============================================================================
// Run expectations
// ============================================================================

/// Expects the program to compile, link, and run successfully (exit code 0).
pub struct Runs;

impl Expectable for Runs {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected program to run, but compilation failed with {} error(s)",
                ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.exit_code == 0 {
            Ok(())
        } else {
            Err(format!(
                "Expected program to run successfully (exit code 0), but got exit code {}\nstderr: {}",
                result.exit_code, result.stderr
            ))
        }
    }
}

/// Expects a specific exit code from the program.
pub struct ExitCode(pub i32);

impl Expectable for ExitCode {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected program to run with exit code {}, but compilation failed with {} error(s)",
                self.0, ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.exit_code == self.0 {
            Ok(())
        } else {
            Err(format!(
                "Expected exit code {}, but got {}\nstdout: {}\nstderr: {}",
                self.0, result.exit_code, result.stdout, result.stderr
            ))
        }
    }
}

/// Expects stdout to contain a specific string.
pub struct StdoutContains(pub &'static str);

impl Expectable for StdoutContains {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected stdout to contain '{}', but compilation failed with {} error(s)",
                self.0, ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.stdout.contains(self.0) {
            Ok(())
        } else {
            Err(format!(
                "Expected stdout to contain '{}', but got:\n{}",
                self.0, result.stdout
            ))
        }
    }
}

/// Expects stdout to equal a specific string exactly.
pub struct StdoutEquals(pub &'static str);

impl Expectable for StdoutEquals {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected stdout to equal '{}', but compilation failed with {} error(s)",
                self.0, ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.stdout == self.0 {
            Ok(())
        } else {
            Err(format!(
                "Expected stdout to equal '{}', but got:\n{}",
                self.0, result.stdout
            ))
        }
    }
}

/// Expects stderr to contain a specific string.
pub struct StderrContains(pub &'static str);

impl Expectable for StderrContains {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected stderr to contain '{}', but compilation failed with {} error(s)",
                self.0, ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.stderr.contains(self.0) {
            Ok(())
        } else {
            Err(format!(
                "Expected stderr to contain '{}', but got:\n{}",
                self.0, result.stderr
            ))
        }
    }
}

/// Expects stderr to equal a specific string exactly.
pub struct StderrEquals(pub &'static str);

impl Expectable for StderrEquals {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if ctx.has_errors {
            return Err(format!(
                "Expected stderr to equal '{}', but compilation failed with {} error(s)",
                self.0, ctx.diagnostics.len()
            ));
        }

        let result = ctx.run_result()?;
        if result.stderr == self.0 {
            Ok(())
        } else {
            Err(format!(
                "Expected stderr to equal '{}', but got:\n{}",
                self.0, result.stderr
            ))
        }
    }
}

/// Symbol expectation with chainable behavior checks
///
/// Symbols can be found by simple name or by dot-separated path:
/// - `Symbol::new("Foo")` - finds first symbol named "Foo" anywhere
/// - `Symbol::new("Outer.Inner")` - finds "Inner" within "Outer"
/// - `Symbol::new("Module.Struct.Method")` - finds "Method" within "Struct" within "Module"
pub struct Symbol {
    path: String,
    kind: Option<SymbolKind>,
    behaviors: Vec<Behavior>,
    negated_behaviors: Vec<Behavior>,
}

impl Symbol {
    /// Create a new symbol expectation
    ///
    /// The path can be:
    /// - A simple name: `"Foo"` - finds first match anywhere in the tree
    /// - A dot-separated path: `"Outer.Inner"` - finds Inner within Outer
    pub fn new(path: &str) -> Self {
        Symbol {
            path: path.to_string(),
            kind: None,
            behaviors: Vec::new(),
            negated_behaviors: Vec::new(),
        }
    }

    /// Assert the symbol is of a specific kind
    pub fn is(mut self, kind: SymbolKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Assert the symbol has a specific behavior/property
    pub fn has(mut self, behavior: Behavior) -> Self {
        self.behaviors.push(behavior);
        self
    }

    /// Assert the symbol does NOT have a specific behavior/property
    pub fn not(mut self, behavior: Behavior) -> Self {
        self.negated_behaviors.push(behavior);
        self
    }

    /// Find a symbol by path in the semantic tree
    ///
    /// Supports dot-separated paths like "Outer.Inner.Method"
    fn find_symbol(
        &self,
        root: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        let segments: Vec<&str> = self.path.split('.').collect();

        if segments.is_empty() {
            return None;
        }

        if segments.len() == 1 {
            // Simple name lookup - find anywhere in tree
            return self.find_by_name(root, segments[0]);
        }

        // Path-based lookup - traverse through each segment
        self.find_by_path(root, &segments)
    }

    /// Find a symbol by simple name anywhere in the tree (depth-first, iterative)
    fn find_by_name(
        &self,
        root: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        // Use an explicit stack to avoid stack overflow on deep trees
        let mut stack = vec![root.clone()];

        while let Some(symbol) = stack.pop() {
            // Check if this symbol matches
            if symbol.metadata().name().value == name {
                return Some(symbol);
            }

            // Add children to stack (in reverse order for left-to-right traversal)
            let children = symbol.metadata().children();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }

        None
    }

    /// Find a symbol by dot-separated path
    ///
    /// For path "A.B.C":
    /// 1. Find "A" anywhere in tree
    /// 2. Find "B" as direct or nested child of A
    /// 3. Find "C" as direct or nested child of B
    fn find_by_path(
        &self,
        root: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        segments: &[&str],
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        if segments.is_empty() {
            return None;
        }

        // Find the first segment anywhere in the tree
        let mut current = self.find_by_name(root, segments[0])?;

        // For each remaining segment, find it within the current symbol
        for &segment in &segments[1..] {
            current = self.find_child_by_name(&current, segment)?;
        }

        Some(current)
    }

    /// Find a child symbol by name (searches only within the given parent, iterative)
    fn find_child_by_name(
        &self,
        parent: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        // Use an explicit stack to avoid stack overflow on deep trees
        let mut stack: Vec<Arc<dyn SymbolTrait<KestrelLanguage>>> =
            parent.metadata().children().into_iter().rev().collect();

        while let Some(child) = stack.pop() {
            if child.metadata().name().value == name {
                return Some(child);
            }
            // Add nested children to stack (in reverse order for left-to-right traversal)
            let nested_children = child.metadata().children();
            for nested in nested_children.into_iter().rev() {
                stack.push(nested);
            }
        }
        None
    }
}

impl Expectable for Symbol {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        // If kind is specified AND path is a simple name (no dots), use registry
        // for precise lookup to avoid finding wrong symbols
        // (e.g., type parameters from prelude's ControlFlow[C, B])
        let symbol = if let Some(expected_kind) = &self.kind {
            if !self.path.contains('.') {
                // Simple name - use registry for O(1) lookup by kind
                let matches = ctx
                    .semantic_model
                    .registry()
                    .find_by_kind_and_name(*expected_kind, &self.path);
                if matches.is_empty() {
                    return Err(format!(
                        "Symbol '{}' with kind {:?} not found",
                        self.path, expected_kind
                    ));
                }
                matches.into_iter().next().unwrap()
            } else {
                // Path-based lookup - use tree search
                let root = ctx.semantic_model.root();
                self.find_symbol(root)
                    .ok_or_else(|| format!("Symbol '{}' not found", self.path))?
            }
        } else {
            // No kind specified - fall back to tree search
            let root = ctx.semantic_model.root();
            self.find_symbol(root)
                .ok_or_else(|| format!("Symbol '{}' not found", self.path))?
        };

        // Check kind if specified
        if let Some(expected_kind) = &self.kind {
            let actual_kind = symbol.metadata().kind();
            if actual_kind != *expected_kind {
                return Err(format!(
                    "Symbol '{}' has kind {:?}, expected {:?}",
                    self.path, actual_kind, expected_kind
                ));
            }
        }

        // Check positive behaviors (must match)
        for behavior in &self.behaviors {
            behavior.check_symbol(&self.path, &symbol, false)?;
        }

        // Check negated behaviors (must NOT match)
        for behavior in &self.negated_behaviors {
            behavior.check_symbol(&self.path, &symbol, true)?;
        }

        Ok(())
    }
}

/// Behaviors are properties that symbols can have
#[derive(Clone)]
pub enum Behavior {
    /// Expected visibility
    Visibility(Visibility),
    /// Expected number of type parameters (for generic types)
    TypeParamCount(usize),
    /// Check if symbol is generic (has at least one type parameter)
    IsGeneric(bool),
    /// Expected number of fields (for structs)
    FieldCount(usize),
    /// Check if function is static
    IsStatic(bool),
    /// Check if function has a body
    HasBody(bool),
    /// Expected number of parameters (for functions)
    ParameterCount(usize),
    /// Expected number of protocol conformances
    ConformanceCount(usize),
    /// Check if function is an instance method
    IsInstanceMethod(bool),
    /// Expected receiver kind (for methods)
    ReceiverKind(Receiver),
    /// Check if symbol has children with a specific count
    ChildCount(usize),
    /// Check if method implements a specific protocol method (protocol_name, method_name)
    ImplementsProtocol(&'static str, &'static str),
    /// Check if method does NOT implement any protocol method
    ImplementsProtocolNone,
    /// Check if symbol has a specific attribute by name
    HasAttribute(&'static str),
    /// Expected number of attributes on the symbol
    AttributeCount(usize),
    /// Expected number of arguments for a specific attribute
    AttributeArgCount(&'static str, usize),
    /// Check if symbol has a negative conformance to a specific protocol by name
    HasNegativeConformance(&'static str),
    /// Check if symbol conforms to a specific protocol by name
    ConformsTo(&'static str),
    /// Check if symbol is copyable (has CopySemanticsBehavior::Copyable or Cloneable)
    IsCopyable(bool),
    /// Check if symbol is cloneable (has CopySemanticsBehavior::Cloneable)
    IsCloneable(bool),
    /// Check if struct has a deinit (has DeinitBehavior)
    HasDeinit(bool),
}

impl Behavior {
    fn check_symbol(
        &self,
        path: &str,
        symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        negated: bool,
    ) -> Result<(), String> {
        let result = self.check_symbol_inner(path, symbol);

        if negated {
            // For negated checks, Ok means the condition matched (which we don't want)
            match result {
                Ok(()) => Err(format!(
                    "Symbol '{}' should NOT have {:?}, but it does",
                    path, self
                )),
                Err(_) => Ok(()), // Condition didn't match, which is what we want
            }
        } else {
            result
        }
    }

    fn check_symbol_inner(
        &self,
        path: &str,
        symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    ) -> Result<(), String> {
        match self {
            Behavior::Visibility(expected) => {
                match symbol.metadata().get_behavior::<VisibilityBehavior>() {
                    Some(vb) => {
                        let actual = vb.visibility();
                        let matches = match (actual, expected) {
                            (Some(SemanticVisibility::Public), Visibility::Public) => true,
                            (Some(SemanticVisibility::Private), Visibility::Private) => true,
                            (Some(SemanticVisibility::Internal), Visibility::Internal) => true,
                            (Some(SemanticVisibility::Fileprivate), Visibility::Fileprivate) => {
                                true
                            }
                            (None, Visibility::Internal) => true, // Default is internal
                            _ => false,
                        };
                        if !matches {
                            return Err(format!(
                                "Symbol '{}' has visibility {:?}, expected {:?}",
                                path, actual, expected
                            ));
                        }
                    }
                    None => {
                        // No visibility behavior means internal (default)
                        if *expected != Visibility::Internal {
                            return Err(format!(
                                "Symbol '{}' has no visibility (defaults to internal), expected {:?}",
                                path, expected
                            ));
                        }
                    }
                }
                Ok(())
            }
            Behavior::TypeParamCount(expected) => {
                let count = get_type_param_count(symbol);
                if count != *expected {
                    return Err(format!(
                        "Symbol '{}' has {} type parameter(s), expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::IsGeneric(expected) => {
                let is_generic = get_type_param_count(symbol) > 0;
                if is_generic != *expected {
                    return Err(format!(
                        "Symbol '{}' is{} generic, expected it to be{} generic",
                        path,
                        if is_generic { "" } else { " not" },
                        if *expected { "" } else { " not" }
                    ));
                }
                Ok(())
            }
            Behavior::FieldCount(expected) => {
                let count = get_field_count(symbol);
                if count != *expected {
                    return Err(format!(
                        "Symbol '{}' has {} field(s), expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::IsStatic(expected) => {
                let is_static = get_is_static(symbol);
                if is_static != Some(*expected) {
                    return Err(format!(
                        "Symbol '{}' is_static={:?}, expected {}",
                        path, is_static, expected
                    ));
                }
                Ok(())
            }
            Behavior::HasBody(expected) => {
                let has_body = get_has_body(symbol);
                if has_body != Some(*expected) {
                    return Err(format!(
                        "Symbol '{}' has_body={:?}, expected {}",
                        path, has_body, expected
                    ));
                }
                Ok(())
            }
            Behavior::ParameterCount(expected) => {
                let count = get_parameter_count(symbol);
                if count != Some(*expected) {
                    return Err(format!(
                        "Symbol '{}' has {:?} parameter(s), expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::ConformanceCount(expected) => {
                let count = get_conformance_count(symbol);
                if count != *expected {
                    return Err(format!(
                        "Symbol '{}' has {} conformance(s), expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::IsInstanceMethod(expected) => {
                let is_instance = get_is_instance_method(symbol);
                if is_instance != Some(*expected) {
                    return Err(format!(
                        "Symbol '{}' is_instance_method={:?}, expected {}",
                        path, is_instance, expected
                    ));
                }
                Ok(())
            }
            Behavior::ReceiverKind(expected) => {
                let receiver = get_receiver_kind(symbol);
                if receiver != Some(*expected) {
                    return Err(format!(
                        "Symbol '{}' has receiver {:?}, expected {:?}",
                        path, receiver, expected
                    ));
                }
                Ok(())
            }
            Behavior::ChildCount(expected) => {
                // Count children excluding type parameters (they're structural, not semantic)
                let count = symbol
                    .metadata()
                    .children()
                    .iter()
                    .filter(|c| c.metadata().kind() != SymbolKind::TypeParameter)
                    .count();
                if count != *expected {
                    return Err(format!(
                        "Symbol '{}' has {} children, expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::ImplementsProtocol(protocol_name, method_name) => {
                let implements_info = get_implements_protocol_info(symbol);
                match implements_info {
                    Some((actual_protocol, actual_method)) => {
                        if actual_protocol != *protocol_name || actual_method != *method_name {
                            return Err(format!(
                                "Symbol '{}' implements protocol method '{}.{}', expected '{}.{}'",
                                path, actual_protocol, actual_method, protocol_name, method_name
                            ));
                        }
                    }
                    None => {
                        return Err(format!(
                            "Symbol '{}' does not implement any protocol method, expected '{}.{}'",
                            path, protocol_name, method_name
                        ));
                    }
                }
                Ok(())
            }
            Behavior::ImplementsProtocolNone => {
                let implements_info = get_implements_protocol_info(symbol);
                if let Some((protocol_name, method_name)) = implements_info {
                    return Err(format!(
                        "Symbol '{}' implements protocol method '{}.{}', expected it to implement no protocol methods",
                        path, protocol_name, method_name
                    ));
                }
                Ok(())
            }
            Behavior::HasAttribute(attr_name) => {
                let has_attr = get_has_attribute(symbol, attr_name);
                if !has_attr {
                    return Err(format!(
                        "Symbol '{}' does not have attribute '@{}'",
                        path, attr_name
                    ));
                }
                Ok(())
            }
            Behavior::AttributeCount(expected) => {
                let count = get_attribute_count(symbol);
                if count != *expected {
                    return Err(format!(
                        "Symbol '{}' has {} attribute(s), expected {}",
                        path, count, expected
                    ));
                }
                Ok(())
            }
            Behavior::AttributeArgCount(attr_name, expected) => {
                let count = get_attribute_arg_count(symbol, attr_name);
                match count {
                    Some(actual) if actual == *expected => Ok(()),
                    Some(actual) => Err(format!(
                        "Symbol '{}' attribute '@{}' has {} argument(s), expected {}",
                        path, attr_name, actual, expected
                    )),
                    None => Err(format!(
                        "Symbol '{}' does not have attribute '@{}'",
                        path, attr_name
                    )),
                }
            }
            Behavior::HasNegativeConformance(protocol_name) => {
                let has_neg = has_negative_conformance_to(symbol, protocol_name);
                if has_neg {
                    Ok(())
                } else {
                    Err(format!(
                        "Symbol '{}' does not have negative conformance to '{}'",
                        path, protocol_name
                    ))
                }
            }
            Behavior::ConformsTo(protocol_name) => {
                let conforms = conforms_to_protocol(symbol, protocol_name);
                if conforms {
                    Ok(())
                } else {
                    Err(format!(
                        "Symbol '{}' does not conform to protocol '{}'",
                        path, protocol_name
                    ))
                }
            }
            Behavior::IsCopyable(expected) => {
                let is_copyable = get_is_copyable(symbol);
                match is_copyable {
                    Some(actual) if actual == *expected => Ok(()),
                    Some(actual) => Err(format!(
                        "Symbol '{}' is_copyable={}, expected {}",
                        path, actual, expected
                    )),
                    None => {
                        // No CopySemanticsBehavior means the type defaults to copyable
                        if *expected {
                            Ok(())
                        } else {
                            Err(format!(
                                "Symbol '{}' has no CopySemanticsBehavior (defaults to copyable), expected not copyable",
                                path
                            ))
                        }
                    }
                }
            }
            Behavior::IsCloneable(expected) => {
                let is_cloneable = get_is_cloneable(symbol);
                match is_cloneable {
                    Some(actual) if actual == *expected => Ok(()),
                    Some(actual) => Err(format!(
                        "Symbol '{}' is_cloneable={}, expected {}",
                        path, actual, expected
                    )),
                    None => {
                        // No CopySemanticsBehavior means the type is not cloneable (just copyable)
                        if !*expected {
                            Ok(())
                        } else {
                            Err(format!(
                                "Symbol '{}' has no CopySemanticsBehavior (not cloneable), expected cloneable",
                                path
                            ))
                        }
                    }
                }
            }
            Behavior::HasDeinit(expected) => {
                let has_deinit = get_has_deinit(symbol);
                if has_deinit == *expected {
                    Ok(())
                } else {
                    Err(format!(
                        "Symbol '{}' has_deinit={}, expected {}",
                        path, has_deinit, expected
                    ))
                }
            }
        }
    }
}

impl std::fmt::Debug for Behavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Behavior::Visibility(v) => write!(f, "Visibility({:?})", v),
            Behavior::TypeParamCount(n) => write!(f, "TypeParamCount({})", n),
            Behavior::IsGeneric(b) => write!(f, "IsGeneric({})", b),
            Behavior::FieldCount(n) => write!(f, "FieldCount({})", n),
            Behavior::IsStatic(b) => write!(f, "IsStatic({})", b),
            Behavior::HasBody(b) => write!(f, "HasBody({})", b),
            Behavior::ParameterCount(n) => write!(f, "ParameterCount({})", n),
            Behavior::ConformanceCount(n) => write!(f, "ConformanceCount({})", n),
            Behavior::IsInstanceMethod(b) => write!(f, "IsInstanceMethod({})", b),
            Behavior::ReceiverKind(r) => write!(f, "ReceiverKind({:?})", r),
            Behavior::ChildCount(n) => write!(f, "ChildCount({})", n),
            Behavior::ImplementsProtocol(p, m) => write!(f, "ImplementsProtocol({}, {})", p, m),
            Behavior::ImplementsProtocolNone => write!(f, "ImplementsProtocolNone"),
            Behavior::HasAttribute(name) => write!(f, "HasAttribute({})", name),
            Behavior::AttributeCount(n) => write!(f, "AttributeCount({})", n),
            Behavior::AttributeArgCount(name, n) => write!(f, "AttributeArgCount({}, {})", name, n),
            Behavior::HasNegativeConformance(name) => {
                write!(f, "HasNegativeConformance({})", name)
            }
            Behavior::ConformsTo(name) => write!(f, "ConformsTo({})", name),
            Behavior::IsCopyable(b) => write!(f, "IsCopyable({})", b),
            Behavior::IsCloneable(b) => write!(f, "IsCloneable({})", b),
            Behavior::HasDeinit(b) => write!(f, "HasDeinit({})", b),
        }
    }
}

/// Helper to get type parameter count for a symbol
fn get_type_param_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
    use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
    use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
    use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;

    let symbol_ref: &dyn SymbolTrait<KestrelLanguage> = symbol.as_ref();

    if let Some(s) = symbol_ref.as_any().downcast_ref::<StructSymbol>() {
        return s.type_parameters().len();
    }
    if let Some(f) = symbol_ref.as_any().downcast_ref::<FunctionSymbol>() {
        return f.type_parameters().len();
    }
    if let Some(p) = symbol_ref.as_any().downcast_ref::<ProtocolSymbol>() {
        return p.type_parameters().len();
    }
    if let Some(a) = symbol_ref.as_any().downcast_ref::<TypeAliasSymbol>() {
        return a.type_parameters().len();
    }
    if let Some(e) = symbol_ref.as_any().downcast_ref::<EnumSymbol>() {
        return e.type_parameters().len();
    }
    0
}

/// Helper to get field count for a symbol (struct only)
fn get_field_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
    // Fields are children with kind Field
    symbol
        .metadata()
        .children()
        .iter()
        .filter(|c| c.metadata().kind() == SymbolKind::Field)
        .count()
}

/// Helper to check if a function is static
fn get_is_static(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    let symbol_ref: &dyn SymbolTrait<KestrelLanguage> = symbol.as_ref();

    if let Some(f) = symbol_ref.as_any().downcast_ref::<FunctionSymbol>() {
        return Some(f.is_static());
    }

    // Also check via FunctionDataBehavior
    get_function_data_behavior(symbol).map(|fdb| fdb.is_static())
}

/// Helper to check if a function has a body
fn get_has_body(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;

    let symbol_ref: &dyn SymbolTrait<KestrelLanguage> = symbol.as_ref();

    if let Some(f) = symbol_ref.as_any().downcast_ref::<FunctionSymbol>() {
        return Some(f.has_body());
    }

    // Also check via FunctionDataBehavior
    get_function_data_behavior(symbol).map(|fdb| fdb.has_body())
}

/// Helper to get parameter count for a callable
fn get_parameter_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<usize> {
    symbol
        .metadata()
        .get_behavior::<CallableBehavior>()
        .map(|cb| cb.arity())
}

/// Helper to get conformance count
fn get_conformance_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
    symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| cb.conformances().len())
        .unwrap_or(0)
}

/// Helper to check if a function is an instance method
fn get_is_instance_method(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    symbol
        .metadata()
        .get_behavior::<CallableBehavior>()
        .map(|cb| cb.is_instance_method())
}

/// Helper to get the receiver kind for a function
fn get_receiver_kind(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<ReceiverKind> {
    symbol
        .metadata()
        .get_behavior::<CallableBehavior>()
        .and_then(|cb| cb.receiver())
}

/// Helper to get FunctionDataBehavior from a symbol
fn get_function_data_behavior(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
) -> Option<FunctionDataBehavior> {
    symbol
        .metadata()
        .behaviors()
        .into_iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::FunctionData))
        .and_then(|b| b.as_ref().downcast_ref::<FunctionDataBehavior>().cloned())
}

/// Helper to get ImplementsBehavior protocol and method name from a symbol
/// Returns (protocol_name, method_name) if the symbol implements a protocol method
fn get_implements_protocol_info(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
) -> Option<(String, String)> {
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
    use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;

    // Look for ImplementsBehavior in the symbol's behaviors
    let impl_behavior = symbol
        .metadata()
        .behaviors()
        .into_iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::Implements))
        .and_then(|b| b.as_ref().downcast_ref::<ImplementsBehavior>().cloned())?;

    // We need to look up the symbols by ID to get their names
    // Walk up to find the root
    let protocol_id = impl_behavior.protocol();
    let method_id = impl_behavior.protocol_method();

    let mut current = symbol.clone();
    while let Some(parent) = current.metadata().parent() {
        current = parent;
    }

    // Find protocol and method symbols by ID
    let protocol = find_symbol_by_id(&current, protocol_id)?;
    let method = find_symbol_by_id(&current, method_id)?;

    Some((
        protocol.metadata().name().value.clone(),
        method.metadata().name().value.clone(),
    ))
}

/// Helper to find a symbol by ID in the tree (iterative to avoid stack overflow)
fn find_symbol_by_id(
    root: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    id: semantic_tree::symbol::SymbolId,
) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
    // Use an explicit stack to avoid stack overflow on deep trees
    let mut stack = vec![root.clone()];

    while let Some(symbol) = stack.pop() {
        if symbol.metadata().id() == id {
            return Some(symbol);
        }

        // Add children to stack
        let children = symbol.metadata().children();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }

    None
}

/// Helper to check if a symbol has a specific attribute by name
fn get_has_attribute(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>, attr_name: &str) -> bool {
    use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;

    symbol
        .metadata()
        .get_behavior::<AttributesBehavior>()
        .map(|ab| ab.attributes().iter().any(|a| a.name == attr_name))
        .unwrap_or(false)
}

/// Helper to get the number of attributes on a symbol
fn get_attribute_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
    use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;

    symbol
        .metadata()
        .get_behavior::<AttributesBehavior>()
        .map(|ab| ab.attributes().len())
        .unwrap_or(0)
}

/// Helper to get the number of arguments for a specific attribute
fn get_attribute_arg_count(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    attr_name: &str,
) -> Option<usize> {
    use kestrel_semantic_tree::behavior::attributes::AttributesBehavior;

    symbol
        .metadata()
        .get_behavior::<AttributesBehavior>()
        .and_then(|ab| {
            ab.attributes()
                .iter()
                .find(|a| a.name == attr_name)
                .map(|a| a.args.len())
        })
}

/// Helper to check if a symbol has a negative conformance to a specific protocol by name
fn has_negative_conformance_to(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    protocol_name: &str,
) -> bool {
    use kestrel_semantic_tree::ty::TyKind;

    symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| {
            cb.negative_conformances().iter().any(|ty| {
                if let TyKind::Protocol { symbol, .. } = ty.kind() {
                    symbol.metadata().name().value == protocol_name
                } else {
                    false
                }
            })
        })
        .unwrap_or(false)
}

/// Helper to check if a symbol conforms to a specific protocol by name
fn conforms_to_protocol(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    protocol_name: &str,
) -> bool {
    use kestrel_semantic_tree::ty::TyKind;

    symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| {
            cb.conformances().iter().any(|ty| {
                if let TyKind::Protocol { symbol, .. } = ty.kind() {
                    symbol.metadata().name().value == protocol_name
                } else {
                    false
                }
            })
        })
        .unwrap_or(false)
}

/// Helper to check if a symbol is copyable (has CopySemanticsBehavior)
fn get_is_copyable(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    symbol
        .metadata()
        .get_behavior::<CopySemanticsBehavior>()
        .map(|csb| csb.is_copyable())
}

/// Helper to check if a symbol is cloneable (has CopySemanticsBehavior::Cloneable)
fn get_is_cloneable(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    symbol
        .metadata()
        .get_behavior::<CopySemanticsBehavior>()
        .map(|csb| csb.is_cloneable())
}

/// Helper to check if a struct has a deinit (has DeinitBehavior)
fn get_has_deinit(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> bool {
    use kestrel_semantic_tree::behavior::deinit::DeinitBehavior;

    symbol.metadata().get_behavior::<DeinitBehavior>().is_some()
}
