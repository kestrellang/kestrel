//! Kestrel Test Suite
//!
//! A fluent test API for testing the Kestrel compiler.
//!
//! # Example
//!
//! ```
//! use kestrel_test_suite::*;
//!
//! #[test]
//! fn test_struct() {
//!     Test::new("module Test\nstruct Foo {}")
//!         .expect(Compiles)
//!         .expect(Symbol::new("Foo").is(SymbolKind::Struct));
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

use std::sync::Arc;

use kestrel_lexer::lex;
use kestrel_span::Span;
use kestrel_parser::{parse_source_file, Parser};
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::callable::ReceiverKind;
use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::visibility::Visibility as SemanticVisibility;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree_builder::SemanticModel;
use semantic_tree::symbol::Symbol as SymbolTrait;

// Re-export commonly used types
pub use kestrel_semantic_tree::behavior::callable::ReceiverKind as Receiver;
pub use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind as SymbolKind;

/// Visibility levels for test expectations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Fileprivate,
    Internal,
    Public,
}

/// Test context containing compilation results
pub struct TestContext {
    pub semantic_model: SemanticModel,
    pub diagnostics: DiagnosticContext,
    pub has_errors: bool,
}

/// A test case that can be run against the Kestrel compiler
pub struct Test {
    files: Vec<(String, String)>,
    context: Option<TestContext>,
}

impl Test {
    /// Create a new test from a single source string
    pub fn new(source: &str) -> Self {
        Test {
            files: vec![("test.ks".to_string(), source.to_string())],
            context: None,
        }
    }

    /// Create a test from multiple source files
    pub fn with_files(files: &[(&str, &str)]) -> Self {
        Test {
            files: files
                .iter()
                .map(|(name, content)| (name.to_string(), content.to_string()))
                .collect(),
            context: None,
        }
    }

    /// Compile the test files and store the result
    fn compile(&mut self) {
        use kestrel_semantic_tree_builder::{SemanticTreeBuilder, SemanticBinder};

        if self.context.is_some() {
            return; // Already compiled
        }

        let mut builder = SemanticTreeBuilder::new();
        let mut diagnostics = DiagnosticContext::new();
        let mut has_parse_errors = false;

        // Parse and add all files
        for (file_name, content) in &self.files {
            let file_id = diagnostics.add_file(file_name.clone(), content.clone());
            let tokens: Vec<_> = lex(content, file_id)
                .filter_map(|t| t.ok())
                .map(|spanned| (spanned.value, spanned.span))
                .collect();

            let result = Parser::parse(content, tokens.into_iter(), parse_source_file);

            if !result.errors.is_empty() {
                has_parse_errors = true;
                // Add parse errors to diagnostics
                for error in &result.errors {
                    let span = error.span.clone().unwrap_or(Span::from(0..1));
                    let diagnostic = kestrel_reporting::Diagnostic::error()
                        .with_message(&error.message)
                        .with_labels(vec![kestrel_reporting::Label::primary(file_id, span.range())]);
                    diagnostics.add_diagnostic(diagnostic);
                }
            }

            builder.add_file(file_name, &result.tree, content, &mut diagnostics, file_id);
        }

        // Build the semantic tree
        let tree = builder.build();

        // Run binding phase
        let model = SemanticBinder::bind(tree, &mut diagnostics);

        // Run analyzers (during migration we mirror builder validations here)
        {
            use kestrel_semantic_analyzers::{Analyzer, AnalysisContext, run_all, default_analyzers};
            let mut owned = default_analyzers();
            let mut analyzers: Vec<&mut dyn Analyzer> = Vec::new();
            for a in owned.iter_mut() { analyzers.push(a.as_mut()); }
            let mut ctx = AnalysisContext::new(&model, &mut diagnostics);
            run_all(&mut analyzers, &model, &mut ctx);
        }

        let has_errors = has_parse_errors || diagnostics.has_errors();

        self.context = Some(TestContext {
            semantic_model: model,
            diagnostics,
            has_errors,
        });
    }

    /// Apply an expectation to this test
    pub fn expect<E: Expectable>(mut self, expectation: E) -> Self {
        self.compile();
        let ctx = self.context.as_ref().unwrap();
        if let Err(e) = expectation.check(ctx) {
            // Emit diagnostics for context
            if ctx.diagnostics.len() > 0 {
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
            Err(format!(
                "Expected {} error(s), but got {}",
                self.0, actual
            ))
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
            .any(|diag| {
                diag.severity == Severity::Warning && diag.message.contains(self.0)
            });

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

    /// Find a symbol by simple name anywhere in the tree (depth-first)
    fn find_by_name(
        &self,
        symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        // Check if this symbol matches
        if symbol.metadata().name().value == name {
            return Some(symbol.clone());
        }

        // Search children
        for child in symbol.metadata().children() {
            if let Some(found) = self.find_by_name(&child, name) {
                return Some(found);
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

    /// Find a child symbol by name (searches only within the given parent)
    fn find_child_by_name(
        &self,
        parent: &Arc<dyn SymbolTrait<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
        for child in parent.metadata().children() {
            if child.metadata().name().value == name {
                return Some(child);
            }
            // Also search nested children (for cases like methods inside structs)
            if let Some(found) = self.find_child_by_name(&child, name) {
                return Some(found);
            }
        }
        None
    }
}

impl Expectable for Symbol {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        let root = ctx.semantic_model.root();
        let symbol = self
            .find_symbol(root)
            .ok_or_else(|| format!("Symbol '{}' not found", self.path))?;

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
                match symbol.visibility_behavior() {
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
        }
    }
}

/// Helper to get type parameter count for a symbol
fn get_type_param_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
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
    symbol.callable_behavior().map(|cb| cb.arity())
}

/// Helper to get conformance count
fn get_conformance_count(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> usize {
    symbol
        .conformances_behavior()
        .map(|cb| cb.conformances().len())
        .unwrap_or(0)
}

/// Helper to check if a function is an instance method
fn get_is_instance_method(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<bool> {
    symbol.callable_behavior().map(|cb| cb.is_instance_method())
}

/// Helper to get the receiver kind for a function
fn get_receiver_kind(symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>) -> Option<ReceiverKind> {
    symbol.callable_behavior().and_then(|cb| cb.receiver())
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
    use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
    use kestrel_semantic_tree::behavior::KestrelBehaviorKind;

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

/// Helper to find a symbol by ID in the tree
fn find_symbol_by_id(
    symbol: &Arc<dyn SymbolTrait<KestrelLanguage>>,
    id: semantic_tree::symbol::SymbolId,
) -> Option<Arc<dyn SymbolTrait<KestrelLanguage>>> {
    if symbol.metadata().id() == id {
        return Some(symbol.clone());
    }

    for child in symbol.metadata().children() {
        if let Some(found) = find_symbol_by_id(&child, id) {
            return Some(found);
        }
    }

    None
}
