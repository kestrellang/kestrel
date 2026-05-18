//! Diagnostic types for the lib compiler pipeline.
//!
//! All phases (lex, parse, type inference) define error types that implement
//! `ToDiagnostic`. Queries use `throw()` to convert and accumulate them
//! as `codespan_reporting::Diagnostic<usize>` in the HECS query system.
//!
//! `WorldFiles` bridges the ECS world to codespan-reporting's `Files` trait
//! for terminal emission.

use std::collections::HashMap;
use std::ops::Range;

use codespan_reporting::files::{self, SimpleFile};
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_reporting::{Diagnostic, Label, ToDiagnostic};
use kestrel_span::Span;
use kestrel_type_infer::error::InferError;

use crate::components::{FilePath, SourceText};

/// Extension trait for reporting diagnostics during query execution.
///
/// Import this trait to call `ctx.throw(error)` on a `QueryContext`.
/// The error must implement `ToDiagnostic` — this ensures all diagnostics
/// go through consistent formatting before accumulation.
pub trait ThrowDiagnostic {
    fn throw(&self, error: impl ToDiagnostic);
}

impl ThrowDiagnostic for QueryContext<'_> {
    fn throw(&self, error: impl ToDiagnostic) {
        self.accumulate(error.to_diagnostic());
    }
}

// ===== Lex errors =====

/// A lexer error — an unexpected character at a source location.
pub struct LexError {
    pub span: Span,
}

impl ToDiagnostic for LexError {
    fn to_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("unexpected character")
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

// ===== Parse errors =====

/// A parser error with message and source location.
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ToDiagnostic for ParseError {
    fn to_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(self.message.clone())
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())])
    }
}

// ===== Type inference errors =====

/// A resolved type inference error — pairs the raw `InferError` with
/// a human-readable detail string (resolved type names).
pub struct ResolvedInferError<'a> {
    pub error: &'a InferError,
    pub detail: &'a str,
}

impl ToDiagnostic for ResolvedInferError<'_> {
    fn to_diagnostic(&self) -> Diagnostic<usize> {
        let span = self.error.span();
        let file_id = span.file_id;
        let range = span.range();
        let detail = self.detail;

        match self.error {
            InferError::TypeMismatch { .. } => Diagnostic::error()
                .with_message("type mismatch")
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::DoesNotConform { .. } => Diagnostic::error()
                .with_message(
                    "type mismatch: does not conform to protocol; does not satisfy constraint",
                )
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::NoMember { .. } => Diagnostic::error()
                // `detail` already includes the lib1-style wording
                // ("no method 'X' on type 'Y'" or "no member ..."), so we use it
                // directly instead of prepending a redundant prefix.
                .with_message(detail)
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::AmbiguousMember { name, .. } => Diagnostic::error()
                .with_message(format!("ambiguous member '{name}'"))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::MemberNotVisible {
                name, visibility, ..
            } => {
                let vis = match visibility {
                    kestrel_ast_builder::Vis::Public => "public",
                    kestrel_ast_builder::Vis::Internal => "internal",
                    kestrel_ast_builder::Vis::Fileprivate => "fileprivate",
                    kestrel_ast_builder::Vis::Private => "private",
                };
                Diagnostic::error()
                    .with_message(format!(
                        "member '{name}' is {vis} and not accessible from this scope"
                    ))
                    .with_labels(vec![Label::primary(file_id, range).with_message(detail)])
            },

            InferError::NoAssociatedType { name, .. } => Diagnostic::error()
                .with_message(format!("no associated type '{name}'"))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::InfiniteType { .. } => Diagnostic::error()
                .with_message("infinite type")
                .with_labels(vec![
                    Label::primary(file_id, range).with_message("recursive type detected"),
                ]),

            InferError::FromHir { .. } => Diagnostic::error()
                .with_message("error in expression")
                .with_labels(vec![Label::primary(file_id, range)]),

            InferError::ImplicitMemberNotFound { name, .. } => Diagnostic::error()
                .with_message(format!("implicit member '.{name}' not found"))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::ArgCountMismatch { expected, got, .. } => Diagnostic::error()
                .with_message(format!(
                    "wrong number of arguments: expected {expected}, got {got}"
                ))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::LabelMismatch { .. } => Diagnostic::error()
                .with_message("wrong argument label")
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::InstanceMethodAsStatic { name, .. } => Diagnostic::error()
                .with_message(format!(
                    "instance method '{name}' cannot be called on a type"
                ))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::TypeParamAsValue { .. } => Diagnostic::error()
                .with_message("type parameter cannot be used as a value")
                .with_labels(vec![
                    Label::primary(file_id, range).with_message("not a value"),
                ]),

            InferError::TypeArgCountMismatch { expected, got, .. } => {
                let msg = if *got < *expected {
                    format!("too few type arguments: expected {expected}, got {got}")
                } else {
                    format!("too many type arguments: expected {expected}, got {got}")
                };
                Diagnostic::error()
                    .with_message(&msg)
                    .with_labels(vec![Label::primary(file_id, range).with_message(detail)])
            },

            InferError::NoMatchingOverload { name, .. } => Diagnostic::error()
                .with_message(format!("no matching overload for '{name}'"))
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::MemberwiseInitArity {
                struct_name,
                expected,
                got,
                ..
            } => Diagnostic::error()
                .with_message(format!(
                    "struct '{struct_name}' has {expected} field(s), but {got} argument(s) were provided"
                ))
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message(format!("expected {expected} argument(s)")),
                ]),

            InferError::MemberwiseInitLabel {
                struct_name,
                expected,
                got,
                ..
            } => {
                let got_desc = got
                    .as_deref()
                    .map(|s| format!("'{}'", s))
                    .unwrap_or_else(|| "unlabeled".into());
                Diagnostic::error()
                    .with_message(format!(
                        "argument for struct '{struct_name}' has {got_desc} label, but expected '{expected}'"
                    ))
                    .with_labels(vec![
                        Label::primary(file_id, range)
                            .with_message(format!("expected label '{expected}'")),
                    ])
            },

            InferError::ItWrongArity { expected, .. } => Diagnostic::error()
                .with_message("implicit 'it' parameter requires single-parameter context")
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message(format!("expected {} parameter(s)", expected)),
                ]),

            InferError::LiteralNotAccepted { .. } => Diagnostic::error()
                .with_message("type mismatch: does not conform to protocol")
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)]),

            InferError::UnresolvedTypeParam { .. } => Diagnostic::error()
                .with_message("cannot infer type parameter")
                .with_labels(vec![Label::primary(file_id, range).with_message(detail)])
                .with_notes(vec![
                    "no argument or context constrains this type parameter; \
                     annotate it explicitly at the call (e.g. `f[_, Int64](...)`) \
                     or at the binding (e.g. `let x: T = f(...)`)"
                        .into(),
                ]),

            InferError::CannotInferType { .. } => Diagnostic::error()
                .with_message("could not infer type")
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message("add a type annotation to resolve this"),
                ]),

            InferError::TupleIndexOnNonTuple { index, .. } => Diagnostic::error()
                .with_message(format!("cannot index into non-tuple type: {detail}"))
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message(format!("'.{index}' requires a tuple receiver")),
                ]),

            InferError::TupleIndexOutOfBounds { arity, index, .. } => Diagnostic::error()
                .with_message(format!(
                    "tuple index {index} out of bounds for {arity}-element tuple"
                ))
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message(format!("valid indices are 0..{}", arity.saturating_sub(1))),
                ]),

            InferError::MemberAccessOnPrimitive { name, .. } => Diagnostic::error()
                .with_message(format!("cannot access member on type: {detail}"))
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message(format!("'{name}' not available")),
                ]),

            InferError::PrimitiveMethodNotCalled { method, .. } => Diagnostic::error()
                .with_message(detail.to_string())
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message("add () to call this method".to_string()),
                ])
                .with_notes(vec![format!(
                    "primitive methods cannot be used as first-class values; use '.{method}()' instead"
                )]),

            InferError::CircularOpaqueReturn { .. } => Diagnostic::error()
                .with_message("circular opaque return type")
                .with_labels(vec![
                    Label::primary(file_id, range)
                        .with_message("concrete type cannot be determined"),
                ])
                .with_notes(vec![
                    "mutually recursive functions with 'some' return types must have at least one non-opaque base case".into(),
                ]),
        }
    }
}

// ===== File provider =====

/// File provider backed by the ECS world.
///
/// Snapshots file names and sources from entities, indexed by entity index.
/// Implements `codespan_reporting::files::Files` so diagnostics can be
/// rendered with source context.
pub struct WorldFiles {
    files: HashMap<usize, SimpleFile<String, String>>,
}

impl WorldFiles {
    /// Build from a World by extracting all entities that have SourceText.
    pub fn from_world(world: &World, file_entities: &HashMap<String, Entity>) -> Self {
        let mut files = HashMap::new();
        for (path, &entity) in file_entities {
            if let Some(source) = world.get::<SourceText>(entity) {
                let name = world
                    .get::<FilePath>(entity)
                    .map(|fp| fp.0.clone())
                    .unwrap_or_else(|| path.clone());
                files.insert(entity.index(), SimpleFile::new(name, source.0.clone()));
            }
        }
        Self { files }
    }
}

impl<'a> files::Files<'a> for WorldFiles {
    type FileId = usize;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: usize) -> Result<&'a str, files::Error> {
        self.files
            .get(&id)
            .map(|f| f.name().as_str())
            .ok_or(files::Error::FileMissing)
    }

    fn source(&'a self, id: usize) -> Result<&'a str, files::Error> {
        self.files
            .get(&id)
            .map(|f| f.source().as_str())
            .ok_or(files::Error::FileMissing)
    }

    fn line_index(&'a self, id: usize, byte_index: usize) -> Result<usize, files::Error> {
        self.files
            .get(&id)
            .ok_or(files::Error::FileMissing)?
            .line_index((), byte_index)
    }

    fn line_range(&'a self, id: usize, line_index: usize) -> Result<Range<usize>, files::Error> {
        self.files
            .get(&id)
            .ok_or(files::Error::FileMissing)?
            .line_range((), line_index)
    }
}
