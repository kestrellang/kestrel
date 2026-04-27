//! Convert compiler diagnostics into LSP `Diagnostic`s.
//!
//! Two input shapes feed into the LSP:
//!
//! * `codespan_reporting::Diagnostic<usize>` — accumulated by lex / parse /
//!   type inference. The `usize` file id is `Entity::index()`.
//! * `kestrel_analyze::AnalyzeDiagnostic` — accumulated by the analyzer crate.
//!
//! Both carry byte-offset spans tagged by entity index. We resolve those to
//! document `Url` + `LineIndex` through the per-document map maintained by
//! the server.

use std::collections::HashMap;

use codespan_reporting::diagnostic::{
    Diagnostic as CsDiagnostic, LabelStyle, Severity as CsSeverity,
};
use kestrel_analyze::{AnalyzeDiagnostic, DiagLabel, Severity as AnalyzeSeverity};
use kestrel_span::Span;
use tower_lsp::lsp_types::{
    DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Range, Url,
};

use crate::position::LineIndex;

/// Resolve a file_id (entity index) → Url + line index. Built once per
/// publish pass from the open-doc map plus any project files we've loaded.
pub struct FileMap<'a> {
    pub by_id: HashMap<usize, (Url, &'a LineIndex)>,
}

impl<'a> FileMap<'a> {
    pub fn lookup(&self, file_id: usize) -> Option<(&Url, &LineIndex)> {
        self.by_id.get(&file_id).map(|(u, i)| (u, *i))
    }

    fn span_to_location(&self, span: &Span) -> Option<Location> {
        let (uri, idx) = self.lookup(span.file_id)?;
        Some(Location {
            uri: uri.clone(),
            range: idx.range_for(span.start, span.end),
        })
    }
}

/// Convert a codespan diagnostic. Returns `(file_id, lsp_diag)` for the
/// primary label's file. Labels in other files become `relatedInformation`.
pub fn from_codespan(
    diag: &CsDiagnostic<usize>,
    files: &FileMap<'_>,
) -> Option<(usize, tower_lsp::lsp_types::Diagnostic)> {
    let primary = diag
        .labels
        .iter()
        .find(|l| l.style == LabelStyle::Primary)
        .or_else(|| diag.labels.first())?;
    let (uri, idx) = files.lookup(primary.file_id)?;
    let range = idx.range_for(primary.range.start, primary.range.end);

    let related: Vec<DiagnosticRelatedInformation> = diag
        .labels
        .iter()
        .filter(|l| !std::ptr::eq(*l, primary))
        .filter_map(|l| {
            let (luri, lidx) = files.lookup(l.file_id)?;
            Some(DiagnosticRelatedInformation {
                location: Location {
                    uri: luri.clone(),
                    range: lidx.range_for(l.range.start, l.range.end),
                },
                message: if l.message.is_empty() {
                    diag.message.clone()
                } else {
                    l.message.clone()
                },
            })
        })
        .collect();

    let message = build_message(&diag.message, &primary.message, &diag.notes);
    let _ = uri; // Url consumed via FileMap; keep the lookup result alive
    Some((
        primary.file_id,
        tower_lsp::lsp_types::Diagnostic {
            range,
            severity: Some(severity_from_codespan(diag.severity)),
            code: diag.code.clone().map(NumberOrString::String),
            code_description: None,
            source: Some("kestrel".into()),
            message,
            related_information: if related.is_empty() {
                None
            } else {
                Some(related)
            },
            tags: None,
            data: None,
        },
    ))
}

/// Convert an analyzer diagnostic. Same primary-label rule.
pub fn from_analyze(
    diag: &AnalyzeDiagnostic,
    files: &FileMap<'_>,
) -> Option<(usize, tower_lsp::lsp_types::Diagnostic)> {
    let primary = diag
        .labels
        .iter()
        .find(|l| l.is_primary)
        .or_else(|| diag.labels.first())?;
    let range = label_range(primary, files)?;

    let related: Vec<DiagnosticRelatedInformation> = diag
        .labels
        .iter()
        .filter(|l| !std::ptr::eq(*l, primary))
        .filter_map(|l| {
            let loc = files.span_to_location(&l.span)?;
            Some(DiagnosticRelatedInformation {
                location: loc,
                message: if l.message.is_empty() {
                    diag.message.clone()
                } else {
                    l.message.clone()
                },
            })
        })
        .collect();

    let message = build_message(&diag.message, &primary.message, &diag.notes);
    Some((
        primary.span.file_id,
        tower_lsp::lsp_types::Diagnostic {
            range,
            severity: Some(severity_from_analyze(diag.severity)),
            code: Some(NumberOrString::String(diag.descriptor_id.into())),
            code_description: None,
            source: Some("kestrel".into()),
            message,
            related_information: if related.is_empty() {
                None
            } else {
                Some(related)
            },
            tags: None,
            data: None,
        },
    ))
}

fn label_range(label: &DiagLabel, files: &FileMap<'_>) -> Option<Range> {
    let (_, idx) = files.lookup(label.span.file_id)?;
    Some(idx.range_for(label.span.start, label.span.end))
}

fn severity_from_codespan(s: CsSeverity) -> DiagnosticSeverity {
    match s {
        CsSeverity::Bug | CsSeverity::Error => DiagnosticSeverity::ERROR,
        CsSeverity::Warning => DiagnosticSeverity::WARNING,
        CsSeverity::Note => DiagnosticSeverity::INFORMATION,
        CsSeverity::Help => DiagnosticSeverity::HINT,
    }
}

fn severity_from_analyze(s: AnalyzeSeverity) -> DiagnosticSeverity {
    match s {
        AnalyzeSeverity::Error => DiagnosticSeverity::ERROR,
        AnalyzeSeverity::Warning => DiagnosticSeverity::WARNING,
        AnalyzeSeverity::Info => DiagnosticSeverity::INFORMATION,
    }
}

fn build_message(headline: &str, label_msg: &str, notes: &[String]) -> String {
    let mut out = headline.to_string();
    if !label_msg.is_empty() && label_msg != headline {
        out.push_str(": ");
        out.push_str(label_msg);
    }
    for n in notes {
        out.push('\n');
        out.push_str(n);
    }
    out
}
