//! The attribute registry: the **only** place an attribute is declared.
//!
//! Every other part of the compiler reads this table: the unknown-attribute
//! check ("not in the registry"), argument parsing and validation
//! (`super::parse`), target checks (`super::target`), duplicate and conflict
//! checks, and LSP completion/hover (`doc`).
//!
//! The shape (named, typed, labelled parameters) is chosen so a future
//! Kestrel `attribute name(params)` declaration maps onto it one to one.

use super::target::{AttrTarget, TargetSet};

/// One attribute's declaration.
#[derive(Debug)]
pub struct AttrDef {
    pub name: &'static str,
    /// Where it may appear.
    pub targets: TargetSet,
    /// Argument schema: positional parameters first, then labelled ones.
    pub params: &'static [ParamDef],
    /// May appear more than once on one declaration.
    pub repeatable: bool,
    /// When it takes effect, which decides which crate may read it.
    pub phase: AttrPhase,
    /// Attributes it cannot be combined with on the same declaration.
    pub conflicts: &'static [Conflict],
    /// Needs the declaration to have a body (E905), e.g. `@inline`.
    pub needs_body: bool,
    /// Extra advice appended to the "appears twice" error (E901).
    pub repeat_hint: Option<&'static str>,
    /// LSP hover and completion text.
    pub doc: &'static str,
}

/// A conflicting attribute and why the pair is rejected.
#[derive(Debug)]
pub struct Conflict {
    pub with: &'static str,
    pub reason: &'static str,
}

/// One parameter of an attribute.
#[derive(Debug)]
pub struct ParamDef {
    /// `None` = positional.
    pub label: Option<&'static str>,
    pub kind: ParamKind,
    pub required: bool,
    /// Positional only, and only the last positional parameter. Takes every
    /// remaining positional argument.
    pub variadic: bool,
    /// Diagnostic code for a missing required argument, when it predates the
    /// E9xx band (E612 for `@extern`). `None` uses E902.
    pub missing_code: Option<&'static str>,
    /// What the argument is, for "needs ..." messages ("a path").
    pub what: &'static str,
}

/// The kind of value a parameter accepts.
#[derive(Debug)]
pub enum ParamKind {
    Str,
    Int,
    Bool,
    /// A closed set of implicit-member cases, matched by name (#111). The
    /// `&str` is the noun used in errors ("inline mode").
    Cases(&'static str, &'static [&'static str]),
    /// A set owned elsewhere, e.g. `Builtin::from_attribute_name`.
    OpenCase(&'static str, fn(&str) -> bool),
    /// A (possibly dotted) path, resolved by name resolution later.
    Path,
    /// Anything (`@dummy` only).
    Any,
}

/// When an attribute takes effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrPhase {
    /// Read by the AST builder while building (`@platform`).
    Build,
    /// Read by name resolution.
    NameRes,
    /// Read by analysis and type inference.
    Semantic,
    /// Read by MIR lowering, which turns it into a typed MIR field.
    Lowering,
    /// No effect (`@dummy`).
    None,
}

const fn positional(kind: ParamKind, required: bool, what: &'static str) -> ParamDef {
    ParamDef {
        label: None,
        kind,
        required,
        variadic: false,
        missing_code: None,
        what,
    }
}

const fn labeled(label: &'static str, kind: ParamKind, what: &'static str) -> ParamDef {
    ParamDef {
        label: Some(label),
        kind,
        required: false,
        variadic: false,
        missing_code: None,
        what,
    }
}

use AttrTarget as T;

/// Every named declaration (the targets of `@builtin`, `@dummy`, ...).
const ANY_DECL: TargetSet = TargetSet::ALL;

fn is_builtin_name(name: &str) -> bool {
    kestrel_hir::Builtin::from_attribute_name(name).is_some()
}

/// The registry. Sorted by name only for readability; lookups are linear
/// over a couple dozen entries.
pub static REGISTRY: &[AttrDef] = &[
    AttrDef {
        name: "builtin",
        targets: ANY_DECL,
        params: &[positional(
            ParamKind::OpenCase("builtin", is_builtin_name),
            true,
            "a builtin name: '@builtin(.Copyable)'",
        )],
        repeatable: false,
        phase: AttrPhase::NameRes,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Marks a standard-library declaration as a language item the compiler refers to.",
    },
    AttrDef {
        name: "dummy",
        targets: ANY_DECL,
        params: &[ParamDef {
            label: None,
            kind: ParamKind::Any,
            required: false,
            variadic: true,
            missing_code: None,
            what: "",
        }],
        repeatable: true,
        phase: AttrPhase::None,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "A placeholder with no effect, used by parser tests.",
    },
    AttrDef {
        name: "extern",
        targets: TargetSet::of(&[T::FreeFunction]),
        params: &[
            ParamDef {
                label: None,
                kind: ParamKind::Cases("calling convention", &["C"]),
                required: true,
                variadic: false,
                missing_code: Some("E612"),
                what: "a calling convention: '@extern(.C)'",
            },
            labeled("mangleName", ParamKind::Str, "a symbol name"),
        ],
        repeatable: false,
        phase: AttrPhase::Lowering,
        conflicts: &[Conflict {
            with: "main",
            reason: "the entry point must be a Kestrel function",
        }],
        needs_body: false,
        repeat_hint: None,
        doc: "Declares a function implemented outside Kestrel, called with the given calling convention.",
    },
    AttrDef {
        name: "fileconstant",
        targets: TargetSet::of(&[T::ModuleValue]),
        params: &[positional(
            ParamKind::Str,
            true,
            "a path: '@fileconstant(\"data/table.bin\")'",
        )],
        repeatable: false,
        phase: AttrPhase::Lowering,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Embeds the contents of a file, relative to the source file, as the value of a module-level constant.",
    },
    AttrDef {
        name: "inline",
        targets: TargetSet::of(&[
            T::FreeFunction,
            T::Method,
            T::Requirement,
            T::Initializer,
            T::Deinit,
            T::Subscript,
            T::ComputedProperty,
        ]),
        params: &[positional(
            ParamKind::Cases("inline mode", &["Always", "Never"]),
            false,
            "an inline mode: '@inline(.Always)'",
        )],
        repeatable: false,
        phase: AttrPhase::Lowering,
        conflicts: &[Conflict {
            with: "extern",
            reason: "an extern function has no body to inline",
        }],
        // A bodiless requirement is a target so the error says "needs a
        // body" (E905) rather than "not allowed here".
        needs_body: true,
        repeat_hint: None,
        doc: "Asks the optimizer to inline the function: '@inline' is a hint, '@inline(.Always)' and '@inline(.Never)' are directives. Has no effect in the Cranelift backend.",
    },
    AttrDef {
        name: "main",
        targets: ANY_DECL,
        params: &[],
        repeatable: false,
        phase: AttrPhase::Semantic,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Marks the program's entry point.",
    },
    AttrDef {
        name: "platform",
        targets: ANY_DECL,
        // `.darwin`/`.linux` are the pre-0.18 spellings, accepted with a
        // warning (E914) for one release.
        params: &[ParamDef {
            label: None,
            kind: ParamKind::Cases("platform", &["Darwin", "Linux", "darwin", "linux"]),
            required: true,
            variadic: true,
            missing_code: None,
            what: "a platform: '@platform(.Linux)'",
        }],
        // Not repeatable: the argument list means *any of*, and two
        // `@platform`s ("all of") could never both match.
        repeatable: false,
        phase: AttrPhase::Build,
        conflicts: &[],
        needs_body: false,
        repeat_hint: Some("list every platform in one '@platform(...)'"),
        doc: "Compiles the declaration only when building for one of the given platforms: '@platform(.Linux, .Darwin)'.",
    },
];

/// The registry entry for `name`.
pub fn lookup(name: &str) -> Option<&'static AttrDef> {
    REGISTRY.iter().find(|d| d.name == name)
}

/// Every registered attribute name, for "did you mean" suggestions and LSP
/// completion.
pub fn names() -> impl Iterator<Item = &'static str> {
    REGISTRY.iter().map(|d| d.name)
}
