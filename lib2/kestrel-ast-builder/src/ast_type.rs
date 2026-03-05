//! AST-level type representation.
//!
//! Types are data (not entities). Extracted from the CST during build so
//! downstream queries don't need to touch CstNodes for type information.
//! Spans are included for error reporting.

use kestrel_span2::Span;
use kestrel_syntax_tree2::{SyntaxKind, SyntaxNode};
use kestrel_syntax_tree2::utils::{extract_path_segments, find_child};

/// A single segment in a qualified type path.
/// Each segment has a name and optional type arguments.
/// e.g. in `Array[Int].Iterator`, `Array[Int]` and `Iterator` are segments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PathSegment {
    pub name: String,
    pub type_args: Vec<AstType>,
    pub span: Span,
}

/// AST-level type representation extracted from CST.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AstType {
    /// Named type with path segments, each optionally having type arguments.
    /// e.g. `Int64`, `std.collections.Array[Int64]`, `Array[Int].Iterator`
    Named {
        segments: Vec<PathSegment>,
        span: Span,
    },
    /// Tuple type, e.g. `(Int, String)`
    Tuple(Vec<AstType>, Span),
    /// Function type, e.g. `(Int) -> String`
    Function {
        params: Vec<AstType>,
        return_type: Box<AstType>,
        span: Span,
    },
    /// Array type, e.g. `[Int]`
    Array(Box<AstType>, Span),
    /// Dictionary type, e.g. `[String: Int]`
    Dictionary(Box<AstType>, Box<AstType>, Span),
    /// Optional type, e.g. `Int?`
    Optional(Box<AstType>, Span),
    /// Result type, e.g. `Int throws Error`
    Result {
        ok: Box<AstType>,
        err: Box<AstType>,
        span: Span,
    },
    /// Unit type `()`
    Unit(Span),
    /// Never type `Never`
    Never(Span),
    /// Inferred type `_`
    Inferred(Span),
}

/// Convert a CST type node to an AstType.
///
/// Walks the Ty* nodes from the syntax tree and produces the corresponding
/// AstType variant. Returns None for unrecognized nodes.
pub fn ast_type_from_cst(node: &SyntaxNode, file_id: usize) -> Option<AstType> {
    let span = node_span(node, file_id);

    match node.kind() {
        SyntaxKind::TyPath => {
            // Extract path segments from the Path child.
            // Currently the parser puts type args on the TyPath (end of path),
            // but we build per-segment PathSegments for forward compatibility
            // with `Array[Int].Iterator` style paths.
            let path_node = find_child(node, SyntaxKind::Path)?;
            let names = extract_path_segments(&path_node);
            if names.is_empty() {
                return None;
            }

            // Type arguments (currently only on the last segment)
            let type_args: Vec<AstType> = find_child(node, SyntaxKind::TypeArgumentList)
                .map(|args_node| {
                    args_node
                        .children()
                        .filter(|c| is_type_node(c.kind()))
                        .filter_map(|c| ast_type_from_cst(&c, file_id))
                        .collect()
                })
                .unwrap_or_default();

            // Build PathSegments — type args go on the last segment
            let segments: Vec<PathSegment> = names
                .iter()
                .enumerate()
                .map(|(i, name)| PathSegment {
                    name: name.clone(),
                    type_args: if i == names.len() - 1 { type_args.clone() } else { vec![] },
                    span: span.clone(),
                })
                .collect();

            Some(AstType::Named { segments, span })
        }

        SyntaxKind::TyTuple => {
            let elements: Vec<AstType> = node
                .children()
                .filter(|c| is_type_node(c.kind()))
                .filter_map(|c| ast_type_from_cst(&c, file_id))
                .collect();
            Some(AstType::Tuple(elements, span))
        }

        SyntaxKind::TyFunction => {
            // CST structure: TyFunction has exactly 2 children:
            //   1. TyList — parameter types (may be empty or contain Ty children)
            //   2. Ty — return type
            let mut children = node.children();

            // First child: TyList with parameter types
            let params = children.next()
                .filter(|c| c.kind() == SyntaxKind::TyList)
                .map(|ty_list| {
                    ty_list.children()
                        .filter(|c| is_type_node(c.kind()))
                        .filter_map(|c| ast_type_from_cst(&c, file_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            // Second child: return type
            let return_type = children.next()
                .filter(|c| is_type_node(c.kind()))
                .and_then(|c| ast_type_from_cst(&c, file_id))
                .unwrap_or(AstType::Unit(span.clone()));

            Some(AstType::Function { params, return_type: Box::new(return_type), span })
        }

        SyntaxKind::TyArray => {
            let inner = node
                .children()
                .find(|c| is_type_node(c.kind()))
                .and_then(|c| ast_type_from_cst(&c, file_id))?;
            Some(AstType::Array(Box::new(inner), span))
        }

        SyntaxKind::TyDictionary => {
            let mut types = node
                .children()
                .filter(|c| is_type_node(c.kind()))
                .filter_map(|c| ast_type_from_cst(&c, file_id));
            let key = types.next()?;
            let value = types.next()?;
            Some(AstType::Dictionary(Box::new(key), Box::new(value), span))
        }

        SyntaxKind::TyOptional => {
            let inner = node
                .children()
                .find(|c| is_type_node(c.kind()))
                .and_then(|c| ast_type_from_cst(&c, file_id))?;
            Some(AstType::Optional(Box::new(inner), span))
        }

        SyntaxKind::TyResult => {
            let mut types = node
                .children()
                .filter(|c| is_type_node(c.kind()))
                .filter_map(|c| ast_type_from_cst(&c, file_id));
            let ok = types.next()?;
            let err = types.next()?;
            Some(AstType::Result {
                ok: Box::new(ok),
                err: Box::new(err),
                span,
            })
        }

        SyntaxKind::TyUnit => Some(AstType::Unit(span)),
        SyntaxKind::TyNever => Some(AstType::Never(span)),
        SyntaxKind::TyInferred => Some(AstType::Inferred(span)),

        // For wrapper nodes like Ty, recurse into the child
        SyntaxKind::Ty => {
            node.children()
                .find(|c| is_type_node(c.kind()))
                .and_then(|c| ast_type_from_cst(&c, file_id))
        }

        _ => None,
    }
}

/// Check if a SyntaxKind is a type node.
fn is_type_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Ty
            | SyntaxKind::TyPath
            | SyntaxKind::TyTuple
            | SyntaxKind::TyFunction
            | SyntaxKind::TyArray
            | SyntaxKind::TyDictionary
            | SyntaxKind::TyOptional
            | SyntaxKind::TyResult
            | SyntaxKind::TyUnit
            | SyntaxKind::TyNever
            | SyntaxKind::TyInferred
    )
}

/// Get byte-offset span from a syntax node.
fn node_span(node: &SyntaxNode, file_id: usize) -> Span {
    let range = node.text_range();
    let start: usize = range.start().into();
    let end: usize = range.end().into();
    Span::new(file_id, start..end)
}
