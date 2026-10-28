//! What a deriver reads from a type declaration: its name, type parameters,
//! and stored fields or enum cases, as source text.

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxKind;
use kestrel_syntax_tree::ast::{self, AstNode, HasAttributes, HasName, HasStatic};

use crate::build::is_excluded_by_target;
use crate::builders::helpers::extract_attribute;
use crate::components::{Name, NodeKind, TargetConfig};

/// A struct or enum, as a deriver sees it.
#[derive(Clone, Debug)]
pub struct Shape {
    /// The type's own name, as printed by `Formattable`.
    pub name: String,
    /// The name to extend it by: `Outer.Inner` for a nested type.
    pub path: String,
    /// Declared type parameter names, in order.
    pub params: Vec<String>,
    /// The declaration's own `where` constraints, as written.
    pub where_constraints: Vec<String>,
    pub body: ShapeBody,
}

#[derive(Clone, Debug)]
pub enum ShapeBody {
    /// Stored instance fields, in declaration order.
    Struct(Vec<Slot>),
    /// Cases in declaration order (after `@platform` filtering).
    Enum(Vec<Case>),
}

#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    pub payload: Vec<Slot>,
    /// Marked `@default`.
    pub is_default: bool,
}

/// A stored field or an enum payload value.
#[derive(Clone, Debug)]
pub struct Slot {
    /// The field name, or the payload label.
    pub label: Option<String>,
    /// The type, as written.
    pub ty: String,
    /// Whether the type is a plain path (`Array[T]`), so `Ty()` is an expression.
    pub ty_is_path: bool,
    /// The declared initializer, as written (fields only).
    pub init: Option<String>,
    /// Every identifier in the type, for the derived bounds.
    pub idents: Vec<String>,
}

impl Shape {
    /// Read the shape of the struct or enum `item` (built as `entity`).
    pub(crate) fn read(
        world: &World,
        entity: Entity,
        item: &ast::Item,
        file_id: usize,
        target: Option<&TargetConfig>,
    ) -> Option<Shape> {
        let (name, params, where_clause, body) = match item {
            ast::Item::StructDeclaration(n) => (
                n.name_text()?,
                n.type_parameter_list(),
                n.where_clause(),
                ShapeBody::Struct(struct_fields(n, target)),
            ),
            ast::Item::EnumDeclaration(n) => (
                n.name_text()?,
                n.type_parameter_list(),
                n.where_clause(),
                ShapeBody::Enum(enum_cases(n, file_id, target)),
            ),
            _ => return None,
        };
        let params = params
            .into_iter()
            .flat_map(|l| l.type_parameters())
            .filter_map(|p| p.name_text())
            .collect();
        let where_constraints = where_clause
            .into_iter()
            .flat_map(|w| w.where_constraints())
            .map(|c| c.syntax().text().to_string())
            .collect();
        Some(Shape {
            path: type_path(world, entity, &name),
            name,
            params,
            where_constraints,
            body,
        })
    }

    /// Every slot: the fields, or every case's payload.
    pub fn slots(&self) -> Vec<&Slot> {
        match &self.body {
            ShapeBody::Struct(fields) => fields.iter().collect(),
            ShapeBody::Enum(cases) => cases.iter().flat_map(|c| &c.payload).collect(),
        }
    }
}

/// `Outer.Inner` for a type nested in other types, else the name.
fn type_path(world: &World, entity: Entity, name: &str) -> String {
    let mut segments = vec![name.to_string()];
    let mut current = entity;
    while let Some(parent) = world.parent_of(current)
        && matches!(
            world.get::<NodeKind>(parent),
            Some(NodeKind::Struct | NodeKind::Enum)
        )
    {
        segments.extend(world.get::<Name>(parent).map(|n| n.0.clone()));
        current = parent;
    }
    segments.reverse();
    segments.join(".")
}

fn struct_fields(node: &ast::StructDeclaration, target: Option<&TargetConfig>) -> Vec<Slot> {
    node.struct_body()
        .into_iter()
        .flat_map(|b| b.items())
        .filter(|item| !is_excluded_by_target(item, target))
        .filter_map(|item| match item {
            ast::Item::FieldDeclaration(f) => Some(f),
            _ => None,
        })
        // Stored instance fields: no `static`, no accessors.
        .filter(|f| !f.is_static() && f.property_accessors().is_none())
        .filter_map(|f| {
            let ty = f.ty()?;
            Some(Slot {
                label: Some(f.name_text()?),
                init: f.expression().map(|e| e.syntax().text().to_string()),
                ..slot_type(&ty)
            })
        })
        .collect()
}

fn enum_cases(
    node: &ast::EnumDeclaration,
    file_id: usize,
    target: Option<&TargetConfig>,
) -> Vec<Case> {
    node.enum_body()
        .into_iter()
        .flat_map(|b| b.items())
        .filter(|item| !is_excluded_by_target(item, target))
        .filter_map(|item| match item {
            ast::Item::EnumCaseDeclaration(c) => Some(c),
            _ => None,
        })
        .filter_map(|c| {
            let payload = c
                .enum_case_parameter_list()
                .into_iter()
                .flat_map(|l| l.enum_case_parameters())
                .filter_map(|p| {
                    Some(Slot {
                        label: p.name_text(),
                        init: None,
                        ..slot_type(&p.ty()?)
                    })
                })
                .collect();
            let is_default = c
                .attributes()
                .filter_map(|a| extract_attribute(&a, file_id))
                .any(|a| a.name == "default");
            Some(Case {
                name: c.name_text()?,
                payload,
                is_default,
            })
        })
        .collect()
}

/// The type parts of a slot.
fn slot_type(ty: &ast::Ty) -> Slot {
    let syntax = ty.syntax();
    let idents = syntax
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .collect();
    Slot {
        label: None,
        ty: syntax.text().to_string().trim().to_string(),
        ty_is_path: is_path_type(syntax),
        init: None,
        idents,
    }
}

/// A named type (`Int64`, `std.Array[T]`), as opposed to a tuple, function,
/// optional or other sugar.
fn is_path_type(syntax: &kestrel_syntax_tree::SyntaxNode) -> bool {
    let mut node = syntax.clone();
    // `Ty` wraps the concrete type node.
    while node.kind() == SyntaxKind::Ty {
        match node.first_child() {
            Some(child) => node = child,
            None => return false,
        }
    }
    matches!(node.kind(), SyntaxKind::TyPath)
}
