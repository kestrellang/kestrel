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
    /// Read only by tools: `kestrel-doc` and the LSP (`@doc`).
    Tooling,
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

/// Everything a program can name and use: every declaration except a
/// `deinit`, which nothing calls. `@deprecated` and `@unavailable` apply to
/// these, and to an `extend` (meaning every member it declares).
const USABLE: TargetSet = TargetSet::ALL.without(T::Deinit);

/// `@deprecated`'s and `@unavailable`'s `replacement:` parameter.
const REPLACEMENT: ParamDef = labeled("replacement", ParamKind::Str, "the name to use instead");

fn is_builtin_name(name: &str) -> bool {
    kestrel_hir::Builtin::from_attribute_name(name).is_some()
}

/// The registry. Sorted by name only for readability; lookups are linear
/// over a couple dozen entries.
pub static REGISTRY: &[AttrDef] = &[
    AttrDef {
        name: "align",
        targets: TargetSet::of(&[T::Struct, T::Enum]),
        params: &[positional(
            ParamKind::Int,
            true,
            "an alignment: '@align(16)'",
        )],
        repeatable: false,
        phase: AttrPhase::Lowering,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Raises the type's alignment to at least N bytes, a power of two from 1 to 4096; the size is rounded up to match. Never lowers it.",
    },
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
        name: "default",
        targets: TargetSet::of(&[T::EnumCase]),
        params: &[],
        repeatable: false,
        phase: AttrPhase::Build,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Marks the case a derived 'Defaultable' enum starts as: '@derive(Defaultable)' on an enum needs exactly one '@default' case.",
    },
    AttrDef {
        name: "deprecated",
        targets: USABLE,
        params: &[
            positional(
                ParamKind::Str,
                false,
                "a message: '@deprecated(\"use 'parse' instead\")'",
            ),
            labeled("since", ParamKind::Str, "a version"),
            REPLACEMENT,
        ],
        repeatable: false,
        phase: AttrPhase::Semantic,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Warns at every use of the declaration (E908). '@deprecated(\"message\", since: \"0.18\", replacement: \"newName\")'; all arguments are optional.",
    },
    AttrDef {
        name: "derive",
        targets: TargetSet::of(&[T::Struct, T::Enum, T::Extension]),
        params: &[ParamDef {
            label: None,
            kind: ParamKind::Path,
            required: true,
            variadic: true,
            missing_code: None,
            what: "a protocol to derive: '@derive(Equatable)'",
        }],
        // One list per declaration, so the conformances it adds read in
        // one place.
        repeatable: false,
        phase: AttrPhase::Build,
        conflicts: &[],
        needs_body: false,
        repeat_hint: Some("list every protocol in one '@derive(...)'"),
        doc: "Has the compiler write the conformance: '@derive(Equatable, Hashable)'. Derivable: Equatable, Hashable, Comparable, Formattable, Defaultable. On a generic type, each type parameter used by a field must conform too.",
    },
    AttrDef {
        name: "doc",
        targets: USABLE,
        params: &[positional(
            ParamKind::Cases("doc option", &["Hidden"]),
            true,
            "a doc option: '@doc(.Hidden)'",
        )],
        repeatable: false,
        phase: AttrPhase::Tooling,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "'@doc(.Hidden)' leaves a public declaration out of generated documentation and editor completion. It stays usable; on an extension it hides every member the extension declares.",
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
        name: "layout",
        targets: TargetSet::of(&[T::Struct, T::Enum]),
        params: &[positional(
            ParamKind::Cases("layout", &["C", "Packed"]),
            true,
            "a layout: '@layout(.C)'",
        )],
        repeatable: false,
        phase: AttrPhase::Lowering,
        // `.Packed` with `@align` is E906 too, but `.C` with `@align` is
        // fine, so that pair is checked on the parsed values (`parse`).
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Fixes the type's memory layout. '.C' follows C's rules (on a payload-less enum, an Int32 tag); '.Packed' places every field at alignment 1 with no padding. Without it the layout is unspecified.",
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
        name: "nonExhaustive",
        targets: TargetSet::of(&[T::Enum]),
        params: &[],
        repeatable: false,
        phase: AttrPhase::Semantic,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Lets a public enum gain cases without a breaking release: a 'match' on it outside its module must have a catch-all arm.",
    },
    AttrDef {
        name: "observabledrop",
        targets: TargetSet::of(&[T::Struct, T::Enum]),
        params: &[],
        repeatable: false,
        phase: AttrPhase::Lowering,
        conflicts: &[],
        needs_body: false,
        repeat_hint: None,
        doc: "Dropping this type has an observable effect (closing a socket, say), so the copy lifetime passes keep every copy's drop at the end of its scope instead of dropping it early.",
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
    AttrDef {
        name: "unavailable",
        targets: USABLE,
        params: &[
            positional(
                ParamKind::Str,
                false,
                "a message: '@unavailable(\"removed in 0.18\")'",
            ),
            REPLACEMENT,
        ],
        repeatable: false,
        phase: AttrPhase::Semantic,
        conflicts: &[
            Conflict {
                with: "deprecated",
                reason: "an unavailable declaration cannot be used at all",
            },
            Conflict {
                with: "main",
                reason: "the entry point must be usable",
            },
        ],
        needs_body: false,
        repeat_hint: None,
        doc: "Makes every use of the declaration an error (E913), for APIs that were removed or cannot work yet. '@unavailable(\"message\", replacement: \"newName\")'; both arguments are optional.",
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
