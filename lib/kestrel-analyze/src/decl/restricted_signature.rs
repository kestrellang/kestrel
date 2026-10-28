//! # Restricted Signature Analyzer (declaration half)
//!
//! Reports `@deprecated` and `@unavailable` types named in a declaration's
//! signature: parameter, return, field and payload types, type alias
//! targets, conformance lists, `where` clauses, type parameter defaults and
//! the type arguments of an `extend` target. Bodies are the body half's job
//! (`body/restricted_use.rs`), which owns the codes, the context exemption
//! and the wording; this half only finds the types.
//!
//! The extended type itself (`Old` in `extend Old`) is not a use: an
//! extension is part of the type it extends, the way its members are.
//!
//! ## Diagnostics
//!
//! E908 and E913, borrowed from `body/restricted_use.rs`, with the primary
//! label on the type as written (the `AstType` span).

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, ConformanceItem, Conformances, ExtensionTarget, NodeKind, TypeAnnotation,
    WhereClause, WhereConstraint,
};
use kestrel_name_res::{ResolveTypePath, TypeResolution};

use crate::body::restricted_use::{self, UseSink};
use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};

static BORROWED: &[&DiagnosticDescriptor] = &[
    &restricted_use::DESCRIPTORS[0],
    &restricted_use::DESCRIPTORS[1],
];

pub struct RestrictedSignatureAnalyzer;

impl Describe for RestrictedSignatureAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::RestrictedSignature
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        &[]
    }
    fn borrowed_descriptors(&self) -> &'static [&'static DiagnosticDescriptor] {
        // E908/E913: the same "uses a restricted declaration" fact the body
        // half reports, found in a signature instead of a body.
        BORROWED
    }
}

impl DeclCheck for RestrictedSignatureAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::EnumCase,
            NodeKind::Protocol,
            NodeKind::Extension,
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Subscript,
            NodeKind::Field,
            NodeKind::TypeAlias,
            NodeKind::TypeParameter,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut walk = SignatureWalk {
            cx,
            sink: UseSink::new(cx.query, cx.entity, cx.root),
        };
        let q = cx.query;
        if let Some(TypeAnnotation(ty)) = q.get::<TypeAnnotation>(cx.entity) {
            walk.ty(ty);
        }
        if let Some(callable) = q.get::<Callable>(cx.entity) {
            callable
                .params
                .iter()
                .filter_map(|p| p.ty.as_ref())
                .for_each(|t| walk.ty(t));
        }
        if let Some(Conformances(items)) = q.get::<Conformances>(cx.entity) {
            for item in items {
                let (ConformanceItem::Positive(ty, _) | ConformanceItem::Negative(ty, _)) = item;
                walk.ty(ty);
            }
        }
        if let Some(WhereClause(constraints)) = q.get::<WhereClause>(cx.entity) {
            for c in constraints {
                match c {
                    WhereConstraint::Bound {
                        subject, protocols, ..
                    } => {
                        walk.ty(subject);
                        protocols.iter().for_each(|p| walk.ty(p));
                    },
                    WhereConstraint::Equality { lhs, rhs, .. } => {
                        walk.ty(lhs);
                        walk.ty(rhs);
                    },
                    WhereConstraint::NegativeBound {
                        subject, protocol, ..
                    } => {
                        walk.ty(subject);
                        walk.ty(protocol);
                    },
                }
            }
        }
        if let Some(ExtensionTarget(target)) = q.get::<ExtensionTarget>(cx.entity) {
            walk.type_args_only(target);
        }
        walk.sink.finish()
    }
}

struct SignatureWalk<'a> {
    cx: &'a DeclContext<'a>,
    sink: UseSink<'a>,
}

impl SignatureWalk<'_> {
    fn ty(&mut self, ty: &AstType) {
        match ty {
            AstType::Named { segments, span } => {
                let segs = segments.iter().map(|s| s.name.clone()).collect();
                if let TypeResolution::Found(e) = self.cx.query.query(ResolveTypePath {
                    segments: segs,
                    context: self.cx.entity,
                    root: self.sink.root(),
                }) {
                    self.sink.entity(e, span);
                }
                self.type_args_only(ty);
            },
            AstType::Tuple(elems, _) => elems.iter().for_each(|e| self.ty(e)),
            AstType::Function {
                params,
                return_type,
                ..
            } => {
                params.iter().for_each(|p| self.ty(p));
                self.ty(return_type);
            },
            AstType::Array(inner, _) | AstType::Optional(inner, _) | AstType::Ref { inner, .. } => {
                self.ty(inner)
            },
            AstType::Dictionary(k, v, _) => {
                self.ty(k);
                self.ty(v);
            },
            AstType::Result { ok, err, .. } => {
                self.ty(ok);
                self.ty(err);
            },
            AstType::Some { bounds, .. } => bounds.iter().for_each(|b| self.ty(b)),
            AstType::Any { .. } => ty.any_children().into_iter().for_each(|t| self.ty(t)),
            AstType::Unit(_) | AstType::Never(_) | AstType::Inferred(_) => {},
        }
    }

    /// The type arguments of a named type, not the type itself.
    fn type_args_only(&mut self, ty: &AstType) {
        let AstType::Named { segments, .. } = ty else {
            return;
        };
        for seg in segments {
            seg.type_args.iter().for_each(|a| self.ty(a));
        }
    }
}
