//! Parameter extraction from CST nodes.
//!
//! CST structure for a Parameter:
//! ```text
//! Parameter "x: Int64"
//!   Pattern "x"
//!     BindingPattern "x"
//!       Identifier "x"
//!   Colon ":"
//!   Ty " Int64"
//!     TyPath ...
//! ```
//! For two-name params like `with x: Int`, there may be an additional
//! label identifier before the Pattern node.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::find_child;

use super::helpers::is_type_kind;

use crate::ast_type::ast_type_from_cst;
use crate::components::{AstParam, Body, FileId, NodeKind, TypeAnnotation};
use crate::lower;

/// Extract parameters from a node containing a ParameterList child.
/// Creates child entities for default value expressions.
pub fn extract_params(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> Vec<AstParam> {
    let param_list = match find_child(node, SyntaxKind::ParameterList) {
        Some(list) => list,
        None => return Vec::new(),
    };

    param_list
        .children()
        .filter(|c| c.kind() == SyntaxKind::Parameter)
        .filter_map(|param_node| extract_single_param(world, &param_node, parent, file_entity, file_id))
        .collect()
}

/// Extract a single parameter from a Parameter CST node.
///
/// The bind name comes from Pattern > BindingPattern > Identifier.
/// A label (if any) is a bare Identifier token at the top level before
/// the Pattern node.
fn extract_single_param(
    world: &mut World,
    node: &SyntaxNode,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> Option<AstParam> {
    // Extract bind name from Pattern > BindingPattern > Identifier
    let name = find_child(node, SyntaxKind::Pattern)
        .and_then(|p| find_child(&p, SyntaxKind::BindingPattern))
        .and_then(|bp| {
            bp.children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::Identifier)
                .map(|t| t.text().to_string())
        })?;

    // Check for a label before the Pattern child. The emitter wraps labels
    // in Name nodes (Name > Identifier), but they could also appear as bare
    // Identifier tokens. Handle both forms.
    let mut label = None;
    for elem in node.children_with_tokens() {
        match elem {
            rowan::NodeOrToken::Node(n) if n.kind() == SyntaxKind::Pattern => break,
            rowan::NodeOrToken::Node(n) if n.kind() == SyntaxKind::Name => {
                // Label wrapped in Name node: Name > Identifier
                label = n.children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::Identifier)
                    .map(|t| t.text().to_string());
            }
            rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::Identifier => {
                label = Some(t.text().to_string());
            }
            rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::Underscore => {
                // Explicit no-label marker — leave label as None
                label = None;
            }
            _ => {}
        }
    }

    // Extract type annotation
    let ty = node
        .children()
        .find(|c| is_type_kind(c.kind()))
        .and_then(|c| ast_type_from_cst(&c, file_id));

    // Create child entity for default value expression
    let default_entity = find_child(node, SyntaxKind::DefaultValue).map(|default_node| {
        let entity = world.spawn();
        world.set(entity, NodeKind::ParamDefault);
        world.set(entity, FileId(file_entity));
        world.set(entity, Body(lower::lower_default_value(&default_node, file_id)));
        // Store the param's type annotation so inference checks the default against it
        if let Some(ref param_ty) = ty {
            world.set(entity, TypeAnnotation(param_ty.clone()));
        }
        world.set_parent(entity, parent);
        entity
    });

    Some(AstParam {
        label,
        name,
        ty,
        default_entity,
    })
}
