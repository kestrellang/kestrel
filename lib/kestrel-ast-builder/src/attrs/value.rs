//! Typed attribute values: what consumers read instead of attribute text.

use kestrel_span::Span;

use crate::components::Os;

/// A parsed argument value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttrValue {
    /// Unescaped, no quotes.
    Str(String),
    Int(i128),
    /// As written; no attribute takes a float yet.
    Float(String),
    Bool(bool),
    /// `.Always`: the name, no dot.
    Case(String),
    /// `Equatable`, `std.fmt.Formattable`.
    Path(Vec<String>),
}

/// The calling convention of an `@extern` function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Abi {
    C,
}

/// `@inline`'s mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InlineMode {
    /// `@inline`: a hint.
    Hint,
    /// `@inline(.Always)`.
    Always,
    /// `@inline(.Never)`.
    Never,
}

/// `@doc`'s option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DocOption {
    /// `@doc(.Hidden)`: left out of docs and completion.
    Hidden,
}

/// `@layout`'s argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutOption {
    /// `@layout(.C)`: C's layout rules.
    C,
    /// `@layout(.Packed)`: every field at alignment 1, no padding.
    Packed,
}

/// Which use restriction a declaration carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RestrictionKind {
    /// `@deprecated`: every use warns (E908).
    Deprecated,
    /// `@unavailable`: every use is an error (E913).
    Unavailable,
}

/// `@deprecated(message, since:, replacement:)` or
/// `@unavailable(message, replacement:)`. One type, because the two share
/// targets, use-site walks and wording; only the severity differs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Restriction {
    pub kind: RestrictionKind,
    pub message: Option<String>,
    /// `@deprecated` only.
    pub since: Option<String>,
    pub replacement: Option<String>,
}

impl Restriction {
    /// "'X' is deprecated since 0.18: reason; use 'Y' instead", given the
    /// already-quoted name of the declaration.
    pub fn describe(&self, quoted_name: &str) -> String {
        let mut out = format!(
            "{quoted_name} is {}",
            match self.kind {
                RestrictionKind::Deprecated => "deprecated",
                RestrictionKind::Unavailable => "unavailable",
            }
        );
        if let Some(since) = &self.since {
            out.push_str(&format!(" since {since}"));
        }
        if let Some(message) = &self.message {
            out.push_str(&format!(": {message}"));
        }
        if let Some(replacement) = &self.replacement {
            out.push_str(&format!("; use '{replacement}' instead"));
        }
        out
    }
}

/// One well-formed attribute.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParsedAttr {
    Main,
    Extern {
        abi: Abi,
        mangle_name: Option<String>,
    },
    /// The builtin and its name as written (`.AddOperatorProtocol` and
    /// `Addable` are one builtin).
    Builtin(kestrel_hir::Builtin, String),
    FileConstant {
        path: String,
    },
    Platform(Vec<Os>),
    Inline(InlineMode),
    /// `@deprecated` or `@unavailable`.
    Restricted(Restriction),
    Doc(DocOption),
    NonExhaustive,
    /// `@align(N)`, already checked to be a power of two in range (E909).
    Align(u64),
    Layout(LayoutOption),
    Dummy,
}

/// A well-formed attribute and the span of its name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttrEntry {
    pub attr: ParsedAttr,
    pub span: Span,
}

/// One validation failure, ready to report. The code and wording are decided
/// here, next to the check, so the analyzer only forwards them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttrError {
    pub code: &'static str,
    pub message: String,
    pub label: String,
    pub span: Span,
    pub secondary: Option<(Span, String)>,
    pub notes: Vec<String>,
    /// Reported as a warning instead of an error.
    pub warning: bool,
}

impl AttrError {
    pub fn new(code: &'static str, message: String, label: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message,
            label: label.into(),
            span,
            secondary: None,
            notes: Vec::new(),
            warning: false,
        }
    }

    pub fn with_secondary(mut self, span: Span, label: impl Into<String>) -> Self {
        self.secondary = Some((span, label.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn as_warning(mut self) -> Self {
        self.warning = true;
        self
    }
}
