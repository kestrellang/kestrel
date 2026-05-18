//! `textDocument/semanticTokens/full` — colored highlighting from the
//! lexer + a small set of name-resolution disambiguations.
//!
//! M2 first cut classifies entirely from the lex token stream plus a
//! PascalCase heuristic for type-shaped identifiers. Smarter
//! identifier classification (function vs property vs parameter) lands
//! later when we wire `ResolveName` results in.

use kestrel_lexer::Token;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensParams, SemanticTokensResult,
};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{SharedState, url_to_path};

/// Token-type legend, indexed by the `tokenType` field in the wire format.
/// Order matters — clients use these indices.
pub const LEGEND: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // 0
    SemanticTokenType::TYPE,      // 1
    SemanticTokenType::FUNCTION,  // 2
    SemanticTokenType::VARIABLE,  // 3
    SemanticTokenType::PROPERTY,  // 4
    SemanticTokenType::NAMESPACE, // 5
    SemanticTokenType::COMMENT,   // 6
    SemanticTokenType::STRING,    // 7
    SemanticTokenType::NUMBER,    // 8
    SemanticTokenType::OPERATOR,  // 9
];

const KEYWORD: u32 = 0;
const TYPE: u32 = 1;
const VARIABLE: u32 = 3;
const COMMENT: u32 = 6;
const STRING: u32 = 7;
const NUMBER: u32 = 8;
const OPERATOR: u32 = 9;

pub async fn handle(
    state: SharedState,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, line_index)
    };

    let data = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<SemanticToken>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let raw_tokens = compiler.lex(file_entity);
                let source = line_index.text();
                Some(encode(&raw_tokens, source, &line_index))
            },
        )
        .await??;

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

fn classify(token: Token, lexeme: &str) -> Option<u32> {
    use Token::*;
    // `Option::Some` because `use Token::*` brings `Token::Some` into scope.
    Option::Some(match token {
        // Trivia
        LineComment | BlockComment => COMMENT,
        Whitespace | Newline => return None,

        // Literals
        String | Char | RawString => STRING,
        Integer | Float => NUMBER,
        Boolean | Null => KEYWORD,

        // Keywords (the simple block — every reserved word that isn't an op).
        Extend | Fileprivate | Func | Import | Deinit | Init | Internal | Let | Module
        | Mutating | Private | Protocol | Public | Static | Struct | Type | Var | Where | Enum
        | Case | Indirect | And | Not | Or | As | Break | Consuming | Continue | Else | For
        | If | In | Loop | Return | Throw | Try | Throws | While | Match | Guard | Get | Set
        | Subscript | Some => KEYWORD,

        // Identifiers — heuristic. PascalCase → type, anything else → variable.
        Identifier => {
            if lexeme.chars().next().is_some_and(|c| c.is_uppercase()) {
                TYPE
            } else {
                VARIABLE
            }
        },
        Underscore => VARIABLE,

        // Treat all other punctuation tokens as operators. Brackets are
        // omitted — clients expect them uncolored, falling back to the
        // TextMate grammar.
        _ => OPERATOR,
    })
}

/// Encode tokens as LSP delta-encoded quintuples. Skips trivia and any
/// token that spans multiple lines (LSP requires per-line entries; for
/// multi-line strings we'd split at newlines — punted to a later pass).
fn encode(
    raw: &[kestrel_lexer::SpannedToken],
    source: &str,
    idx: &LineIndex,
) -> Vec<SemanticToken> {
    let mut out = Vec::with_capacity(raw.len());
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for spanned in raw {
        let token = spanned.value.clone();
        let span = &spanned.span;
        let lexeme = source.get(span.start..span.end).unwrap_or("");
        let Some(ttype) = classify(token, lexeme) else {
            continue;
        };

        let start_pos = idx.offset_to_position(span.start);
        let end_pos = idx.offset_to_position(span.end);
        if start_pos.line != end_pos.line {
            // Multi-line tokens (block comments, raw strings) — skip for M2.
            // They still get TextMate highlighting from the grammar.
            continue;
        }
        let length = end_pos.character - start_pos.character;
        if length == 0 {
            continue;
        }

        let delta_line = start_pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            start_pos.character - prev_start
        } else {
            start_pos.character
        };

        out.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ttype,
            token_modifiers_bitset: 0,
        });

        prev_line = start_pos.line;
        prev_start = start_pos.character;
    }
    out
}
