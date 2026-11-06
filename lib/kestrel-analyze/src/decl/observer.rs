//! # Observer Analyzer
//!
//! Validates `willSet` / `didSet` on a property.
//!
//! ## Diagnostics
//!
//! ### E638 -- `invalid_observer` (Error, Correctness)
//! `willSet` / `didSet` anywhere but an instance stored `var` of a concrete
//! type: mixed with `get`/`set`/`ref` accessors, on a subscript, a `let`, a
//! static or global, a failable member, or in a protocol. Also a `return`
//! in an observer body: both observers are inlined into one synthesized
//! setter around the store, so a `return` in `willSet` would skip the
//! store.

use crate::context::DeclContext;
use crate::decl::place_accessor::{has_instance_receiver, in_protocol_scope};
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Computed, DeclaredAccessors, Failable, FieldMutability, NodeKind};
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E638",
    name: "invalid_observer",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ObserverAnalyzer;

impl Describe for ObserverAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::Observer
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ObserverAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Subscript, NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let declared = cx
            .query
            .get::<DeclaredAccessors>(cx.entity)
            .copied()
            .unwrap_or_default();
        if !declared.any_observer() {
            return Vec::new();
        }
        if let Some((message, note)) = misplaced(cx, &declared) {
            let span = util::entity_span(cx.query, cx.entity);
            return vec![diagnostic(span, "declares observers", message, note)];
        }
        observer_returns(cx)
            .into_iter()
            .map(|span| {
                diagnostic(
                    span,
                    "returns from the property's setter",
                    "`return` is not allowed in `willSet` / `didSet`",
                    "both observers run inside the setter around the store, so a `return` \
                     would skip the rest of the write; use `if` to skip code instead",
                )
            })
            .collect()
    }
}

fn diagnostic(
    span: kestrel_span::Span,
    label: &str,
    message: &str,
    note: &str,
) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: message.into(),
        labels: vec![DiagLabel {
            span,
            message: label.into(),
            is_primary: true,
        }],
        notes: vec![note.into()],
    }
}

/// Why observers can't go on this member: they belong on an instance
/// stored `var` of a concrete type. The first rule broken is reported.
fn misplaced(
    cx: &DeclContext<'_>,
    declared: &DeclaredAccessors,
) -> Option<(&'static str, &'static str)> {
    let q = cx.query;
    Some(if q.get::<NodeKind>(cx.entity) != Some(&NodeKind::Field) {
        (
            "subscripts can't have `willSet` / `didSet` observers",
            "observe the storage the subscript writes instead",
        )
    } else if q.has::<Computed>(cx.entity) || declared.set || declared.mutating_ref {
        (
            "`willSet` / `didSet` can't be combined with `get`, `set` or `ref` accessors",
            "observers watch a stored property; a computed property can run code in its `set`",
        )
    } else if q.get::<FieldMutability>(cx.entity) == Some(&FieldMutability::Let) {
        (
            "a `let` property can't have `willSet` / `didSet` observers",
            "it is never written after initialization; declare it `var` to observe writes",
        )
    } else if q.has::<Failable>(cx.entity) {
        (
            "a failable property can't have `willSet` / `didSet` observers",
            "a failable place is read-only",
        )
    } else if in_protocol_scope(cx) {
        (
            "`willSet` / `didSet` are not allowed in protocols or protocol extensions",
            "declare the observed property on a concrete type",
        )
    } else if !has_instance_receiver(cx) {
        (
            "`willSet` / `didSet` on static and global properties are not supported yet",
            "observers currently need an instance stored property",
        )
    } else {
        return None;
    })
}

/// The span of each `return` in an observer body, outside nested closures
/// (a closure's `return` leaves only the closure).
fn observer_returns(cx: &DeclContext<'_>) -> Vec<kestrel_span::Span> {
    let Some(accessors) = kestrel_ast_builder::syntax::cst_node(cx.query, cx.entity)
        .and_then(|decl| decl.descendants().find_map(ast::PropertyAccessors::cast))
    else {
        return Vec::new();
    };
    let bodies = [
        accessors.will_set().and_then(|c| c.code_block()),
        accessors.did_set().and_then(|c| c.code_block()),
    ];
    let mut out = Vec::new();
    for body in bodies.into_iter().flatten() {
        collect_returns(body.syntax(), &mut out);
    }
    let file = util::entity_span(cx.query, cx.entity).file_id;
    out.into_iter()
        .map(|range| kestrel_span::Span::new(file, range))
        .collect()
}

fn collect_returns(node: &SyntaxNode, out: &mut Vec<std::ops::Range<usize>>) {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::ExprClosure => {},
            SyntaxKind::ExprReturn => {
                let range = child.text_range();
                out.push(range.start().into()..range.end().into());
            },
            _ => collect_returns(&child, out),
        }
    }
}
