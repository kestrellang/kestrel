//! Function, initializer, and deinit declaration builders.

use kestrel_ast::{AstType, PathSegment};
use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNode;
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName, HasStatic};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::params::extract_params;
use super::type_param::build_type_parameters;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build a function declaration entity from CST.
///
/// Components: NodeKind::Function, Name, FileId, Vis, Callable,
/// [TypeAnnotation (return)], [Valued (body)], [Static],
/// [TypeParams], [WhereClause], [Attributes], [Documentation]
pub fn build_function(
    world: &mut World,
    node: &ast::FunctionDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Function);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(SyntaxNodePtr::new(&syntax)));
    world.set_parent(entity, parent);

    if let Some(name) = node.name_text() {
        world.set(entity, Name(name));
    }

    // Non-static functions inside type declarations are methods. An explicit
    // `mutating`/`consuming` picks the receiver; the default is Borrowing.
    let is_static = node.is_static();
    let parent_is_type = world
        .get::<NodeKind>(parent)
        .is_some_and(NodeKind::is_type_scope);
    let receiver = if is_static || !parent_is_type {
        None
    } else if node.mutating_token().is_some() {
        Some(ReceiverKind::Mutating)
    } else if node.consuming_token().is_some() {
        Some(ReceiverKind::Consuming)
    } else {
        Some(ReceiverKind::Borrowing)
    };

    // Parameters (creates child entities for default value expressions)
    let params = extract_params(world, node.parameter_list(), entity, file_entity, file_id);
    world.set(entity, Callable { params, receiver });

    if let Some(ty) = node
        .return_type()
        .and_then(|r| lower_opt_type(r.ty(), file_id))
    {
        world.set(entity, TypeAnnotation(ty));
    }

    // `{ … }` or `= expr`
    if let Some(body) = node.function_body() {
        if let Some(code_block) = body.code_block() {
            set_block_body(world, entity, &code_block);
        } else {
            world.set(entity, Valued(SyntaxNodePtr::new(body.syntax())));
        }
    }

    if is_static {
        world.set(entity, Static);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
    desugar_opaque_params(world, entity, file_entity, syntax);
}

/// Point the entity's `Valued` body at a `{ … }` block.
fn set_block_body(world: &mut World, entity: Entity, block: &ast::CodeBlock) {
    world.set(entity, Valued(SyntaxNodePtr::new(block.syntax())));
}

/// Build an initializer declaration entity from CST.
///
/// Components: NodeKind::Initializer, FileId, Vis, Callable,
/// [InitEffect], [TypeAnnotation (return)],
/// [Valued (body)], [TypeParams], [WhereClause], [Attributes], [Documentation]
pub fn build_initializer(
    world: &mut World,
    node: &ast::InitializerDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Initializer);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(SyntaxNodePtr::new(&syntax)));
    world.set_parent(entity, parent);

    let params = extract_params(world, node.parameter_list(), entity, file_entity, file_id);
    // Inits always have a `self` receiver (mutating — they're building the instance)
    world.set(
        entity,
        Callable {
            params,
            receiver: Some(ReceiverKind::Mutating),
        },
    );

    // Init effect: `?` (failable) or `throws E` (throwing). The body's
    // return type becomes `()?` or `() throws E`.
    if let Some(effect) = node.init_effect() {
        let effect_span = get_decl_span(effect.syntax(), file_id);
        let unit_ty = AstType::Unit(effect_span.clone());
        if effect.question_token().is_some() {
            world.set(entity, InitEffect::Failable);
            world.set(
                entity,
                TypeAnnotation(AstType::Optional(Box::new(unit_ty), effect_span)),
            );
        } else if let Some(err_ty) = lower_opt_type(effect.ty(), file_id) {
            world.set(entity, InitEffect::Throwing);
            world.set(
                entity,
                TypeAnnotation(AstType::Result {
                    ok: Box::new(unit_ty),
                    err: Box::new(err_ty),
                    span: effect_span,
                }),
            );
        }
    }

    if let Some(block) = node.function_body().and_then(|b| b.code_block()) {
        set_block_body(world, entity, &block);
    }

    set_visibility(world, entity, node);
    set_attributes(world, entity, node, file_id);
    set_documentation(world, entity, syntax);
    set_where_clause(world, entity, node, file_id);
    build_type_parameters(world, entity, node, file_entity, file_id);
}

/// Build a deinit declaration entity from CST.
///
/// Components: NodeKind::Deinit, FileId, [Valued (body)]
pub fn build_deinit(
    world: &mut World,
    node: &ast::DeinitDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Deinit);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(SyntaxNodePtr::new(&syntax)));
    world.set_parent(entity, parent);

    // Deinits receive &var self. The caller owns the memory and handles
    // deallocation after the deinit body runs cleanup.
    world.set(
        entity,
        Callable {
            params: Vec::new(),
            receiver: Some(ReceiverKind::Mutating),
        },
    );

    if let Some(block) = node.function_body().and_then(|b| b.code_block()) {
        set_block_body(world, entity, &block);
    }
}

/// Desugar `some P` in parameter types to synthetic type parameters.
///
/// `func draw(shape: some Drawable)` becomes:
/// `func draw[__opaque_0: Drawable](shape: __opaque_0)`
///
/// Each `some` creates an independent synthetic type parameter.
fn desugar_opaque_params(
    world: &mut World,
    func_entity: Entity,
    file_entity: Entity,
    cst_node: &SyntaxNode,
) {
    let callable = match world.get::<Callable>(func_entity) {
        Some(c) => c.clone(),
        None => return,
    };

    let mut opaque_index = 0u32;
    let mut synthetic_params: Vec<Entity> = Vec::new();
    let mut new_where_constraints: Vec<WhereConstraint> = Vec::new();
    let mut new_callable_params = callable.params.clone();
    let mut changed = false;

    for param in &mut new_callable_params {
        if let Some(AstType::Some {
            bounds,
            negative,
            span,
        }) = &param.ty
        {
            let tp_name = format!("__opaque_{}", opaque_index);
            let tp_span = span.clone();
            opaque_index += 1;

            let tp = world.spawn();
            world.set(tp, NodeKind::TypeParameter);
            world.set(tp, Name(tp_name.clone()));
            world.set(tp, FileId(file_entity));
            world.set(tp, DeclSpan(tp_span.clone()));
            world.set(tp, CstNode(SyntaxNodePtr::new(&cst_node)));
            world.set_parent(tp, func_entity);
            synthetic_params.push(tp);

            let tp_ast = AstType::Named {
                segments: vec![PathSegment {
                    name: tp_name,
                    type_args: Vec::new(),
                    span: tp_span.clone(),
                }],
                span: tp_span,
            };

            new_where_constraints.push(WhereConstraint::Bound {
                subject: tp_ast.clone(),
                protocols: bounds.clone(),
                node: SyntaxNodePtr::new(&cst_node),
            });

            // `some P and not Copyable` in param position desugars to the
            // existing `where __opaque_N: not Copyable` machinery.
            if let Some(negative) = negative {
                new_where_constraints.push(WhereConstraint::NegativeBound {
                    subject: tp_ast.clone(),
                    protocol: (**negative).clone(),
                    node: SyntaxNodePtr::new(&cst_node),
                });
            }

            param.ty = Some(tp_ast);
            changed = true;
        }
    }

    if !changed {
        return;
    }

    world.set(
        func_entity,
        Callable {
            params: new_callable_params,
            receiver: callable.receiver,
        },
    );

    if !synthetic_params.is_empty() {
        let combined = match world.get::<TypeParams>(func_entity) {
            Some(existing) => {
                let mut out = existing.0.clone();
                out.extend(synthetic_params);
                out
            },
            None => synthetic_params,
        };
        world.set(func_entity, TypeParams(combined));
    }

    if !new_where_constraints.is_empty() {
        let combined = match world.get::<WhereClause>(func_entity) {
            Some(existing) => {
                let mut out = existing.0.clone();
                out.extend(new_where_constraints);
                out
            },
            None => new_where_constraints,
        };
        world.set(func_entity, WhereClause(combined));
    }
}
