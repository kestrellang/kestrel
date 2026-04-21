use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_lexer2::{SpannedToken, lex};

use crate::components::SourceText;
use crate::diagnostic::{LexError, ThrowDiagnostic};

/// Lex a source file entity into tokens.
///
/// Reads the `SourceText` component, runs the lexer, and accumulates
/// diagnostics for any lex errors. Returns a clean token stream —
/// downstream queries don't need to handle errors.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LexFile {
    pub entity: Entity,
}

impl QueryFn for LexFile {
    type Output = Vec<SpannedToken>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        let source = match ctx.get::<SourceText>(self.entity) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Entity index serves as the file_id in spans
        let file_id = self.entity.index();

        let mut tokens = Vec::new();
        for result in lex(&source.0, file_id) {
            match result {
                Ok(token) => tokens.push(token),
                Err(err) => ctx.throw(LexError { span: err.span }),
            }
        }
        tokens
    }
}
