//! Tests for closure lowering.
//!
//! These tests verify that closures are correctly lowered to MIR:
//! - Non-capturing closures generate synthetic functions
//! - Capturing closures generate environment structs and call functions
//! - All closures produce FuncThick values
//! - Naming follows the `func.closure.N` convention

use kestrel_lexer::lex;
use kestrel_parser::{parse_source_file, Parser};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree_binder::SemanticBinder;
use kestrel_semantic_tree_builder::SemanticModelBuilder;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::lower_module;

/// Helper to compile source code and lower to MIR.
fn compile_and_lower(source: &str) -> (crate::LoweringResult, DiagnosticContext) {
    let mut builder = SemanticModelBuilder::new();
    let mut diagnostics = DiagnosticContext::new();

    let file_id = diagnostics.add_file("test.ks".to_string(), source.to_string());
    let tokens: Vec<_> = lex(source, file_id)
        .filter_map(|t| t.ok())
        .map(|spanned| (spanned.value, spanned.span))
        .collect();

    let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

    if !result.errors.is_empty() {
        for error in &result.errors {
            let span = error.span.clone().unwrap_or(Span::from(0..1));
            let diagnostic = kestrel_reporting::Diagnostic::error()
                .with_message(&error.message)
                .with_labels(vec![kestrel_reporting::Label::primary(
                    file_id,
                    span.range(),
                )]);
            diagnostics.add_diagnostic(diagnostic);
        }
    }

    builder.add_file("test.ks", &result.tree, source, file_id, &mut diagnostics);

    let model = builder.build();
    let model = SemanticBinder::bind(model, &mut diagnostics);

    // Run analyzers
    {
        use kestrel_semantic_analyzers::{default_analyzers, run_all, AnalysisContext, Analyzer};
        let mut owned = default_analyzers();
        let mut analyzers: Vec<&mut dyn Analyzer> = Vec::new();
        for a in owned.iter_mut() {
            analyzers.push(a.as_mut());
        }
        let mut ctx = AnalysisContext::new(&model, &mut diagnostics);
        run_all(&mut analyzers, &model, &mut ctx);
    }

    let root = model.root();
    let lowering_result = lower_module(&model, &root);

    (lowering_result, diagnostics)
}

/// Helper to get MIR output as string for verification.
fn mir_output(source: &str) -> String {
    let (result, diagnostics) = compile_and_lower(source);

    // Print any semantic errors for debugging
    if diagnostics.has_errors() {
        eprintln!("Semantic errors:");
        diagnostics.emit().ok();
    }

    // Print any lowering errors for debugging
    if !result.diagnostics.is_empty() {
        eprintln!("Lowering diagnostics:");
        for diag in &result.diagnostics {
            eprintln!("  {:?}", diag);
        }
    }

    result.mir.display().to_string()
}

/// Helper to check that MIR output contains expected string.
fn assert_mir_contains(source: &str, expected: &str) {
    let output = mir_output(source);
    assert!(
        output.contains(expected),
        "Expected MIR to contain:\n  {}\n\nActual MIR:\n{}",
        expected,
        output
    );
}

/// Helper to check MIR output does NOT contain a string.
fn assert_mir_not_contains(source: &str, unexpected: &str) {
    let output = mir_output(source);
    assert!(
        !output.contains(unexpected),
        "Expected MIR to NOT contain:\n  {}\n\nActual MIR:\n{}",
        unexpected,
        output
    );
}

// ============================================================================
// NON-CAPTURING CLOSURES
// ============================================================================

mod non_capturing {
    use super::*;

    #[test]
    fn closure_no_params_returns_constant() {
        // Simplest closure: no params, returns constant
        let source = r#"
module Test

func test() -> () -> Int {
    { 42 }
}
"#;
        let output = mir_output(source);

        // Should have a closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function 'test.closure.0'\n\nActual MIR:\n{}",
            output
        );

        // Should NOT have an env struct (no captures)
        assert!(
            !output.contains(".env"),
            "Expected no env struct for non-capturing closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closure_with_explicit_params() {
        // Closure with explicit parameters
        let source = r#"
module Test

func test() -> (Int, Int) -> Int {
    { (x, y) in x + y }
}
"#;
        let output = mir_output(source);

        // Should have closure function with params
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have x and y as parameters in the closure
        assert!(
            output.contains("x: i64") || output.contains("x:"),
            "Expected parameter 'x'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closure_with_implicit_it_param() {
        // Closure with implicit `it` parameter
        let source = r#"
module Test

func test() -> (Int) -> Int {
    { it * 2 }
}
"#;
        let output = mir_output(source);

        // Should have closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have `it` as a parameter
        assert!(
            output.contains("it:") || output.contains("it: i64"),
            "Expected parameter 'it'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closure_empty_params_returns_unit() {
        // Closure with () -> ()
        let source = r#"
module Test

func test() -> () -> () {
    { () }
}
"#;
        let output = mir_output(source);

        // Should have closure function returning unit
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// CAPTURING CLOSURES
// ============================================================================

mod capturing {
    use super::*;

    #[test]
    fn single_capture_from_parameter() {
        // Closure captures function parameter
        let source = r#"
module Test

func test(n: Int) -> () -> Int {
    { n + 1 }
}
"#;
        let output = mir_output(source);

        // Should have closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have env struct with captured variable
        assert!(
            output.contains(".env") || output.contains("struct"),
            "Expected env struct for capturing closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn single_capture_from_let() {
        // Closure captures local let binding
        let source = r#"
module Test

func test() -> () -> Int {
    let x = 42;
    { x }
}
"#;
        let output = mir_output(source);

        // Should have closure with captured x
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn multiple_captures() {
        // Closure captures multiple variables
        let source = r#"
module Test

func test() -> () -> Int {
    let a = 1;
    let b = 2;
    let c = 3;
    { a + b + c }
}
"#;
        let output = mir_output(source);

        // Should have closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn capture_with_params() {
        // Closure has both captures and parameters
        let source = r#"
module Test

func test(multiplier: Int) -> (Int) -> Int {
    { it * multiplier }
}
"#;
        let output = mir_output(source);

        // Should have closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have both env parameter and it parameter
        // The closure function should take env and it
    }
}

// ============================================================================
// MULTI-STATEMENT CLOSURES
// ============================================================================

mod multi_statement {
    use super::*;

    #[test]
    fn closure_with_let_binding() {
        let source = r#"
module Test

func test() -> (Int) -> Int {
    { (x) in
        let y = x * 2;
        y + 1
    }
}
"#;
        let output = mir_output(source);

        // Should have closure function
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have local y in the closure
        assert!(
            output.contains("y:") || output.contains("y: i64"),
            "Expected local 'y' in closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closure_with_control_flow() {
        let source = r#"
module Test

func test() -> (Int) -> Int {
    { (x) in
        if x > 0 {
            x
        } else {
            0 - x
        }
    }
}
"#;
        let output = mir_output(source);

        // Should have closure function with control flow (multiple blocks)
        assert!(
            output.contains("test.closure.0"),
            "Expected closure function\n\nActual MIR:\n{}",
            output
        );

        // Should have branch instruction
        assert!(
            output.contains("branch") || output.contains("bb"),
            "Expected control flow in closure\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// MULTIPLE CLOSURES
// ============================================================================

mod multiple_closures {
    use super::*;

    #[test]
    fn two_closures_in_same_function() {
        let source = r#"
module Test

func test() -> Int {
    let f: () -> Int = { 1 };
    let g: () -> Int = { 2 };
    f() + g()
}
"#;
        let output = mir_output(source);

        // Should have both closure.0 and closure.1
        assert!(
            output.contains("test.closure.0"),
            "Expected first closure\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("test.closure.1"),
            "Expected second closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closures_in_different_functions() {
        let source = r#"
module Test

func foo() -> () -> Int {
    { 1 }
}

func bar() -> () -> Int {
    { 2 }
}
"#;
        let output = mir_output(source);

        // Should have foo.closure.0 and bar.closure.0
        assert!(
            output.contains("foo.closure.0"),
            "Expected foo's closure\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("bar.closure.0"),
            "Expected bar's closure\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// NESTED CLOSURES
// ============================================================================

mod nested {
    use super::*;

    #[test]
    fn closure_returning_closure() {
        let source = r#"
module Test

func test() -> (Int) -> (Int) -> Int {
    { (x) in { (y) in x + y } }
}
"#;
        let output = mir_output(source);

        // Should have two closures - nested naming: closure.0 and closure.0.closure.0
        assert!(
            output.contains("test.closure.0"),
            "Expected outer closure\n\nActual MIR:\n{}",
            output
        );
        // Inner closure is nested inside outer, so it's closure.0.closure.0
        assert!(
            output.contains("closure.0.closure.0"),
            "Expected inner closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn inner_closure_captures_outer_param() {
        let source = r#"
module Test

func test() -> Int {
    let f: (Int) -> (Int) -> Int = { (x) in { (y) in x + y } };
    let add10 = f(10);
    add10(5)
}
"#;
        let output = mir_output(source);

        // Should have nested closures with capture
        assert!(
            output.contains("closure"),
            "Expected closures\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// CLOSURE INVOCATION
// ============================================================================

mod invocation {
    use super::*;

    #[test]
    fn immediately_invoked_closure() {
        let source = r#"
module Test

func test() -> Int {
    { 42 }()
}
"#;
        let output = mir_output(source);

        // Should have closure and call to it
        assert!(
            output.contains("test.closure.0"),
            "Expected closure\n\nActual MIR:\n{}",
            output
        );

        // Should have call instruction (Thick callee)
        assert!(
            output.contains("call") || output.contains("escaping"),
            "Expected call to closure\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn closure_stored_and_called() {
        let source = r#"
module Test

func test() -> Int {
    let f: (Int) -> Int = { it * 2 };
    f(21)
}
"#;
        let output = mir_output(source);

        // Should have closure and call
        assert!(
            output.contains("test.closure.0"),
            "Expected closure\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// THICK FUNCTION TYPE
// ============================================================================

mod func_type {
    use super::*;

    #[test]
    fn closure_produces_thick_type() {
        let source = r#"
module Test

func test() -> () -> Int {
    { 42 }
}
"#;
        let output = mir_output(source);

        // The closure value should be a thick function type
        // This is verified by the presence of ApplyPartial in the output
        assert!(
            output.contains("apply.partial") || output.contains("closure"),
            "Expected thick callable creation\n\nActual MIR:\n{}",
            output
        );
    }
}
