use kestrel_span2::Span;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// A diagnostic produced during query execution.
///
/// Accumulated via `ctx.accumulate()` inside queries. The HECS accumulator
/// system automatically clears stale diagnostics when a query re-executes,
/// so diagnostics always reflect the current compilation state.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub severity: Severity,
}
