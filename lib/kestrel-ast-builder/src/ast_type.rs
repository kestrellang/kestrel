//! CST-to-AST type lowering.
//!
//! Converts typed `Ty` views into `AstType` data types (defined in
//! kestrel-ast). Every `AstType` is spanned at the `Ty*` kind node it comes
//! from; grouping parens (`TyParen`) are transparent.

use kestrel_span::Span;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

pub use kestrel_ast::{AstType, FnTypeKind, ParamConvention, PathSegment};

/// Lower a `Ty` view.
pub fn lower_type(ty: &ast::Ty, file_id: usize) -> Option<AstType> {
    lower_kind(&ty.ty_kind()?, file_id)
}

/// Lower an optional `Ty` (the common `node.ty()` result).
pub fn lower_opt_type(ty: Option<ast::Ty>, file_id: usize) -> Option<AstType> {
    lower_type(&ty?, file_id)
}

/// Lower every `Ty` of a list, dropping the ones that fail.
pub fn lower_types(types: impl Iterator<Item = ast::Ty>, file_id: usize) -> Vec<AstType> {
    types.filter_map(|t| lower_type(&t, file_id)).collect()
}

/// Untyped entry point: `node` is a `Ty` or one of its kinds.
pub fn ast_type_from_cst(node: &SyntaxNode, file_id: usize) -> Option<AstType> {
    if let Some(ty) = ast::Ty::cast(node.clone()) {
        return lower_type(&ty, file_id);
    }
    lower_kind(&ast::TyKind::cast(node.clone())?, file_id)
}

fn lower_kind(kind: &ast::TyKind, file_id: usize) -> Option<AstType> {
    use ast::TyKind as K;
    let span = node_span(kind.syntax(), file_id);
    let boxed = |t: Option<ast::Ty>| lower_opt_type(t, file_id).map(Box::new);
    Some(match kind {
        K::TyPath(p) => {
            let names = p.path()?.segments();
            if names.is_empty() {
                return None;
            }
            // Type arguments go on the last segment.
            let type_args = p
                .type_argument_list()
                .map(|args| lower_types(args.types(), file_id))
                .unwrap_or_default();
            let last = names.len() - 1;
            let segments = names
                .into_iter()
                .enumerate()
                .map(|(i, name)| PathSegment {
                    name,
                    type_args: if i == last { type_args.clone() } else { vec![] },
                    span: span.clone(),
                })
                .collect();
            AstType::Named { segments, span }
        },
        K::TyTuple(t) => AstType::Tuple(lower_types(t.types(), file_id), span),
        K::TyFunction(f) => lower_fn_type(f, span, file_id),
        K::TyArray(a) => AstType::Array(boxed(a.ty())?, span),
        K::TyDictionary(d) => AstType::Dictionary(boxed(d.key())?, boxed(d.value())?, span),
        K::TyOptional(o) => AstType::Optional(boxed(o.ty())?, span),
        K::TyResult(r) => AstType::Result {
            ok: boxed(r.ty())?,
            err: boxed(r.error())?,
            span,
        },
        K::TyUnit(_) => AstType::Unit(span),
        K::TyNever(_) => AstType::Never(span),
        K::TyInferred(_) => AstType::Inferred(span),
        K::TyRef(r) => AstType::Ref {
            inner: boxed(r.ty())?,
            mutating: false,
            span,
        },
        K::TyMutRef(r) => AstType::Ref {
            inner: boxed(r.ty())?,
            mutating: true,
            span,
        },
        K::TySome(s) => {
            // Positive bounds are the direct `Ty` children; the negative
            // bound (`and not Copyable`) sits in a NegativeConformance.
            let bounds = lower_types(s.types(), file_id);
            if bounds.is_empty() {
                return None;
            }
            let negative = s
                .negative_conformance()
                .and_then(|n| lower_opt_type(n.ty(), file_id))
                .map(Box::new);
            AstType::Some {
                bounds,
                negative,
                span,
            }
        },
        // Grouping parens are transparent: the type keeps the inner span.
        K::TyParen(p) => return lower_opt_type(p.ty(), file_id),
    })
}

/// `kind? (mutating? T, …) -> R`. A `mutating` inside the list marks the
/// parameter after it as `MutBorrow`; every other parameter is `Consuming`.
/// The node's span covers the kind keyword (LSP signature help slices
/// source by it).
fn lower_fn_type(f: &ast::TyFunction, span: Span, file_id: usize) -> AstType {
    let mut params = Vec::new();
    let mut param_conventions = Vec::new();
    if let Some(list) = f.ty_list() {
        let mut pending_mut = false;
        for child in list.syntax().children_with_tokens() {
            if child.kind() == SyntaxKind::Mutating {
                pending_mut = true;
                continue;
            }
            let Some(ty) = child.into_node().and_then(ast::Ty::cast) else {
                continue;
            };
            if let Some(ty) = lower_type(&ty, file_id) {
                params.push(ty);
                param_conventions.push(if pending_mut {
                    ParamConvention::MutBorrow
                } else {
                    ParamConvention::Consuming
                });
            }
            pending_mut = false;
        }
    }
    let return_type = lower_opt_type(f.ty(), file_id).unwrap_or(AstType::Unit(span.clone()));
    AstType::Function {
        kind: fn_type_kind(f),
        params,
        param_conventions,
        return_type: Box::new(return_type),
        span,
    }
}

/// The kind keyword before the parameter list. `escaping` is contextual,
/// so it arrives as an `Identifier` and is matched by text.
fn fn_type_kind(f: &ast::TyFunction) -> FnTypeKind {
    let Some(tok) = f.kind_token() else {
        return FnTypeKind::Normal;
    };
    match tok.kind() {
        SyntaxKind::Mutating => FnTypeKind::Mutating,
        SyntaxKind::Consuming => FnTypeKind::Consuming,
        SyntaxKind::Identifier if tok.text() == "escaping" => FnTypeKind::Escaping,
        _ => FnTypeKind::Normal,
    }
}

/// Byte-offset span of a syntax node.
pub(crate) fn node_span(node: &SyntaxNode, file_id: usize) -> Span {
    let range = node.text_range();
    Span::new(file_id, range.start().into()..range.end().into())
}
