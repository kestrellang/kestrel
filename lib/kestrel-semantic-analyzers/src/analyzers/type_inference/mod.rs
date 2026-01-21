//! Type inference analyzer.
//!
//! This analyzer runs type inference on function bodies and adds
//! `ResolvedExecutableBehavior` to functions with the resolved types.

use std::sync::Arc;

use kestrel_semantic_model::InferenceResultFor;
use kestrel_semantic_tree::behavior::executable::{ExecutableBehavior, ResolvedExecutableBehavior};
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::deinit::DeinitSymbol;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::local::LocalContainer;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty};
use kestrel_semantic_type_inference::{apply_solution, apply_solution_to_locals};
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;

use diagnostics::InferenceErrorDiagnostic;

/// Get the concrete self type for a symbol (struct, enum, extension).
///
/// The `self` local is created with `SelfType` which needs to be resolved
/// to the actual type. For generic types like `Optional[T]`, this constructs
/// the full type with type parameters (e.g., `Optional[T]` not just `Optional`).
fn get_concrete_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    let parent = symbol.metadata().parent()?;
    let span = parent.metadata().span().clone();

    match parent.metadata().kind() {
        KestrelSymbolKind::Extension => parent
            .metadata()
            .get_behavior::<ExtensionTargetBehavior>()
            .and_then(|b| {
                // Protocol extensions keep Self abstract (like direct protocol methods)
                // This allows constraint method resolution to work correctly
                if b.is_protocol_extension() {
                    None
                } else {
                    Some(b.target_type().clone())
                }
            }),

        KestrelSymbolKind::Struct => {
            // Downcast Arc<dyn Symbol> to Arc<StructSymbol>
            let struct_arc = Arc::clone(&parent).downcast_arc::<StructSymbol>().ok()?;

            // Build substitutions mapping each type parameter to itself
            let mut substitutions = Substitutions::new();
            if let Some(generics) = parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }

            Some(Ty::generic_struct(struct_arc, substitutions, span))
        },

        KestrelSymbolKind::Enum => {
            // Downcast Arc<dyn Symbol> to Arc<EnumSymbol>
            let enum_arc = Arc::clone(&parent).downcast_arc::<EnumSymbol>().ok()?;

            // Build substitutions mapping each type parameter to itself
            let mut substitutions = Substitutions::new();
            if let Some(generics) = parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }

            Some(Ty::generic_enum(enum_arc, substitutions, span))
        },

        KestrelSymbolKind::Protocol => {
            // Protocol methods keep Self abstract
            None
        },
        _ => None,
    }
}

/// Analyzer that runs type inference on function bodies.
///
/// This analyzer:
/// 1. Gets the `ExecutableBehavior` from each function/initializer
/// 2. Runs type inference via the `InferenceResultFor` query
/// 3. Reports any inference errors as diagnostics
/// 4. Applies the solution to create a `ResolvedExecutableBehavior`
pub struct TypeInferenceAnalyzer;

impl TypeInferenceAnalyzer {
    /// Create a new type inference analyzer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeInferenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeInferenceAnalyzer {
    fn name(&self) -> &'static str {
        "type_inference"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();

        // Only process functions, initializers, getters, setters, and deinits
        if kind != KestrelSymbolKind::Function
            && kind != KestrelSymbolKind::Initializer
            && kind != KestrelSymbolKind::Getter
            && kind != KestrelSymbolKind::Setter
            && kind != KestrelSymbolKind::Deinit
        {
            return;
        }

        // Only process symbols with executable bodies
        let Some(executable) = symbol.metadata().get_behavior::<ExecutableBehavior>() else {
            return;
        };

        // Run type inference via query
        let Some(solution) = ctx.model.query(InferenceResultFor {
            symbol_id: symbol.metadata().id(),
        }) else {
            return;
        };

        // Report any inference errors
        for error in solution.errors() {
            ctx.report(InferenceErrorDiagnostic::from(error.clone()));
        }

        // Apply solution to create resolved body (even if there are errors)
        let resolved_body = apply_solution(executable.body(), &solution);

        // Update local variables in the container with resolved types.
        // This is necessary because pattern-bound locals are created with Ty::infer()
        // placeholder types, and subsequent code reads the type from the LocalContainer.
        // Also resolves SelfType to the concrete type for the `self` local.
        let concrete_self_type = get_concrete_self_type(symbol);
        if let Some(func) = symbol.as_ref().downcast_ref::<FunctionSymbol>() {
            apply_solution_to_locals(
                func as &dyn LocalContainer,
                &solution,
                concrete_self_type.as_ref(),
            );
        } else if let Some(init) = symbol.as_ref().downcast_ref::<InitializerSymbol>() {
            apply_solution_to_locals(
                init as &dyn LocalContainer,
                &solution,
                concrete_self_type.as_ref(),
            );
        } else if let Some(deinit) = symbol.as_ref().downcast_ref::<DeinitSymbol>() {
            apply_solution_to_locals(
                deinit as &dyn LocalContainer,
                &solution,
                concrete_self_type.as_ref(),
            );
        }

        // Add ResolvedExecutableBehavior to the symbol
        symbol
            .metadata()
            .add_behavior(ResolvedExecutableBehavior::new(resolved_body));
    }
}
