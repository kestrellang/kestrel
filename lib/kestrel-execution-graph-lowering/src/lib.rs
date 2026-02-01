//! Kestrel Execution Graph Lowering
//!
//! This crate transforms the semantic tree (typed AST) into the execution graph (MIR).
//! It handles the conversion of high-level constructs like expressions, statements,
//! and control flow into the flat, explicit MIR representation.
//!
//! # Example
//!
//! ```ignore
//! use kestrel_execution_graph_lowering::lower_module;
//!
//! let result = lower_module(&model, &module_symbol);
//! if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
//!     // Handle errors
//! }
//! let mir = result.mir;
//! ```

mod bound_method;
mod closure;
mod context;
mod error;
mod expr;
mod lowerer;
mod match_lowering;
mod name;
mod pattern;
mod stmt;
mod thunk;
mod ty;

pub use context::LoweringContext;
pub use error::LoweringError;

use kestrel_execution_graph::MirContext;
use kestrel_reporting::Diagnostic;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

/// Result of lowering a module to MIR.
pub struct LoweringResult {
    /// The generated MIR context.
    pub mir: MirContext,
    /// Diagnostics (errors, warnings) produced during lowering.
    pub diagnostics: Vec<Diagnostic<usize>>,
}

impl LoweringResult {
    /// Check if any errors occurred during lowering.
    pub fn has_errors(&self) -> bool {
        use kestrel_reporting::Severity;
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error || d.severity == Severity::Bug)
    }
}

/// Lower a module and all its contents to MIR.
///
/// This is the main entry point for the lowering pass. It traverses the semantic tree
/// starting from the given module symbol and generates MIR for all items within.
///
/// # Arguments
///
/// * `model` - The semantic model providing context and queries
/// * `module` - The module symbol to lower (typically a SourceFile or Module)
///
/// # Returns
///
/// A `LoweringResult` containing the generated MIR and any diagnostics.
pub fn lower_module(
    model: &SemanticModel,
    module: &Arc<dyn Symbol<KestrelLanguage>>,
) -> LoweringResult {
    let mut ctx = LoweringContext::new(model);

    // Lower all children of the module
    for child in module.metadata().children() {
        lowerer::lower_item(&mut ctx, &child);
    }

    // Generate the __kestrel_init_statics function if there are any statics with initializers
    // and inject a call to it at the start of main()
    let init_func_name = generate_static_init_function(&mut ctx, module);
    if let Some(init_name) = init_func_name {
        inject_init_call_into_main(&mut ctx, init_name);
    }

    ctx.finish()
}

/// Generate the __kestrel_init_statics function that initializes all static variables.
///
/// This function is called at the start of main() to ensure all statics are initialized
/// before any user code runs.
///
/// Returns the qualified name of the init function if it was created, None if no statics need init.
fn generate_static_init_function(
    ctx: &mut LoweringContext,
    module: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<kestrel_execution_graph::Id<kestrel_execution_graph::id::QualifiedName>> {
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::symbol::field::FieldSymbol;

    // Collect all static fields with initializers
    let mut static_fields: Vec<Arc<dyn Symbol<KestrelLanguage>>> = Vec::new();
    collect_static_fields_with_initializers(module, &mut static_fields);

    if static_fields.is_empty() {
        return None;
    }

    // Create the __kestrel_init_statics function
    let init_func_name = ctx.mir.intern_name_parts(&["__kestrel_init_statics"]);
    let unit_ty = ctx.mir.ty_unit();

    let func_id = ctx.mir.add_function(init_func_name, unit_ty).id();

    // Enter function context and create entry block
    ctx.enter_function(func_id);
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // For each static field with an initializer, emit an assignment
    for field_sym in &static_fields {
        if let Some(field) = field_sym.as_ref().downcast_ref::<FieldSymbol>() {
            // Get the ExecutableBehavior containing the initializer
            if let Some(executable) = field_sym.metadata().get_behavior::<ExecutableBehavior>() {
                let body = executable.body();

                // Get the yield expression (the initializer value)
                if let Some(init_expr) = &body.yield_expr {
                    // Get the static's name
                    let static_name = name::qualified_name_for_symbol(ctx, field_sym);

                    // Lower the initializer expression
                    let init_value = expr::lower_expression(ctx, init_expr);

                    // Get or create the static in MIR
                    let field_ty = ty::lower_type(ctx, field.field_type());

                    // Check if static already exists, if not add it
                    let static_exists = ctx
                        .mir
                        .statics
                        .iter()
                        .any(|(_, def)| def.name == static_name);
                    if !static_exists {
                        ctx.mir.add_static(static_name, field_ty);
                    }

                    // Create a global place for the static
                    let global_place = kestrel_execution_graph::Place::global(static_name);

                    // Emit assignment: static = init_value
                    ctx.emit_assign_value(global_place, init_value);
                }
            }
        }
    }

    // Emit return and exit function context
    ctx.emit_return_unit();
    ctx.exit_function();

    Some(init_func_name)
}

/// Recursively collect all static fields with ExecutableBehavior (initializers).
fn collect_static_fields_with_initializers(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    result: &mut Vec<Arc<dyn Symbol<KestrelLanguage>>>,
) {
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    use kestrel_semantic_tree::symbol::field::FieldSymbol;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

    let kind = symbol.metadata().kind();

    // Check if this is a static field with an initializer
    if kind == KestrelSymbolKind::Field {
        if let Some(field) = symbol.as_ref().downcast_ref::<FieldSymbol>() {
            // Module-level fields are implicitly static even without the 'static' keyword
            let is_module_level = symbol
                .metadata()
                .parent()
                .map(|p| {
                    let pk = p.metadata().kind();
                    pk == KestrelSymbolKind::Module || pk == KestrelSymbolKind::SourceFile
                })
                .unwrap_or(false);

            let is_static_field = field.is_static() || is_module_level;

            if is_static_field
                && !field.is_computed()
                && symbol
                    .metadata()
                    .get_behavior::<ExecutableBehavior>()
                    .is_some()
            {
                result.push(symbol.clone());
            }
        }
    }

    // Recurse into children
    for child in symbol.metadata().children() {
        collect_static_fields_with_initializers(&child, result);
    }
}

/// Inject a call to __kestrel_init_statics at the start of main().
///
/// This finds the "main" function in MIR and inserts a call to the init function
/// as the first statement of its entry block.
fn inject_init_call_into_main(
    ctx: &mut LoweringContext,
    init_func_name: kestrel_execution_graph::Id<kestrel_execution_graph::id::QualifiedName>,
) {
    use kestrel_execution_graph::{Callee, StatementData};

    // Find main function by looking for a function named "main" or "Main.main"
    let main_func_id = ctx.mir.functions.iter().find_map(|(id, func_def)| {
        let name = ctx.mir.name(func_def.name);
        // Check if the name ends with "main"
        if name.segments.last().map(|s| s.as_str()) == Some("main") {
            Some(id)
        } else {
            None
        }
    });

    let Some(main_func_id) = main_func_id else {
        // No main function found, nothing to inject into
        return;
    };

    // Get the entry block of main
    let entry_block_id = match ctx.mir.functions[main_func_id].entry_block {
        Some(id) => id,
        None => return, // No entry block, can't inject
    };

    // Create a call statement to __kestrel_init_statics()
    let call_stmt = StatementData::call(Callee::direct(init_func_name), vec![]);
    let stmt_id = ctx.mir.statements.alloc(call_stmt);

    // Insert the call at the beginning of the entry block's statements
    ctx.mir.blocks[entry_block_id].statements.insert(0, stmt_id);
}
