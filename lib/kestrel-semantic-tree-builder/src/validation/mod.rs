//! Validation pass infrastructure for semantic analysis
//!
//! This module provides a unified system for running validation passes
//! on the semantic tree after binding is complete. The architecture uses
//! a single tree walk that invokes all validators at each node type.
//!
//! ## Architecture
//!
//! Validators implement the `Validator` trait with methods for each node type:
//! - `validate_symbol` - Called for each symbol in the tree
//! - `validate_statement` - Called for each statement in function bodies
//! - `validate_expression` - Called for each expression in function bodies
//! - `validate_type` - Called for each type reference
//! - `validate_pattern` - Called for each pattern
//! - `finalize` - Called after the entire tree has been walked
//!
//! Validators only implement the methods they need - defaults are no-ops.
//! Some validators (like cycle detection) collect data during the walk
//! and perform analysis in `finalize`.

mod assignment_validation;
mod conformance;
mod constraint_cycles;
mod dead_code;
mod duplicate_symbol;
mod exhaustive_return;
mod extension_conflict;
mod function_body;
mod generics;
mod imports;
mod initializer_verification;
mod protocol_method;
mod static_context;
mod struct_cycles;
mod type_alias_cycles;
pub mod type_assignability;
mod type_check;
mod visibility_consistency;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::Symbol;

use kestrel_semantic_model::SemanticModel;

pub use assignment_validation::AssignmentValidator;
pub use conformance::ConformanceValidator;
pub use constraint_cycles::ConstraintCycleValidator;
pub use dead_code::DeadCodeValidator;
pub use duplicate_symbol::DuplicateSymbolValidator;
pub use exhaustive_return::ExhaustiveReturnValidator;
pub use extension_conflict::ExtensionConflictValidator;
pub use function_body::FunctionBodyValidator;
pub use generics::GenericsValidator;
pub use imports::ImportValidator;
pub use initializer_verification::InitializerVerificationValidator;
pub use protocol_method::ProtocolMethodValidator;
pub use static_context::StaticContextValidator;
pub use struct_cycles::StructCycleValidator;
pub use type_alias_cycles::TypeAliasCycleValidator;
pub use type_check::TypeCheckValidator;
pub use visibility_consistency::VisibilityConsistencyValidator;

// Keep old names as aliases for backwards compatibility
pub use AssignmentValidator as AssignmentValidationPass;
pub use ConformanceValidator as ConformancePass;
pub use ConstraintCycleValidator as ConstraintCyclePass;
pub use DuplicateSymbolValidator as DuplicateSymbolPass;
pub use FunctionBodyValidator as FunctionBodyPass;
pub use GenericsValidator as GenericsPass;
pub use ImportValidator as ImportValidationPass;
pub use InitializerVerificationValidator as InitializerVerificationPass;
pub use ProtocolMethodValidator as ProtocolMethodPass;
pub use StaticContextValidator as StaticContextPass;
pub use StructCycleValidator as StructCyclePass;
pub use TypeAliasCycleValidator as TypeAliasCyclePass;
pub use VisibilityConsistencyValidator as VisibilityConsistencyPass;

/// Configuration for which validation passes to run
#[derive(Default, Clone)]
pub struct ValidationConfig {
    /// If true, include pass name in error messages (for debugging)
    pub debug_mode: bool,
    /// Set of pass names that should be disabled
    disabled_passes: HashSet<&'static str>,
}

impl ValidationConfig {
    /// Create a new validation config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable a specific validation pass by name
    pub fn disable(&mut self, pass_name: &'static str) {
        self.disabled_passes.insert(pass_name);
    }

    /// Check if a pass is enabled
    pub fn is_enabled(&self, pass_name: &'static str) -> bool {
        !self.disabled_passes.contains(pass_name)
    }

    /// Enable debug mode (shows pass name in errors)
    pub fn with_debug_mode(mut self) -> Self {
        self.debug_mode = true;
        self
    }
}

/// Shared diagnostics context wrapper for use during tree walking
type SharedDiagnostics = Rc<RefCell<DiagnosticsWrapper>>;

/// Wrapper to hold the mutable reference to DiagnosticContext
///
/// This type provides safe access to the DiagnosticContext during validation tree walks.
pub struct DiagnosticsWrapper {
    inner: *mut DiagnosticContext,
}

impl DiagnosticsWrapper {
    /// Get a mutable reference to the underlying DiagnosticContext
    pub fn get(&self) -> &mut DiagnosticContext {
        // SAFETY: The wrapper is only created and used within run(), and the
        // DiagnosticContext reference is valid for the duration of the walk.
        unsafe { &mut *self.inner }
    }
}

/// Context passed to validators when validating symbols
pub struct SymbolContext<'a> {
    /// The symbol being validated
    pub symbol: &'a Arc<dyn Symbol<KestrelLanguage>>,
    /// Whether we're inside a protocol
    pub in_protocol: bool,
    /// Whether we're inside a struct
    pub in_struct: bool,
    /// Whether we're inside an extension
    pub in_extension: bool,
    /// The semantic model for queries
    pub model: &'a SemanticModel,
    /// Diagnostics context for reporting errors
    diagnostics: SharedDiagnostics,
    /// The file ID for the current symbol
    pub file_id: usize,
}

impl<'a> SymbolContext<'a> {
    /// Get a mutable reference to the diagnostics context
    pub fn diagnostics(&self) -> std::cell::RefMut<'_, DiagnosticsWrapper> {
        self.diagnostics.borrow_mut()
    }
}

/// Context passed to validators when validating body nodes (statements, expressions, patterns)
pub struct BodyContext<'a> {
    /// The containing symbol (function or initializer)
    pub container: &'a Arc<dyn Symbol<KestrelLanguage>>,
    /// The semantic model for queries
    pub model: &'a SemanticModel,
    /// Diagnostics context for reporting errors
    diagnostics: SharedDiagnostics,
    /// The file ID for the current symbol
    pub file_id: usize,
}

impl<'a> BodyContext<'a> {
    /// Get a mutable reference to the diagnostics context
    pub fn diagnostics(&self) -> std::cell::RefMut<'_, DiagnosticsWrapper> {
        self.diagnostics.borrow_mut()
    }
}

/// Context passed to validators when validating types
pub struct TypeContext<'a> {
    /// The containing symbol where this type appears
    pub container: &'a Arc<dyn Symbol<KestrelLanguage>>,
    /// The semantic model for queries
    pub model: &'a SemanticModel,
    /// Diagnostics context for reporting errors
    diagnostics: SharedDiagnostics,
    /// The file ID for the current symbol
    pub file_id: usize,
}

impl<'a> TypeContext<'a> {
    /// Get a mutable reference to the diagnostics context
    pub fn diagnostics(&self) -> std::cell::RefMut<'_, DiagnosticsWrapper> {
        self.diagnostics.borrow_mut()
    }
}

/// Unified trait for all validation passes
///
/// Validators implement only the methods they need - all have default no-op implementations.
/// The tree walker calls each method at the appropriate point during traversal.
pub trait Validator: Send + Sync {
    /// Unique identifier for this validator
    fn name(&self) -> &'static str;

    /// Validate a symbol in the semantic tree
    ///
    /// Called for each symbol during tree traversal. Validators that need to
    /// analyze symbol-level properties implement this method.
    fn validate_symbol(&self, _ctx: &SymbolContext<'_>) {}

    /// Validate a statement in a function/initializer body
    ///
    /// Called for each statement during body traversal.
    fn validate_statement(&self, _stmt: &Statement, _ctx: &BodyContext<'_>) {}

    /// Validate an expression in a function/initializer body
    ///
    /// Called for each expression during body traversal.
    fn validate_expression(&self, _expr: &Expression, _ctx: &BodyContext<'_>) {}

    /// Validate a type reference
    ///
    /// Called for type references in signatures, fields, etc.
    fn validate_type(&self, _ty: &Ty, _ctx: &TypeContext<'_>) {}

    /// Validate a pattern
    ///
    /// Called for patterns in variable bindings.
    fn validate_pattern(&self, _pattern: &Pattern, _ctx: &BodyContext<'_>) {}

    /// Called after the entire tree has been walked
    ///
    /// Validators that collect data during traversal and analyze it at the end
    /// implement this method (e.g., cycle detection).
    fn finalize(&self, _model: &SemanticModel, _diagnostics: &mut DiagnosticContext) {}
}

/// Legacy trait for backwards compatibility
///
/// New code should use the `Validator` trait instead.
pub trait ValidationPass: Send + Sync {
    /// Unique identifier for this pass
    fn name(&self) -> &'static str;

    /// Run the validation pass on the semantic tree
    fn validate(
        &self,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
        model: &SemanticModel,
        diagnostics: &mut DiagnosticContext,
        config: &ValidationConfig,
    );
}

/// Registry and runner for all validation passes
pub struct ValidationRunner {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidationRunner {
    /// Create a new validation runner with all registered validators
    pub fn new() -> Self {
        let validators: Vec<Box<dyn Validator>> = vec![
            Box::new(FunctionBodyValidator::new()),
            Box::new(ProtocolMethodValidator::new()),
            Box::new(StaticContextValidator::new()),
            Box::new(DuplicateSymbolValidator::new()),
            Box::new(VisibilityConsistencyValidator::new()),
            Box::new(GenericsValidator::new()),
            Box::new(TypeAliasCycleValidator::new()),
            Box::new(StructCycleValidator::new()),
            Box::new(ConstraintCycleValidator::new()),
            Box::new(ImportValidator::new()),
            Box::new(ConformanceValidator::new()),
            Box::new(ExtensionConflictValidator::new()),
            Box::new(InitializerVerificationValidator::new()),
            Box::new(AssignmentValidator::new()),
            Box::new(DeadCodeValidator::new()),
            Box::new(ExhaustiveReturnValidator::new()),
            Box::new(TypeCheckValidator::new()),
        ];

        Self { validators }
    }

    /// Run all enabled validation passes using a single tree walk
    pub fn run(
        &self,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
        model: &SemanticModel,
        diagnostics: &mut DiagnosticContext,
        config: &ValidationConfig,
    ) {
        // Filter to only enabled validators
        let enabled: Vec<&dyn Validator> = self
            .validators
            .iter()
            .filter(|v| config.is_enabled(v.name()))
            .map(|v| v.as_ref())
            .collect();

        // Wrap diagnostics in shared container for the walk
        let shared_diagnostics = Rc::new(RefCell::new(DiagnosticsWrapper {
            inner: diagnostics as *mut DiagnosticContext,
        }));

        // Single tree walk calling all validators
        walk_symbol(root, &enabled, model, &shared_diagnostics, false, false, false);

        // Finalize all validators (diagnostics is still valid here)
        for validator in &enabled {
            validator.finalize(model, diagnostics);
        }
    }
}

impl Default for ValidationRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Walk a symbol and all its descendants, calling validators at each node
fn walk_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    validators: &[&dyn Validator],
    model: &SemanticModel,
    diagnostics: &SharedDiagnostics,
    in_protocol: bool,
    in_struct: bool,
    in_extension: bool,
) {
    let kind = symbol.metadata().kind();
    let file_id = crate::syntax::get_file_id_for_symbol(symbol, diagnostics.borrow_mut().get());

    // Update context flags
    let in_protocol = in_protocol || kind == KestrelSymbolKind::Protocol;
    let in_struct = in_struct || kind == KestrelSymbolKind::Struct;
    let in_extension = in_extension || kind == KestrelSymbolKind::Extension;

    // Create symbol context
    let ctx = SymbolContext {
        symbol,
        in_protocol,
        in_struct,
        in_extension,
        model,
        diagnostics: Rc::clone(diagnostics),
        file_id,
    };

    // Call all validators for this symbol
    for validator in validators {
        validator.validate_symbol(&ctx);
    }

    // If this symbol has a body (function or initializer), walk the body
    if matches!(
        kind,
        KestrelSymbolKind::Function | KestrelSymbolKind::Initializer
    ) {
        if let Some(body) = get_executable_body(symbol) {
            let body_ctx = BodyContext {
                container: symbol,
                model,
                diagnostics: Rc::clone(diagnostics),
                file_id,
            };

            // Walk statements
            for stmt in &body.statements {
                walk_statement(stmt, validators, &body_ctx);
            }

            // Walk yield expression if present
            if let Some(yield_expr) = body.yield_expr() {
                walk_expression(yield_expr, validators, &body_ctx);
            }
        }
    }

    // Recursively walk children
    for child in symbol.metadata().children() {
        walk_symbol(&child, validators, model, diagnostics, in_protocol, in_struct, in_extension);
    }
}

/// Walk a statement, calling validators
fn walk_statement(stmt: &Statement, validators: &[&dyn Validator], ctx: &BodyContext<'_>) {
    // Call all validators for this statement
    for validator in validators {
        validator.validate_statement(stmt, ctx);
    }

    // Walk nested nodes
    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            walk_pattern(pattern, validators, ctx);
            if let Some(value) = value {
                walk_expression(value, validators, ctx);
            }
        }
        StatementKind::Expr(expr) => {
            walk_expression(expr, validators, ctx);
        }
    }
}

/// Walk an expression, calling validators
fn walk_expression(expr: &Expression, validators: &[&dyn Validator], ctx: &BodyContext<'_>) {
    // Call all validators for this expression
    for validator in validators {
        validator.validate_expression(expr, ctx);
    }

    // Walk nested expressions
    match &expr.kind {
        ExprKind::Array(elements) => {
            for elem in elements {
                walk_expression(elem, validators, ctx);
            }
        }
        ExprKind::Tuple(elements) => {
            for elem in elements {
                walk_expression(elem, validators, ctx);
            }
        }
        ExprKind::Grouping(inner) => {
            walk_expression(inner, validators, ctx);
        }
        ExprKind::FieldAccess { object, .. } => {
            walk_expression(object, validators, ctx);
        }
        ExprKind::TupleIndex { tuple, .. } => {
            walk_expression(tuple, validators, ctx);
        }
        ExprKind::MethodRef { receiver, .. } => {
            walk_expression(receiver, validators, ctx);
        }
        ExprKind::Call { callee, arguments, .. } => {
            walk_expression(callee, validators, ctx);
            for arg in arguments {
                walk_expression(&arg.value, validators, ctx);
            }
        }
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            walk_expression(receiver, validators, ctx);
            for arg in arguments {
                walk_expression(&arg.value, validators, ctx);
            }
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                walk_expression(&arg.value, validators, ctx);
            }
        }
        ExprKind::Assignment { target, value } => {
            walk_expression(target, validators, ctx);
            walk_expression(value, validators, ctx);
        }
        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            walk_expression(condition, validators, ctx);
            for stmt in then_branch {
                walk_statement(stmt, validators, ctx);
            }
            if let Some(value) = then_value {
                walk_expression(value, validators, ctx);
            }
            if let Some(else_branch) = else_branch {
                match else_branch {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            walk_statement(stmt, validators, ctx);
                        }
                        if let Some(value) = value {
                            walk_expression(value, validators, ctx);
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(if_expr) => {
                        walk_expression(if_expr, validators, ctx);
                    }
                }
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            walk_expression(condition, validators, ctx);
            for stmt in body {
                walk_statement(stmt, validators, ctx);
            }
        }
        ExprKind::Loop { body, .. } => {
            for stmt in body {
                walk_statement(stmt, validators, ctx);
            }
        }
        // Leaf expressions - no nested nodes
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Return { value: None }
        | ExprKind::Error => {}

        ExprKind::Return { value: Some(val) } => {
            walk_expression(val, validators, ctx);
        }
    }
}

/// Walk a pattern, calling validators
fn walk_pattern(pattern: &Pattern, validators: &[&dyn Validator], ctx: &BodyContext<'_>) {
    // Call all validators for this pattern
    for validator in validators {
        validator.validate_pattern(pattern, ctx);
    }

    // Future: walk nested patterns for tuple/struct patterns
}

/// Get the executable body from a symbol (function or initializer)
fn get_executable_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<kestrel_semantic_tree::behavior::executable::CodeBlock> {
    let behaviors = symbol.metadata().behaviors();
    for b in behaviors.iter() {
        if matches!(b.kind(), KestrelBehaviorKind::Executable) {
            if let Some(exec) = b.as_ref().downcast_ref::<ExecutableBehavior>() {
                return Some(exec.body().clone());
            }
        }
    }
    None
}
