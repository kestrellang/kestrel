use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_parser2::{ParseResult, parse_source_file_from_source};

use crate::components::SourceText;
use crate::diagnostic::{self, ThrowDiagnostic};
use crate::queries::LexFile;

/// Parse a source file entity into a syntax tree.
///
/// Depends on `LexFile` (records the dependency automatically) and reads
/// `SourceText` for the raw source. Accumulates parse errors as diagnostics.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ParseFile {
    pub entity: Entity,
}

impl QueryFn for ParseFile {
    type Output = ParseResult;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        // Sub-query: get lexed tokens (records dependency on LexFile)
        let tokens = ctx.query(LexFile {
            entity: self.entity,
        });

        let source = ctx
            .get::<SourceText>(self.entity)
            .map(|s| s.0.as_str())
            .unwrap_or("");

        // Convert SpannedToken → (Token, Span) iterator for the parser
        let token_iter = tokens.iter().map(|st| (st.value.clone(), st.span.clone()));

        let result = parse_source_file_from_source(source, token_iter);

        // Accumulate parse errors as diagnostics
        for error in &result.errors {
            if let Some(span) = &error.span {
                ctx.throw(diagnostic::ParseError {
                    message: error.message.clone(),
                    span: span.clone(),
                });
            }
        }

        result
    }
}
