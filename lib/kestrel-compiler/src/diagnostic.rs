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

/// A parser error with its `E8xx` code, message and source location.
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub code: Option<&'static str>,
}

impl ToDiagnostic for ParseError {
    fn to_diagnostic(&self) -> Diagnostic<usize> {
        let mut diag = Diagnostic::error()
            .with_message(self.message.clone())
            .with_labels(vec![Label::primary(self.span.file_id, self.span.range())]);
        if let Some(code) = self.code {
            diag = diag.with_code(code);
        }
        diag
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
    /// Thin wrapper over `InferError::render` — the single owner of every
    /// inference error's code, message, label and notes (F15). Do not add
    /// per-variant wording here; add the arm in `kestrel-type-infer`.
    fn to_diagnostic(&self) -> Diagnostic<usize> {
        let span = self.error.span();
        let r = self.error.render(self.detail);

        let mut label = Label::primary(span.file_id, span.range());
        if let Some(text) = r.label {
            label = label.with_message(text);
        }

        Diagnostic::error()
            .with_code(r.code)
            .with_message(r.message)
            .with_labels(vec![label])
            .with_notes(r.notes)
    }
}

// ===== MIR diagnostics =====

/// Resolve a span for a diagnostic: prefer the provided span, fall back to
/// the entity's DeclSpan from the World, then to a synthetic span.
fn resolve_span(span: Option<&Span>, entity: Entity, world: &World) -> Span {
    if let Some(s) = span {
        return s.clone();
    }
    world
        .get::<kestrel_ast_builder::DeclSpan>(entity)
        .map(|s| s.0.clone())
        .unwrap_or_else(|| Span::synthetic(0))
}

pub fn mir_verify_error_to_diagnostic(
    error: &kestrel_mir::verify::VerifyError,
    world: &World,
) -> Diagnostic<usize> {
    let span = resolve_span(error.span.as_ref(), error.entity, world);

    // Coded errors (escape check E494-E496) are user diagnostics, not ICEs.
    if let Some(diag) = &error.diag {
        let mut labels = vec![Label::primary(span.file_id, span.range())];
        if let Some((sec_span, sec_msg)) = &diag.secondary {
            labels.push(Label::secondary(sec_span.file_id, sec_span.range()).with_message(sec_msg));
        }
        return Diagnostic::error()
            .with_code(diag.code)
            .with_message(&error.message)
            .with_labels(labels)
            .with_notes(diag.notes.clone());
    }

    let location = match error.inst {
        Some(i) => format!(" at bb{}[{}]", error.block.index(), i),
        None => format!(" at bb{}", error.block.index()),
    };

    Diagnostic::bug()
        .with_message(format!(
            "internal compiler error: OSSA verify failed in '{}'{}: {}",
            error.func_name, location, error.message
        ))
        .with_labels(vec![
            Label::primary(span.file_id, span.range()).with_message(&error.message),
        ])
        .with_notes(vec![
            "this is an internal compiler error; please file a bug report".into(),
        ])
}

pub fn mir_mono_verify_error_to_diagnostic(
    error: &kestrel_mir::mono::verify::MonoVerifyError,
    module: &kestrel_mir::mono::MonoModule,
    world: &World,
) -> Diagnostic<usize> {
    let func = &module.functions[error.func_idx];
    let span = resolve_span(error.span.as_ref(), func.source, world);

    // A user-facing verify error is a front-end gap surfaced late (e.g. an
    // unresolved conformance witness) — render it as a plain build error, not
    // an "internal compiler error / file a bug report" ICE.
    if error.user_facing {
        return Diagnostic::error()
            .with_message(&error.message)
            .with_labels(vec![
                Label::primary(span.file_id, span.range()).with_message(&error.message),
            ]);
    }

    let location = match (error.block, error.inst) {
        (Some(b), Some(i)) => format!(" at bb{}[{}]", b.index(), i),
        (Some(b), None) => format!(" at bb{}", b.index()),
        _ => String::new(),
    };

    Diagnostic::bug()
        .with_message(format!(
            "internal compiler error: post-mono verify failed in '{}'{}: {}",
            func.name, location, error.message
        ))
        .with_labels(vec![
            Label::primary(span.file_id, span.range()).with_message(&error.message),
        ])
        .with_notes(vec![
            "this is an internal compiler error; please file a bug report".into(),
        ])
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
