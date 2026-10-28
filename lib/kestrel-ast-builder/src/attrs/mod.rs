//! Attribute infrastructure: one registry, one parser, one query.
//!
//! - [`registry`] declares every attribute (targets, argument schema, phase,
//!   conflicts, docs). Nothing else lists attribute names.
//! - [`parse`] turns the raw [`Attributes`] component into typed
//!   [`ParsedAttr`] values plus [`AttrError`]s. It is the only code that reads
//!   attribute argument text.
//! - [`AttrsOf`] is the query consumers call. The `AttributeValidation`
//!   analyzer (kestrel-analyze) reports its errors; consumers read the typed
//!   values through the accessors on [`ParsedAttrs`].

pub mod parse;
pub mod registry;
pub mod target;
pub mod value;

use std::sync::Arc;

use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_span::Span;

pub use registry::{AttrDef, AttrPhase, REGISTRY, lookup};
pub use target::{AttrTarget, TargetSet, attr_target};
pub use value::{
    Abi, AttrEntry, AttrError, AttrValue, DocOption, InlineMode, LayoutOption, ParsedAttr,
    Restriction, RestrictionKind,
};

use crate::components::{Attributes, DeclSpan, NodeKind, Os, Valued};

/// Typed attributes of one declaration. A pure function of its `Attributes`
/// component (plus the declaration's kind, for target checks), so the
/// incremental engine invalidates it exactly when the attribute text changes.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AttrsOf {
    pub entity: Entity,
}

impl QueryFn for AttrsOf {
    type Output = Arc<ParsedAttrs>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Arc<ParsedAttrs> {
        let Some(attrs) = ctx.get::<Attributes>(self.entity) else {
            return Arc::new(ParsedAttrs::default());
        };
        let target = attr_target(ctx, self.entity);
        let has_body = has_body(ctx, self.entity);
        let decl_span = ctx
            .get::<DeclSpan>(self.entity)
            .map_or_else(|| Span::synthetic(0), |s| s.0.clone());
        let (parsed, errors) = parse::parse_attributes(&attrs.0, target, has_body, &decl_span);
        let mentioned = attrs
            .0
            .iter()
            .filter_map(|a| registry::lookup(&a.name).map(|d| d.name))
            .collect();
        Arc::new(ParsedAttrs {
            attrs: parsed,
            errors,
            mentioned,
        })
    }
}

/// Whether the declaration has code: its own body, or an accessor body
/// (a computed property's `set`, a subscript's `ref`).
fn has_body(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    ctx.has::<Valued>(entity)
        || ctx.children_of(entity).iter().any(|&c| {
            matches!(
                ctx.get::<NodeKind>(c),
                Some(NodeKind::Setter | NodeKind::RefAccessor)
            ) && ctx.has::<Valued>(c)
        })
}

/// Typed attributes of one declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ParsedAttrs {
    /// The well-formed ones, in source order.
    pub attrs: Vec<AttrEntry>,
    /// Every validation failure, with spans.
    pub errors: Vec<AttrError>,
    /// Every registered attribute written on the declaration, well-formed or
    /// not. Only for suppressing follow-on diagnostics (a malformed `@extern`
    /// still means "no body expected"); never for behaviour.
    mentioned: Vec<&'static str>,
}

impl ParsedAttrs {
    fn find<T>(&self, f: impl Fn(&ParsedAttr) -> Option<T>) -> Option<T> {
        self.attrs.iter().find_map(|e| f(&e.attr))
    }

    /// The span of the first attribute matching `f`.
    pub fn span_of(&self, f: impl Fn(&ParsedAttr) -> bool) -> Option<Span> {
        self.attrs
            .iter()
            .find(|e| f(&e.attr))
            .map(|e| e.span.clone())
    }

    /// Whether `@name` is written at all, even malformed. See `mentioned`.
    pub fn mentions(&self, name: &str) -> bool {
        self.mentioned.contains(&name)
    }

    pub fn is_main(&self) -> bool {
        self.find(|a| matches!(a, ParsedAttr::Main).then_some(()))
            .is_some()
    }

    /// `@extern(abi, mangleName:)`.
    pub fn extern_info(&self) -> Option<(Abi, Option<&str>)> {
        self.attrs.iter().find_map(|e| match &e.attr {
            ParsedAttr::Extern { abi, mangle_name } => Some((*abi, mangle_name.as_deref())),
            _ => None,
        })
    }

    pub fn is_extern(&self) -> bool {
        self.extern_info().is_some()
    }

    pub fn builtin(&self) -> Option<kestrel_hir::Builtin> {
        self.find(|a| match a {
            ParsedAttr::Builtin(b, _) => Some(*b),
            _ => None,
        })
    }

    /// `@builtin`'s name as written (no dot) and the attribute's span.
    pub fn builtin_written(&self) -> Option<(&str, Span)> {
        self.attrs.iter().find_map(|e| match &e.attr {
            ParsedAttr::Builtin(_, name) => Some((name.as_str(), e.span.clone())),
            _ => None,
        })
    }

    pub fn inline(&self) -> Option<InlineMode> {
        self.find(|a| match a {
            ParsedAttr::Inline(m) => Some(*m),
            _ => None,
        })
    }

    /// `@deprecated` or `@unavailable` written on this declaration itself.
    /// Consumers want [`UseRestrictionOf`], which also looks at the
    /// enclosing extension.
    pub fn restriction(&self) -> Option<&Restriction> {
        self.attrs.iter().find_map(|e| match &e.attr {
            ParsedAttr::Restricted(r) => Some(r),
            _ => None,
        })
    }

    /// `@doc(.Hidden)` written on this declaration itself. Consumers want
    /// [`is_doc_hidden`], which also looks at the enclosing extension.
    pub fn doc_hidden(&self) -> bool {
        self.attrs
            .iter()
            .any(|e| matches!(e.attr, ParsedAttr::Doc(DocOption::Hidden)))
    }

    pub fn is_non_exhaustive(&self) -> bool {
        self.attrs
            .iter()
            .any(|e| matches!(e.attr, ParsedAttr::NonExhaustive))
    }

    pub fn is_observable_drop(&self) -> bool {
        self.attrs
            .iter()
            .any(|e| matches!(e.attr, ParsedAttr::ObservableDrop))
    }

    /// The protocol paths of `@derive(...)`, as written.
    pub fn derives(&self) -> &[Vec<String>] {
        self.attrs
            .iter()
            .find_map(|e| match &e.attr {
                ParsedAttr::Derive(paths) => Some(paths.as_slice()),
                _ => None,
            })
            .unwrap_or(&[])
    }

    pub fn is_default_case(&self) -> bool {
        self.attrs
            .iter()
            .any(|e| matches!(e.attr, ParsedAttr::Default))
    }

    /// `@layout(...)`.
    pub fn layout(&self) -> Option<LayoutOption> {
        self.find(|a| match a {
            ParsedAttr::Layout(l) => Some(*l),
            _ => None,
        })
    }

    /// `@align(N)`.
    pub fn align(&self) -> Option<u64> {
        self.find(|a| match a {
            ParsedAttr::Align(n) => Some(*n),
            _ => None,
        })
    }

    pub fn file_constant(&self) -> Option<&str> {
        self.attrs.iter().find_map(|e| match &e.attr {
            ParsedAttr::FileConstant { path } => Some(path.as_str()),
            _ => None,
        })
    }

    /// Every `@platform(...)` list on the declaration.
    pub fn platforms(&self) -> impl Iterator<Item = &[Os]> {
        self.attrs.iter().filter_map(|e| match &e.attr {
            ParsedAttr::Platform(p) => Some(p.as_slice()),
            _ => None,
        })
    }
}

/// Shorthand for `ctx.query(AttrsOf { entity })`.
pub fn attrs_of(ctx: &QueryContext<'_>, entity: Entity) -> Arc<ParsedAttrs> {
    ctx.query(AttrsOf { entity })
}

/// The use restriction (`@deprecated` / `@unavailable`) that applies to
/// using `entity`: its own, or else the one on the `extend` that declares it
/// (an attribute on an extension applies to every member it declares).
/// Accessors answer for their property or subscript.
///
/// The single source of truth for "may this be used, and what do we say if
/// not": the use-site analyzers and the LSP both read it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct UseRestrictionOf {
    pub entity: Entity,
}

impl QueryFn for UseRestrictionOf {
    type Output = Option<Restriction>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Restriction> {
        let mut entity = self.entity;
        if matches!(
            ctx.get::<NodeKind>(entity),
            Some(NodeKind::Setter | NodeKind::RefAccessor)
        ) {
            entity = ctx.parent_of(entity)?;
        }
        if let Some(r) = attrs_of(ctx, entity).restriction() {
            return Some(r.clone());
        }
        let parent = ctx.parent_of(entity)?;
        if ctx.get::<NodeKind>(parent) != Some(&NodeKind::Extension) {
            return None;
        }
        attrs_of(ctx, parent).restriction().cloned()
    }
}

/// Whether docs and completion leave `entity` out: `@doc(.Hidden)` on it, or
/// on the `extend` that declares it. Not a visibility level: lookups and
/// access checks never read it.
pub fn is_doc_hidden(ctx: &QueryContext<'_>, entity: Entity) -> bool {
    if attrs_of(ctx, entity).doc_hidden() {
        return true;
    }
    ctx.parent_of(entity).is_some_and(|p| {
        ctx.get::<NodeKind>(p) == Some(&NodeKind::Extension) && attrs_of(ctx, p).doc_hidden()
    })
}
