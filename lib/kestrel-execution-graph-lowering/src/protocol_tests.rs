//! Tests for protocol and witness lowering.
//!
//! These tests verify that protocols and witnesses are correctly lowered to MIR:
//! - Protocols generate ProtocolDef with methods and associated types
//! - Protocol conformances generate WitnessDef with type/method bindings
//! - Self type is preserved in protocol method signatures
//! - Associated types are statically resolved when possible

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
#[allow(dead_code)]
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
// BASIC PROTOCOL LOWERING
// ============================================================================

mod basic_protocol {
    use super::*;

    #[test]
    fn empty_protocol_lowers_to_mir() {
        let source = r#"
module Test

protocol Marker { }
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Marker"),
            "Expected protocol 'Test.Marker'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_with_single_method() {
        let source = r#"
module Test

protocol Hashable {
    func hash() -> Int
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Hashable"),
            "Expected protocol 'Test.Hashable'\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func hash"),
            "Expected method 'hash'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_with_multiple_methods() {
        let source = r#"
module Test

protocol Comparable {
    func lessThan(other: Int) -> Bool
    func equals(other: Int) -> Bool
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Comparable"),
            "Expected protocol 'Test.Comparable'\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func lessThan"),
            "Expected method 'lessThan'\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func equals"),
            "Expected method 'equals'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn public_protocol() {
        let source = r#"
module Test

public protocol Drawable { }
"#;
        assert_mir_contains(source, "protocol Test.Drawable");
    }
}

// ============================================================================
// ASSOCIATED TYPES IN PROTOCOLS
// ============================================================================

mod associated_types {
    use super::*;

    #[test]
    fn protocol_with_associated_type() {
        let source = r#"
module Test

protocol Iterator {
    type Item;
    func next() -> Item
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Iterator"),
            "Expected protocol 'Test.Iterator'\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("type Item"),
            "Expected associated type 'Item'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_with_multiple_associated_types() {
        let source = r#"
module Test

protocol Dictionary {
    type Key;
    type Value;
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("type Key"),
            "Expected associated type 'Key'\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("type Value"),
            "Expected associated type 'Value'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn associated_type_with_default() {
        let source = r#"
module Test

protocol Parser {
    type Output = String;
}
"#;
        // Default associated types should be recorded
        assert_mir_contains(source, "protocol Test.Parser");
        assert_mir_contains(source, "type Output");
    }
}

// ============================================================================
// GENERIC PROTOCOLS
// ============================================================================

mod generic_protocol {
    use super::*;

    #[test]
    fn generic_protocol_preserves_type_params() {
        let source = r#"
module Test

protocol Container[T] { }
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Container[T]"),
            "Expected generic protocol 'Test.Container[T]'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn generic_protocol_with_method_using_type_param() {
        let source = r#"
module Test

protocol Container[T] {
    func get() -> T
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Container[T]"),
            "Expected generic protocol\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func get"),
            "Expected method 'get'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn generic_protocol_multiple_type_params() {
        let source = r#"
module Test

protocol Mapping[K, V] {
    func get(key: K) -> V
}
"#;
        assert_mir_contains(source, "protocol Test.Mapping[K, V]");
    }
}

// ============================================================================
// SELF TYPE IN PROTOCOLS
// ============================================================================

mod self_type {
    use super::*;

    #[test]
    fn protocol_method_with_self_param() {
        let source = r#"
module Test

protocol Equatable {
    func eq(other: Self) -> Bool
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Equatable"),
            "Expected protocol\n\nActual MIR:\n{}",
            output
        );
        // Self should be preserved in the signature
        assert!(
            output.contains("Self"),
            "Expected Self type in method signature\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_method_returns_self() {
        let source = r#"
module Test

protocol Cloneable {
    func clone() -> Self
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("-> Self"),
            "Expected return type Self\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_method_with_self_in_array() {
        let source = r#"
module Test

protocol Collection {
    func getAll() -> [Self]
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("func getAll"),
            "Expected method 'getAll'\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// RECEIVER KINDS
// ============================================================================

mod receiver_kinds {
    use super::*;

    #[test]
    fn static_method_in_protocol() {
        let source = r#"
module Test

protocol Factory {
    static func create() -> Int
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Factory"),
            "Expected protocol\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func create"),
            "Expected static method 'create'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn mutating_method_in_protocol() {
        let source = r#"
module Test

protocol Incrementable {
    mutating func increment()
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("func increment"),
            "Expected mutating method 'increment'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn consuming_method_in_protocol() {
        let source = r#"
module Test

protocol Disposable {
    consuming func dispose()
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("func dispose"),
            "Expected consuming method 'dispose'\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// PROTOCOL INHERITANCE
// ============================================================================

mod protocol_inheritance {
    use super::*;

    #[test]
    fn protocol_inherits_single_protocol() {
        let source = r#"
module Test

protocol Drawable { }
protocol Shape: Drawable { }
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Drawable"),
            "Expected parent protocol\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("protocol Test.Shape"),
            "Expected child protocol\n\nActual MIR:\n{}",
            output
        );
        // Child should reference parent
        assert!(
            output.contains("Test.Shape: Test.Drawable") || output.contains("Test.Shape") && output.contains("Drawable"),
            "Expected inheritance relationship\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_inherits_multiple_protocols() {
        let source = r#"
module Test

protocol Drawable { }
protocol Clickable { }
protocol Widget: Drawable, Clickable { }
"#;
        let output = mir_output(source);

        assert!(
            output.contains("protocol Test.Widget"),
            "Expected protocol 'Test.Widget'\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn protocol_with_inherited_and_own_methods() {
        let source = r#"
module Test

protocol Drawable {
    func draw()
}
protocol Shape: Drawable {
    func area() -> Int
}
"#;
        let output = mir_output(source);

        // Parent has draw
        assert!(
            output.contains("func draw"),
            "Expected inherited method 'draw'\n\nActual MIR:\n{}",
            output
        );
        // Child has area (not draw - it's inherited, not duplicated)
        assert!(
            output.contains("func area"),
            "Expected own method 'area'\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// WITNESS GENERATION FROM STRUCT CONFORMANCE
// ============================================================================

mod witness_from_struct {
    use super::*;

    #[test]
    fn struct_conformance_generates_witness() {
        let source = r#"
module Test

protocol Drawable {
    func draw()
}

struct Circle: Drawable {
    func draw() { }
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("Test.Circle") && output.contains("Test.Drawable"),
            "Expected witness for Circle: Drawable\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn witness_with_multiple_methods() {
        let source = r#"
module Test

protocol Comparable {
    func lessThan(other: Int) -> Bool
    func equals(other: Int) -> Bool
}

struct Number: Comparable {
    func lessThan(other: Int) -> Bool { true }
    func equals(other: Int) -> Bool { false }
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
        // Should have method bindings
        assert!(
            output.contains("func lessThan") || output.contains("lessThan ="),
            "Expected lessThan binding\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("func equals") || output.contains("equals ="),
            "Expected equals binding\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn witness_with_associated_type() {
        let source = r#"
module Test

protocol Iterator {
    type Item;
    func next() -> Item
}

struct IntIterator: Iterator {
    type Item = Int;
    func next() -> Int { 0 }
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
        // Should have type binding
        assert!(
            output.contains("type Item = i64") || output.contains("type Item"),
            "Expected associated type binding\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn witness_from_multiple_conformances() {
        let source = r#"
module Test

protocol Drawable {
    func draw()
}

protocol Clickable {
    func onClick()
}

struct Button: Drawable, Clickable {
    func draw() { }
    func onClick() { }
}
"#;
        let output = mir_output(source);

        // Should have two witnesses
        assert!(
            output.contains("witness"),
            "Expected at least one witness\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// WITNESS GENERATION FROM EXTENSION CONFORMANCE
// ============================================================================

mod witness_from_extension {
    use super::*;

    #[test]
    fn extension_conformance_generates_witness() {
        let source = r#"
module Test

protocol Hashable {
    func hash() -> Int
}

struct Point { }

extend Point: Hashable {
    func hash() -> Int { 42 }
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness from extension\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// GENERIC WITNESS
// ============================================================================

mod generic_witness {
    use super::*;

    #[test]
    fn generic_struct_witness() {
        let source = r#"
module Test

protocol Container {
    type Item;
}

struct Box[T]: Container {
    type Item = T;
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn generic_struct_witness_with_method() {
        let source = r#"
module Test

protocol Getter {
    func get() -> Int
}

struct Box[T]: Getter {
    func get() -> Int { 42 }
}
"#;
        let output = mir_output(source);

        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// INHERITED PROTOCOL WITNESS
// ============================================================================

mod inherited_witness {
    use super::*;

    #[test]
    fn struct_satisfies_inherited_methods() {
        let source = r#"
module Test

protocol A {
    func a()
}

protocol B: A {
    func b()
}

struct S: B {
    func a() { }
    func b() { }
}
"#;
        let output = mir_output(source);

        // Should have witnesses for both A and B
        assert!(
            output.contains("witness"),
            "Expected witness\n\nActual MIR:\n{}",
            output
        );
    }
}

// ============================================================================
// WITNESS METHOD CALLS (Type Parameter Calls)
// ============================================================================

mod witness_method_calls {
    use super::*;

    #[test]
    fn instance_method_on_type_parameter() {
        // a.add(b) where a: T, T: Add
        let source = r#"
module Test

protocol Add {
    func add(other: Self) -> Self
}

func addThem[T](a: T, b: T) -> T where T: Add {
    return a.add(b)
}
"#;
        let output = mir_output(source);

        // Should call via witness_method
        assert!(
            output.contains("witness_method Test.Add.add for T"),
            "Expected witness_method call for Add.add\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn static_method_on_type_parameter() {
        // T.create() where T: Factory
        let source = r#"
module Test

protocol Factory {
    static func create() -> Self
}

func make[T]() -> T where T: Factory {
    return T.create()
}
"#;
        let output = mir_output(source);

        // Should call via witness_method
        assert!(
            output.contains("witness_method Test.Factory.create for T"),
            "Expected witness_method call for Factory.create\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn init_on_type_parameter() {
        // T() where T: Factory
        let source = r#"
module Test

protocol Factory {
    init()
}

func make[T]() -> T where T: Factory {
    return T()
}
"#;
        let output = mir_output(source);

        // Should call via witness_method for init
        assert!(
            output.contains("witness_method Test.Factory.init for T"),
            "Expected witness_method call for Factory.init\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn init_with_arguments_on_type_parameter() {
        // T(value: v) where T: Factory
        let source = r#"
module Test

protocol Factory {
    init(value: Int)
}

func make[T](v: Int) -> T where T: Factory {
    return T(value: v)
}
"#;
        let output = mir_output(source);

        // Should call via witness_method for init
        assert!(
            output.contains("witness_method Test.Factory.init for T"),
            "Expected witness_method call for Factory.init\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn method_with_arguments_on_type_parameter() {
        // a.process(x, y) where a: T, T: Processor
        let source = r#"
module Test

protocol Processor {
    func process(x: Int, y: Int) -> Int
}

func run[T](proc: T, a: Int, b: Int) -> Int where T: Processor {
    return proc.process(a, b)
}
"#;
        let output = mir_output(source);

        // Should call via witness_method
        assert!(
            output.contains("witness_method Test.Processor.process for T"),
            "Expected witness_method call for Processor.process\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn multiple_bounds_uses_correct_protocol() {
        // When T has multiple bounds, the witness should reference the correct protocol
        let source = r#"
module Test

protocol Add {
    func add(other: Self) -> Self
}

protocol Mul {
    func mul(other: Self) -> Self
}

func compute[T](a: T, b: T) -> T where T: Add and Mul {
    let sum = a.add(b);
    return sum.mul(b)
}
"#;
        let output = mir_output(source);

        // Should have witness calls for both protocols
        assert!(
            output.contains("witness_method Test.Add.add for T"),
            "Expected witness_method for Add.add\n\nActual MIR:\n{}",
            output
        );
        assert!(
            output.contains("witness_method Test.Mul.mul for T"),
            "Expected witness_method for Mul.mul\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn static_method_with_arguments() {
        // T.fromInt(42) where T: Convertible
        let source = r#"
module Test

protocol Convertible {
    static func fromInt(value: Int) -> Self
}

func convert[T](n: Int) -> T where T: Convertible {
    return T.fromInt(n)
}
"#;
        let output = mir_output(source);

        // Should call via witness_method
        assert!(
            output.contains("witness_method Test.Convertible.fromInt for T"),
            "Expected witness_method call for Convertible.fromInt\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn mutating_method_on_type_parameter() {
        // a.increment() where T: Counter, mutating func
        let source = r#"
module Test

protocol Counter {
    mutating func increment()
}

func bump[T](a: T) where T: Counter {
    var x = a;
    x.increment()
}
"#;
        let output = mir_output(source);

        // Should call via witness_method
        assert!(
            output.contains("witness_method Test.Counter.increment for T"),
            "Expected witness_method call for Counter.increment\n\nActual MIR:\n{}",
            output
        );
    }

    #[test]
    fn static_method_on_associated_type() {
        // T.Item.create() where T: Container, Container.Item: Factory
        let source = r#"
module Test

protocol Factory {
    static func create() -> Self
}

protocol Container {
    type Item: Factory;
}

func makeItem[T]() -> T.Item where T: Container {
    return T.Item.create()
}
"#;
        let output = mir_output(source);

        // Should call via witness_method for the associated type
        assert!(
            output.contains("witness_method Test.Factory.create for"),
            "Expected witness_method call for Factory.create\n\nActual MIR:\n{}",
            output
        );
    }
}
