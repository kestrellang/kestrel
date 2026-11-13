//! AST-level type representation.
//!
//! Pure data types extracted from the CST during build. Stored in
//! `TypeAnnotation` components; `kestrel-hir-lower` also builds them from
//! type syntax inside bodies before resolving them to `HirTy`.
//! All types carry a `Span` for error reporting.

use kestrel_span::Span;

/// Parameter passing convention carried on function types.
///
/// Defined here (the lowest language-fact crate that AST/HIR/type-infer all
/// depend on) so a `mutating` closure/function parameter's convention can ride
/// the type from parse through inference. Mirrors `kestrel_mir::ParamConvention`;
/// converted at the mir-lower boundary.
///
/// `Consuming` is the default for an un-annotated function-type parameter — this
/// preserves pre-#106 lowering (mir-lower previously hardcoded `Consuming` for
/// every function-type param). `MutBorrow` is introduced only by an explicit
/// `mutating` annotation or an inference upgrade from an expected type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ParamConvention {
    /// Read-only borrow.
    Borrow,
    /// Mutable (by-reference) borrow — the `mutating` convention.
    MutBorrow,
    /// Takes ownership. Default for un-annotated function-type params.
    #[default]
    Consuming,
}

/// The *kind* of a function type — the closure tier it names.
///
/// Spelled as an optional keyword prefix on a function type
/// (`escaping (Int64) -> ()`), never on a closure literal. Lives here next to
/// [`ParamConvention`] and for the same reason: it is the lowest crate that
/// AST / HIR / type-infer all depend on, so the kind can ride the type from
/// parse through inference.
///
/// The kind is a whole-type property and is orthogonal to the per-parameter
/// [`ParamConvention`]: `mutating (T) -> R` is a *mutating-kind* closure,
/// while `(mutating T) -> R` is a normal-kind closure taking `T` by mutable
/// borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FnTypeKind {
    /// `(T) -> R` — views of the frame, Copyable, frame-bound.
    #[default]
    Normal,
    /// `mutating (T) -> R` — `&mutating` views, not Copyable, exclusive calls.
    Mutating,
    /// `consuming (T) -> R` — owned captures, not Copyable, one-shot.
    Consuming,
    /// `escaping (T) -> R` — owned shared snapshot, Cloneable, may outlive the frame.
    Escaping,
    /// `@extern(.C) (T) -> R` — the address of a C-ABI function: no
    /// environment, Copyable, FFISafe when its params and result are.
    /// Only a named `@export(.C)` / `@extern(.C)` function makes one.
    C,
}

impl FnTypeKind {
    /// The **view tier**: the environment holds views into the enclosing frame
    /// and owns nothing, so the value is frame-bound (E494) and its captures
    /// are neither copied nor moved. The complement is the *owning* tier
    /// (`consuming`/`escaping`), whose environment owns its captures and may
    /// outlive the frame. Single source of truth for the tier split.
    pub fn is_view(self) -> bool {
        matches!(self, FnTypeKind::Normal | FnTypeKind::Mutating)
    }

    /// The source keyword, or `None` for the unmarked `Normal` kind.
    pub fn keyword(self) -> Option<&'static str> {
        match self {
            FnTypeKind::Normal => None,
            FnTypeKind::Mutating => Some("mutating"),
            FnTypeKind::Consuming => Some("consuming"),
            FnTypeKind::Escaping => Some("escaping"),
            FnTypeKind::C => None,
        }
    }

    /// Rendering prefix — `""` for `Normal`, `"<keyword> "` otherwise.
    /// Single source of truth for every function-type renderer.
    pub fn prefix(self) -> &'static str {
        match self {
            FnTypeKind::Normal => "",
            FnTypeKind::Mutating => "mutating ",
            FnTypeKind::Consuming => "consuming ",
            FnTypeKind::Escaping => "escaping ",
            FnTypeKind::C => "@extern(.C) ",
        }
    }
}

/// A single segment in a qualified type path.
/// Each segment has a name and optional type arguments.
/// e.g. in `Array[Int].Iterator`, `Array[Int]` and `Iterator` are segments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PathSegment {
    pub name: String,
    pub type_args: Vec<AstType>,
    pub span: Span,
}

/// One protocol bound of an `any` type, with the associated types it pins:
/// `Iterator[Item = Int64]` is `protocol: Iterator`, `bindings: [Item = Int64]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnyBound {
    pub protocol: AstType,
    pub bindings: Vec<AssocTypeBinding>,
}

/// `Name = Type` pinning an associated type in an `any` bound.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssocTypeBinding {
    pub name: String,
    pub ty: AstType,
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
    /// Function type, e.g. `(Int) -> String` or `(mutating Grid) -> Unit`.
    /// `param_conventions` is parallel to `params` (same length); a
    /// `mutating` prefix on a param yields `MutBorrow`, otherwise `Consuming`.
    /// `kind` is the whole-type closure tier from an optional keyword prefix
    /// (`escaping (Int) -> String`); it is orthogonal to `param_conventions`.
    /// `span` covers the keyword when one is present.
    Function {
        kind: FnTypeKind,
        params: Vec<AstType>,
        param_conventions: Vec<ParamConvention>,
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
    /// Opaque type, e.g. `some P`, `some P and Q`, `some P and not Copyable`.
    /// `negative` holds the trailing `not <path>` bounds; only the implicit
    /// builtins (`Copyable`, `Static`, `Sized`) are legal there — validated
    /// at HIR lowering, where the paths resolve.
    Some {
        bounds: Vec<AstType>,
        negative: Vec<AstType>,
        span: Span,
    },
    /// Boxed existential, e.g. `any P`, `any[UniqueBox] P and Q`,
    /// `any Iterator[Item = Int64]`. Sugar for `Box[<raw existential>]`:
    /// `box_ty` is the unapplied box (`None` = the builtin default box), and
    /// HIR lowering does the desugaring.
    Any {
        box_ty: Option<Box<AstType>>,
        bounds: Vec<AnyBound>,
        span: Span,
    },
    /// Reference type, e.g. `&T` or `&mutating T`. Parses in every type
    /// position but is accepted in none (stage 0.5) — every occurrence is
    /// rejected at HIR lowering, so no `Ref` survives past it.
    Ref {
        inner: Box<AstType>,
        mutating: bool,
        span: Span,
    },
    /// A lang item named by its `@builtin` tag rather than a path. Never
    /// parsed — only compiler-generated code builds it (`@derive`'s
    /// `T: Copyable` field bounds), so no user declaration can shadow it.
    /// Resolves through `ResolveBuiltin`.
    Builtin(crate::Builtin, Span),
}

impl AstType {
    /// The source span of the whole annotation.
    pub fn span(&self) -> Span {
        match self {
            AstType::Named { span, .. }
            | AstType::Tuple(_, span)
            | AstType::Function { span, .. }
            | AstType::Array(_, span)
            | AstType::Dictionary(_, _, span)
            | AstType::Optional(_, span)
            | AstType::Result { span, .. }
            | AstType::Unit(span)
            | AstType::Never(span)
            | AstType::Inferred(span)
            | AstType::Some { span, .. }
            | AstType::Any { span, .. }
            | AstType::Ref { span, .. }
            | AstType::Builtin(_, span) => span.clone(),
        }
    }

    /// A param type that spells its access mode (`&T` / `&mutating T`):
    /// the pointee and whether it is mutating. A nested ref (`&&T`) is not
    /// one — it stays, and is rejected as nesting (E487).
    pub fn param_ref(&self) -> Option<(&AstType, bool)> {
        match self {
            AstType::Ref {
                inner, mutating, ..
            } if !matches!(**inner, AstType::Ref { .. }) => Some((inner, *mutating)),
            _ => None,
        }
    }

    /// The types written inside an `any` type, in source order: the box
    /// (if named), each bound protocol, and each pinned associated type.
    /// Empty for every other variant. Shared by the AST walkers so none of
    /// them forgets a part.
    pub fn any_children(&self) -> Vec<&AstType> {
        let AstType::Any { box_ty, bounds, .. } = self else {
            return Vec::new();
        };
        let mut out: Vec<&AstType> = box_ty.iter().map(|b| b.as_ref()).collect();
        for bound in bounds {
            out.push(&bound.protocol);
            out.extend(bound.bindings.iter().map(|b| &b.ty));
        }
        out
    }
}
