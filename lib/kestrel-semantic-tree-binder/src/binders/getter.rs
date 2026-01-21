use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::getter::GetterSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::declaration_binder::{BindingContext, DeclarationBinder};
use kestrel_syntax_tree::utils::find_child;

/// Binder for getter declarations (computed properties)
pub struct GetterBinder;

impl DeclarationBinder for GetterBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _syntax: &SyntaxNode,
        _context: &mut BindingContext,
    ) {
        // Only process getter symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Getter {
            return;
        }

        let span = symbol.metadata().span().clone();

        // Get the parent field symbol to determine the field type and static modifier
        let Some(parent) = symbol.metadata().parent() else {
            return;
        };

        if parent.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        // Get the field type from the parent field's TypedBehavior
        let field_type = parent
            .metadata()
            .get_behavior::<TypedBehavior>()
            .map(|tb| tb.ty().clone())
            .unwrap_or_else(|| Ty::error(span.clone()));

        // Check if the field is static by downcasting to FieldSymbol
        let is_static = parent
            .as_ref()
            .downcast_ref::<FieldSymbol>()
            .map(|f| f.is_static())
            .unwrap_or(false);

        // Determine receiver kind: None for static getters, Borrowing for instance getters
        let receiver_kind = if is_static {
            None
        } else {
            Some(ReceiverKind::Borrowing)
        };

        // Create CallableBehavior:
        // - No parameters
        // - Return type = field type
        // - Receiver = Borrowing for instance getters, None for static getters
        let callable = match receiver_kind {
            Some(kind) => {
                CallableBehavior::with_receiver(vec![], field_type.clone(), kind, span.clone())
            },
            None => CallableBehavior::new(vec![], field_type.clone(), span.clone()),
        };

        symbol.metadata().add_behavior(callable);
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process getter symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Getter {
            return;
        }

        // Check if this getter belongs to a subscript
        // If so, the SubscriptBinder will handle body binding (including parameters)
        if let Some(parent) = symbol.metadata().parent()
            && parent.metadata().kind() == KestrelSymbolKind::Subscript
        {
            // Skip - subscript getter bodies are handled by SubscriptBinder
            return;
        }

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Find the getter body
        // Two cases:
        // 1. Shorthand syntax: syntax IS the CodeBlock directly
        // 2. Explicit syntax: syntax is GetterClause containing a CodeBlock
        let body_node = if syntax.kind() == SyntaxKind::CodeBlock {
            // Shorthand: var x: Int { expr }
            Some(syntax.clone())
        } else {
            // Explicit: var x: Int { get { expr } }
            find_child(syntax, SyntaxKind::CodeBlock)
        };

        if let Some(body_node) = body_node {
            resolve_getter_body(symbol, &body_node, context, &source, file_id);
        }
    }
}

/// Resolve a getter's body and attach ExecutableBehavior to the symbol
fn resolve_getter_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_node: &SyntaxNode,
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::BodyResolutionContext;
    use crate::body_resolver::context::{create_local_scope_for_body, resolve_code_block};
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;

    // Downcast to GetterSymbol
    let Some(_) = symbol.as_ref().downcast_ref::<GetterSymbol>() else {
        return;
    };

    // Create a local scope - getters use a dummy function for local storage
    // since GetterSymbol doesn't implement LocalContainer
    let mut local_scope = create_local_scope_for_body(symbol.clone(), "__getter_body_temp");

    // Get the receiver kind from CallableBehavior to determine if we need to inject `self`
    let callable = symbol.metadata().get_behavior::<CallableBehavior>();
    let has_receiver = callable
        .as_ref()
        .map(|c| c.receiver().is_some())
        .unwrap_or(false);

    // If this is an instance getter, inject `self` as the first local (immutable)
    if has_receiver && let Some(self_type) = get_self_type(symbol) {
        let decl_span = symbol.metadata().span().clone();
        let self_span = Span::new(decl_span.file_id, decl_span.start..decl_span.start);

        // Add self to local scope (immutable - getters only read, not modify)
        local_scope.bind(
            "self".to_string(),
            self_type.clone(),
            false,
            self_span.clone(),
        );
    }

    // Create body resolution context
    let mut body_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        symbol.metadata().id(),
        local_scope,
        None, // Getters don't have their own where clause
    );

    // Resolve the code block and attach ExecutableBehavior
    let code_block = resolve_code_block(body_node, &mut body_ctx);
    let executable = ExecutableBehavior::new(code_block);
    symbol.metadata().add_behavior(executable);
}

/// Get the type of `self` for a getter
///
/// Returns the concrete type of the containing struct/enum (grandparent of the getter).
/// The hierarchy is: Struct/Enum -> Field -> Getter
fn get_self_type(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<Ty> {
    use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
    use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
    use kestrel_semantic_tree::ty::Substitutions;

    // Getter's parent is Field, Field's parent is Struct/Enum/Extension
    let field = symbol.metadata().parent()?;
    let type_parent = field.metadata().parent()?;
    let type_span = type_parent.metadata().span().clone();

    match type_parent.metadata().kind() {
        KestrelSymbolKind::Struct => {
            // Create concrete struct type with type parameters mapping to themselves
            let struct_arc = Arc::clone(&type_parent)
                .downcast_arc::<StructSymbol>()
                .ok()?;
            let mut substitutions = Substitutions::new();
            if let Some(generics) = type_parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), type_span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }
            Some(Ty::generic_struct(struct_arc, substitutions, type_span))
        },
        KestrelSymbolKind::Enum => {
            // Create concrete enum type with type parameters mapping to themselves
            let enum_arc = Arc::clone(&type_parent).downcast_arc::<EnumSymbol>().ok()?;
            let mut substitutions = Substitutions::new();
            if let Some(generics) = type_parent.metadata().get_behavior::<GenericsBehavior>() {
                for param in generics.type_parameters() {
                    let param_id = param.metadata().id();
                    let param_ty = Ty::type_parameter(param.clone(), type_span.clone());
                    substitutions.insert(param_id, param_ty);
                }
            }
            Some(Ty::generic_enum(enum_arc, substitutions, type_span))
        },
        KestrelSymbolKind::Extension => {
            // For extension properties, use the target type
            type_parent
                .metadata()
                .get_behavior::<ExtensionTargetBehavior>()
                .map(|b| b.target_type().clone())
        },
        _ => None,
    }
}
