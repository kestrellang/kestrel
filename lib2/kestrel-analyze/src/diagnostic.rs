//! Diagnostic types for analysis output.
//!
//! `AnalyzeDiagnostic` is the rich diagnostic type accumulated via HECS.
//! `DiagnosticDescriptor` is static metadata per diagnostic kind (Roslyn-style).

use kestrel_span2::Span;

/// Severity of an analysis diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Category for grouping diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Correctness,
    Style,
    Performance,
    Usage,
}

/// Static metadata for a diagnostic kind (Roslyn: DiagnosticDescriptor).
///
/// Each analyzer declares one or more descriptors. Users configure
/// severity overrides and suppressions by descriptor ID.
pub struct DiagnosticDescriptor {
    /// Unique ID, e.g. "KS001". Used for configuration and suppression.
    pub id: &'static str,
    /// Human-readable name, e.g. "missing_return".
    pub name: &'static str,
    /// Default severity before user overrides.
    pub default_severity: Severity,
    /// Grouping category.
    pub category: Category,
}

/// A labeled source location within a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiagLabel {
    pub span: Span,
    pub message: String,
    /// Primary label (the main error site) vs secondary (related context).
    pub is_primary: bool,
}

/// A rich diagnostic produced by an analyzer.
///
/// Accumulated via HECS accumulators. Decoupled from codespan_reporting —
/// conversion to display format happens in the compiler orchestration layer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnalyzeDiagnostic {
    /// Which descriptor produced this (e.g. "KS001"). Used for suppression.
    pub descriptor_id: &'static str,
    /// Severity (starts as descriptor default, may be overridden by config).
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Source labels (primary + secondary).
    pub labels: Vec<DiagLabel>,
    /// Additional notes (free text, shown after labels).
    pub notes: Vec<String>,
}
