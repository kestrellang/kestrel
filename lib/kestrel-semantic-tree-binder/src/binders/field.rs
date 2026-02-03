use std::sync::Arc;

use kestrel_semantic_tree::behavior::ComputedMemberAccessBehavior;
use kestrel_semantic_tree::behavior::FileConstantBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::binders::utils::attributes::{parse_fileconstant_attribute, FileConstantParseResult};
use crate::body_resolver::BodyResolutionContext;
use crate::body_resolver::context::create_local_scope_for_body;
use crate::declaration_binder::{BindingContext, DeclarationBinder};
use crate::resolution::type_resolver::{TypeSyntaxContext, resolve_type_from_ty_node};

/// Binder for field declarations
pub struct FieldBinder;

impl DeclarationBinder for FieldBinder {
    fn bind_signature(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process field symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().span().clone();

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve attributes
        let attributes_behavior = crate::binders::utils::attributes::resolve_attributes(
            syntax,
            &source,
            file_id,
            context.diagnostics,
        );

        // Check for @fileconstant attribute
        match parse_fileconstant_attribute(&attributes_behavior, &source, context.diagnostics) {
            FileConstantParseResult::Success {
                relative_path,
                span,
            } => {
                // Attach FileConstantBehavior - file data will be loaded during lowering
                let file_constant_behavior = FileConstantBehavior::new(relative_path, span);
                symbol.metadata().add_behavior(file_constant_behavior);
            },
            FileConstantParseResult::NotFileConstant => {
                // Normal field, no action needed
            },
            FileConstantParseResult::Error => {
                // Error already reported by parse_fileconstant_attribute
            },
        }

        symbol.metadata().add_behavior(attributes_behavior);

        // Resolve the type directly from syntax
        let resolved_type =
            resolve_field_type_from_syntax(syntax, &source, file_id, symbol_id, context);

        // Add a TypedBehavior with the resolved type
        let typed_behavior = TypedBehavior::new(resolved_type.clone(), span.clone());
        symbol.metadata().add_behavior(typed_behavior);

        // Determine if this field needs ValueBehavior for path resolution.
        // Module-level fields (global properties) and static fields can be accessed
        // directly by name or via Type.field, so they need ValueBehavior.
        let is_module_level = symbol
            .metadata()
            .parent()
            .map(|p| {
                let kind = p.metadata().kind();
                kind == KestrelSymbolKind::Module || kind == KestrelSymbolKind::SourceFile
            })
            .unwrap_or(false);

        // Add appropriate member access behavior so this field can be accessed via dot notation
        let field_name = symbol.metadata().name().value.clone();

        // Get field properties from the FieldSymbol
        if let Some(field_symbol) = symbol.as_ref().downcast_ref::<FieldSymbol>() {
            let is_static = field_symbol.is_static();

            // Add ValueBehavior for module-level or static fields
            // This allows them to be resolved as values in path expressions
            if is_module_level || is_static {
                let value_behavior = ValueBehavior::new(resolved_type.clone(), span.clone());
                symbol.metadata().add_behavior(value_behavior);
            }

            if field_symbol.is_computed() {
                // Computed property: use ComputedMemberAccessBehavior
                let getter_id = field_symbol
                    .getter()
                    .expect("computed property must have a getter");
                let setter_id = field_symbol.setter();
                let computed_behavior = ComputedMemberAccessBehavior::new(
                    field_name,
                    resolved_type,
                    getter_id,
                    setter_id,
                );
                symbol.metadata().add_behavior(computed_behavior);
            } else {
                // Stored field: use MemberAccessBehavior
                let is_mutable = field_symbol.is_mutable();
                let member_access_behavior =
                    MemberAccessBehavior::new(field_name, resolved_type, is_mutable);
                symbol.metadata().add_behavior(member_access_behavior);
            }
        } else {
            // Fallback: treat as immutable stored field
            // Also add ValueBehavior for module-level fields
            if is_module_level {
                let value_behavior = ValueBehavior::new(resolved_type.clone(), span.clone());
                symbol.metadata().add_behavior(value_behavior);
            }
            let member_access_behavior =
                MemberAccessBehavior::new(field_name, resolved_type, false);
            symbol.metadata().add_behavior(member_access_behavior);
        }
    }

    fn bind_body(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        // Only process field symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Field {
            return;
        }

        // Skip file constants - they don't have runtime initializers
        if symbol
            .metadata()
            .get_behavior::<FileConstantBehavior>()
            .is_some()
        {
            return;
        }

        // Only process static fields with initializers
        let Some(field_symbol) = symbol.as_ref().downcast_ref::<FieldSymbol>() else {
            return;
        };

        // Skip computed properties (they use getter/setter bodies instead)
        if field_symbol.is_computed() {
            return;
        }

        // Check if this is a module-level field (implicitly static)
        let is_module_level = symbol
            .metadata()
            .parent()
            .map(|p| {
                let kind = p.metadata().kind();
                kind == KestrelSymbolKind::Module || kind == KestrelSymbolKind::SourceFile
            })
            .unwrap_or(false);

        // Skip non-static instance fields (their initialization is handled by constructors)
        // Module-level fields are implicitly static even without the 'static' keyword
        if !field_symbol.is_static() && !is_module_level {
            return;
        }

        // Find the initializer expression in syntax
        // Structure: ... Equals Expression ...
        let initializer_expr = find_initializer_expression(syntax);
        let Some(expr_node) = initializer_expr else {
            return;
        };

        let source = context.source_for_symbol(symbol);
        let file_id = context.file_id_for_symbol(symbol);

        // Resolve the initializer expression
        resolve_static_field_initializer(symbol, &expr_node, context, &source, file_id);
    }
}

/// Find the initializer expression in a field declaration syntax node
/// Returns the expression node after the Equals token, if present
fn find_initializer_expression(syntax: &SyntaxNode) -> Option<SyntaxNode> {
    let mut found_equals = false;

    for child in syntax.children_with_tokens() {
        if let Some(token) = child.as_token() {
            if token.kind() == SyntaxKind::Equals {
                found_equals = true;
            }
        } else if let Some(node) = child.as_node() {
            // After Equals, the next Expression-like node is the initializer
            // Skip Ty nodes and PropertyAccessors
            if found_equals
                && node.kind() != SyntaxKind::Ty
                && node.kind() != SyntaxKind::PropertyAccessors
            {
                return Some(node.clone());
            }
        }
    }

    None
}

/// Resolve a static field's initializer and attach ExecutableBehavior to the symbol
fn resolve_static_field_initializer(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    expr_node: &SyntaxNode,
    context: &mut BindingContext,
    source: &str,
    file_id: usize,
) {
    use crate::body_resolver::context::resolve_expression_to_code_block;
    use kestrel_semantic_tree::behavior::typed::TypedBehavior;

    // Create a local scope for resolving the initializer expression
    let local_scope = create_local_scope_for_body(symbol.clone(), "__static_init_temp");

    // Create resolution context
    let mut resolution_ctx = BodyResolutionContext::new_with_scope(
        context.model,
        context.diagnostics,
        source,
        file_id,
        symbol.metadata().id(),
        local_scope,
        None, // Static initializers don't have their own where clause
    );

    // Resolve the expression into a CodeBlock
    let mut code_block = resolve_expression_to_code_block(expr_node, &mut resolution_ctx);

    // Apply the field's declared type to the initializer if it has an inference type.
    // This ensures literals like `1` get properly typed as `Int64` instead of staying as raw i64.
    if let Some(typed_behavior) = symbol.metadata().get_behavior::<TypedBehavior>() {
        let field_type = typed_behavior.ty();
        if let Some(ref mut yield_expr) = code_block.yield_expr
            && yield_expr.ty.is_infer()
        {
            yield_expr.ty = field_type.clone();
        }
    }

    // Attach ExecutableBehavior to the field symbol
    let executable = ExecutableBehavior::new(code_block);
    symbol.metadata().add_behavior(executable);
}

/// Resolve the field type from a FieldDeclaration syntax node
/// This extracts the type from syntax and immediately resolves it
fn resolve_field_type_from_syntax(
    syntax: &SyntaxNode,
    source: &str,
    file_id: usize,
    context_id: semantic_tree::symbol::SymbolId,
    ctx: &mut BindingContext,
) -> Ty {
    // Find the Ty node and resolve using shared utility
    if let Some(ty_node) = syntax
        .children()
        .find(|child| child.kind() == SyntaxKind::Ty)
    {
        let mut type_ctx =
            TypeSyntaxContext::new(ctx.model, ctx.diagnostics, source, file_id, context_id);
        return resolve_type_from_ty_node(&ty_node, &mut type_ctx);
    }

    // No type found - return error
    Ty::error(Span::new(file_id, 0..0))
}
