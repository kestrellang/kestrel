pub use kestrel_span::{Span, Spanned};
use logos::Logos;
use unicode_xid::UnicodeXID;

/// Check if a string is a valid Unicode identifier
fn is_valid_identifier(lex: &mut logos::Lexer<Token>) -> bool {
    let slice = lex.slice();
    let mut chars = slice.chars();

    // First character must be XID_Start or underscore
    if let Some(first) = chars.next() {
        if !first.is_xid_start() && first != '_' {
            return false;
        }
    } else {
        return false;
    }

    // Remaining characters must be XID_Continue
    chars.all(|c| c.is_xid_continue())
}

/// Parse a raw string literal that starts with 3+ quotes and ends with the same number.
/// The token has already matched the initial `"""`, so we need to:
/// 1. Count any additional opening quotes (consecutive quotes at the start of remainder)
/// 2. Determine the total quote count for this raw string
/// 3. Find the matching closing quotes
///
/// For `""""content""""` (4-quote raw string):
/// - regex matches `"""`
/// - remainder is `"content""""`
/// - 1 more quote at start → quote_count = 4
/// - Content starts after that extra quote
/// - We scan until we find 4 consecutive quotes
///
/// For `""""""` (empty 3-quote raw string):
/// - regex matches `"""`
/// - remainder is `"""`
/// - If those 3 quotes are immediately followed by non-quote or EOF, they are closing quotes
/// - quote_count = 3, empty content
fn parse_raw_string(lex: &mut logos::Lexer<Token>) -> bool {
    let remainder = lex.remainder();
    let mut chars = remainder.chars().peekable();
    let mut offset = 0;

    // Count consecutive quotes at the start (these are additional opening quotes)
    let mut additional_quotes = 0;
    while chars.peek() == Some(&'"') {
        // Look ahead: if this quote followed by (additional_quotes + 3) more quotes
        // would give us exactly quote_count closing quotes, we should stop counting.
        // The heuristic: if the next non-quote char comes right after, these aren't opening quotes.

        // For now, use simple heuristic: if we see N quotes followed by non-quote or EOF,
        // and N >= 3 (the initial match), then N total quotes form an empty raw string.
        // Otherwise, continue counting as opening quotes.

        // Actually, let's use a different approach: peek ahead to see if stopping now
        // would result in a valid closing sequence immediately.
        let peek_chars = remainder[additional_quotes..].chars();
        let mut consecutive = 0;
        for c in peek_chars.clone() {
            if c == '"' {
                consecutive += 1;
            } else {
                break;
            }
        }

        // If we're at a point where the remaining quotes (after additional_quotes additional opening)
        // exactly equals the quote count we'd need, it's an empty string
        let potential_quote_count = 3 + additional_quotes;
        if consecutive == potential_quote_count {
            // This means we have exactly the right number of closing quotes for an empty string
            // Don't count more opening quotes
            break;
        }

        chars.next();
        offset += 1;
        additional_quotes += 1;
    }

    let quote_count = 3 + additional_quotes;

    // Now scan for the closing sequence of `quote_count` quotes
    let mut consecutive_quotes = 0;

    for c in chars {
        offset += c.len_utf8();

        if c == '"' {
            consecutive_quotes += 1;
            if consecutive_quotes == quote_count {
                // Found the closing sequence
                lex.bump(offset);
                return true;
            }
        } else {
            consecutive_quotes = 0;
        }
    }

    // Unterminated raw string - consume everything we've seen
    lex.bump(offset);
    true
}

/// Scan a nested string within an interpolation expression.
/// Returns the number of bytes consumed (including the closing quote).
fn scan_nested_string(chars: &mut std::iter::Peekable<std::str::Chars>, remainder: &str) -> usize {
    let mut offset = 0;

    while let Some(&c) = chars.peek() {
        chars.next();
        offset += c.len_utf8();

        match c {
            '"' => {
                // End of nested string
                return offset;
            }
            '\\' => {
                // Escape sequence - consume the next character
                if let Some(&next) = chars.peek() {
                    chars.next();
                    offset += next.len_utf8();

                    if next == '(' {
                        // Nested interpolation within nested string!
                        offset += scan_interpolation(chars, remainder);
                    }
                }
            }
            _ => {}
        }
    }

    offset
}

/// Scan an interpolation expression `\(...)`.
/// We've already consumed the `\(`. This scans until the matching `)`.
/// Returns the number of additional bytes consumed.
fn scan_interpolation(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    remainder: &str,
) -> usize {
    let mut offset = 0;
    let mut paren_depth = 1; // We've already seen one '('
    let mut bracket_depth = 0;
    let mut brace_depth = 0;

    while let Some(&c) = chars.peek() {
        chars.next();
        offset += c.len_utf8();

        match c {
            '(' => paren_depth += 1,
            ')' => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    // End of interpolation
                    return offset;
                }
            }
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
            '"' => {
                // Nested string within interpolation
                offset += scan_nested_string(chars, remainder);
            }
            '\'' => {
                // Character literal within interpolation - scan it
                offset += scan_char_literal(chars);
            }
            '/' => {
                // Possible comment - check for // or /*
                if let Some(&next) = chars.peek() {
                    if next == '/' {
                        // Line comment - skip to end of line
                        chars.next();
                        offset += 1;
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            chars.next();
                            offset += c.len_utf8();
                        }
                    } else if next == '*' {
                        // Block comment - skip with nesting
                        chars.next();
                        offset += 1;
                        offset += scan_block_comment_in_interpolation(chars);
                    }
                }
            }
            _ => {}
        }
    }

    // Unterminated interpolation
    offset
}

/// Scan a character literal within an interpolation.
/// We've already consumed the opening `'`.
fn scan_char_literal(chars: &mut std::iter::Peekable<std::str::Chars>) -> usize {
    let mut offset = 0;

    while let Some(&c) = chars.peek() {
        chars.next();
        offset += c.len_utf8();

        match c {
            '\'' => return offset, // End of char literal
            '\\' => {
                // Escape sequence - consume next char
                if let Some(&next) = chars.peek() {
                    chars.next();
                    offset += next.len_utf8();
                }
            }
            _ => {}
        }
    }

    offset
}

/// Scan a block comment within an interpolation (handles nesting).
/// We've already consumed `/*`.
fn scan_block_comment_in_interpolation(chars: &mut std::iter::Peekable<std::str::Chars>) -> usize {
    let mut offset = 0;
    let mut depth = 1;

    while let Some(&c) = chars.peek() {
        chars.next();
        offset += c.len_utf8();

        if c == '/' {
            if let Some(&next) = chars.peek() {
                if next == '*' {
                    chars.next();
                    offset += 1;
                    depth += 1;
                }
            }
        } else if c == '*' {
            if let Some(&next) = chars.peek() {
                if next == '/' {
                    chars.next();
                    offset += 1;
                    depth -= 1;
                    if depth == 0 {
                        return offset;
                    }
                }
            }
        }
    }

    offset
}

/// Parse a string literal, properly handling interpolation with nested strings.
///
/// This replaces the simple regex matcher because strings with interpolation
/// can contain nested strings (e.g., `"\(dict["key"])"`), which the regex
/// can't handle correctly.
///
/// For strings WITHOUT interpolation, this produces a `String` token.
/// For strings WITH interpolation (containing `\(...)`), this produces
/// an `InterpolatedString` token.
fn parse_string(lex: &mut logos::Lexer<Token>) -> bool {
    let remainder = lex.remainder();
    let mut chars = remainder.chars().peekable();
    let mut offset = 0;

    while let Some(&c) = chars.peek() {
        chars.next();
        offset += c.len_utf8();

        match c {
            '"' => {
                // End of string
                lex.bump(offset);
                return true;
            }
            '\\' => {
                // Escape sequence
                if let Some(&next) = chars.peek() {
                    chars.next();
                    offset += next.len_utf8();

                    if next == '(' {
                        // Interpolation - scan the expression properly
                        offset += scan_interpolation(&mut chars, remainder);
                    }
                }
            }
            _ => {}
        }
    }

    // Unterminated string - consume everything we've seen
    lex.bump(offset);
    true
}

/// Parse nested block comments and return the full comment as a token
fn parse_block_comment(lex: &mut logos::Lexer<Token>) -> bool {
    let remainder = lex.remainder();
    let mut depth = 1;
    let mut chars = remainder.chars();
    let mut offset = 0;

    while let Some(c) = chars.next() {
        offset += c.len_utf8();

        if c == '/' {
            if matches!(chars.clone().next(), Some('*')) {
                chars.next();
                offset += 1;
                depth += 1;
            }
        } else if c == '*' && matches!(chars.clone().next(), Some('/')) {
            {
                chars.next();
                offset += 1;
                depth -= 1;
                if depth == 0 {
                    lex.bump(offset);
                    return true;
                }
            }
        }
    }

    // Unclosed comment - bump to end
    lex.bump(offset);
    true
}

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    // ===== Trivia =====
    // Whitespace and comments are emitted as tokens so rowan can calculate
    // correct source positions. The parser treats these as trivia.
    #[regex(r"[ \t\n\f]+")]
    Whitespace,

    #[regex(r"//[^\n]*", allow_greedy = true)]
    LineComment,

    #[regex(r"/\*", parse_block_comment)]
    BlockComment,

    // ===== Literals =====
    // Underscore alone is a special token (for inferred types)
    // Higher priority ensures "_" is matched as Underscore, not Identifier
    #[token("_", priority = 3)]
    Underscore,

    // Match potential Unicode identifiers and validate with XID rules
    #[regex(r"[\p{L}_][\p{L}\p{N}_]*", is_valid_identifier)]
    Identifier,

    // String literals - use callback to handle interpolation with nested strings
    // The callback properly handles cases like `"\(dict["key"])"` where nested
    // strings appear within interpolation expressions.
    #[regex(r#"""#, parse_string)]
    String,

    // Character literals - single quotes with escape support
    #[regex(r#"'([^'\\]|\\(.|\r|\n))*'"#)]
    Char,

    // Raw string literals: """content""" or """"content"""" etc.
    // Must have higher priority than String to match first
    #[regex(r#"""""#, parse_raw_string, priority = 2)]
    RawString,

    // Integer literals with optional underscores: 1_000_000, 0xFF_FF, 0b1010_1010, 0o755_000
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*|0[bB][01][01_]*|0[oO][0-7][0-7_]*|[0-9][0-9_]*")]
    Integer,

    // Float literals with optional underscores: 1_000.5, 1.5e10
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9][0-9_]*)?")]
    Float,

    #[token("true")]
    #[token("false")]
    Boolean,

    #[token("null")]
    Null,

    // ===== Declaration Keywords =====
    #[token("extend")]
    Extend,

    #[token("fileprivate")]
    Fileprivate,

    #[token("func")]
    Func,

    #[token("import")]
    Import,

    #[token("deinit")]
    Deinit,

    #[token("init")]
    Init,

    #[token("internal")]
    Internal,

    #[token("let")]
    Let,

    #[token("module")]
    Module,

    #[token("mutating")]
    Mutating,

    #[token("private")]
    Private,

    #[token("protocol")]
    Protocol,

    #[token("public")]
    Public,

    #[token("static")]
    Static,

    #[token("struct")]
    Struct,

    #[token("type")]
    Type,

    #[token("var")]
    Var,

    #[token("where")]
    Where,

    // ===== Enum Keywords =====
    #[token("enum")]
    Enum,

    #[token("case")]
    Case,

    #[token("indirect")]
    Indirect,

    // ===== Logical Keywords =====
    #[token("and")]
    And,

    #[token("not")]
    Not,

    #[token("or")]
    Or,

    // ===== Statement Keywords =====
    #[token("as")]
    As,

    #[token("break")]
    Break,

    #[token("consuming")]
    Consuming,

    #[token("continue")]
    Continue,

    #[token("else")]
    Else,

    #[token("for")]
    For,

    #[token("if")]
    If,

    #[token("in")]
    In,

    #[token("loop")]
    Loop,

    #[token("return")]
    Return,

    #[token("throw")]
    Throw,

    #[token("try")]
    Try,

    #[token("throws")]
    Throws,

    #[token("while")]
    While,

    #[token("match")]
    Match,

    #[token("guard")]
    Guard,

    // ===== Property Accessor Keywords =====
    #[token("get")]
    Get,

    #[token("set")]
    Set,

    #[token("subscript")]
    Subscript,

    // ===== Braces =====
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    // ===== Punctuation =====
    #[token(";")]
    Semicolon,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token(":")]
    Colon,

    #[token("?")]
    Question,

    #[token("!")]
    Bang,

    // ===== Operators =====
    // Note: Longer tokens must come before shorter ones for correct matching

    // Multi-character operators (longest first)
    #[token("..=")]
    DotDotEquals,

    #[token("..<")]
    DotDotLess,

    #[token("..")]
    DotDot,

    // Compound assignment operators (3-char, must come before 2-char shift operators)
    #[token("<<=")]
    LessLessEquals,

    #[token(">>=")]
    GreaterGreaterEquals,

    // Shift operators (2-char)
    #[token("<<")]
    LessLess,

    #[token(">>")]
    GreaterGreater,

    // Comparison operators (2-char)
    #[token("<=")]
    LessEquals,

    #[token(">=")]
    GreaterEquals,

    #[token("==")]
    EqualsEquals,

    #[token("!=")]
    BangEquals,

    #[token("??")]
    QuestionQuestion,

    #[token("->")]
    Arrow,

    #[token("=>")]
    FatArrow,

    // Compound assignment operators (2-char, must come before single-char operators)
    #[token("+=")]
    PlusEquals,

    #[token("-=")]
    MinusEquals,

    #[token("*=")]
    StarEquals,

    #[token("/=")]
    SlashEquals,

    #[token("%=")]
    PercentEquals,

    #[token("&=")]
    AmpersandEquals,

    #[token("|=")]
    PipeEquals,

    #[token("^=")]
    CaretEquals,

    // Single-character operators
    #[token("=")]
    Equals,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token("^")]
    Caret,

    #[token("<")]
    Less,

    #[token(">")]
    Greater,

    #[token("@")]
    At,
}

pub type SpannedToken = Spanned<Token>;

/// Lex source code and return an iterator of tokens with their spans.
///
/// The `file_id` is embedded in each token's span for use in diagnostics.
pub fn lex(
    source: &str,
    file_id: usize,
) -> impl Iterator<Item = Result<SpannedToken, Spanned<()>>> + '_ {
    Token::lexer(source).spanned().map(move |(token, span)| {
        let span = Span::new(file_id, span);
        token
            .map(|t| Spanned::new(t, span.clone()))
            .map_err(|_| Spanned::new((), span))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Filter out trivia tokens (whitespace and comments) for tests
    fn filter_trivia(tokens: Vec<Result<Spanned<Token>, Spanned<()>>>) -> Vec<Spanned<Token>> {
        tokens
            .into_iter()
            .filter_map(|t| t.ok())
            .filter(|t| {
                !matches!(
                    t.value,
                    Token::Whitespace | Token::LineComment | Token::BlockComment
                )
            })
            .collect()
    }

    #[test]
    fn test_lexer() {
        let source = "func main() { let x = 42; }";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert!(!tokens.is_empty());

        // First token should be 'func' at position 0..4
        assert_eq!(tokens[0].value, Token::Func);
        assert_eq!(tokens[0].span.range(), 0..4);
    }

    #[test]
    fn test_spans() {
        let source = "let x = 42";
        let tokens = filter_trivia(lex(source, 0).collect());

        // Verify spans don't overlap and cover the source
        assert_eq!(tokens[0].span.range(), 0..3); // "let"
        assert_eq!(tokens[1].span.range(), 4..5); // "x"
        assert_eq!(tokens[2].span.range(), 6..7); // "="
        assert_eq!(tokens[3].span.range(), 8..10); // "42"
    }

    #[test]
    fn test_literals() {
        // Test string literals
        let source = r#""hello world""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::String);

        // Test integer literals - decimal
        let source = "42";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Integer);

        // Test integer literals - hexadecimal
        let source = "0xFF 0XAB 0x1a2b";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Integer);
        assert_eq!(tokens[1].value, Token::Integer);
        assert_eq!(tokens[2].value, Token::Integer);

        // Test integer literals - binary
        let source = "0b1010 0B1111";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Integer);
        assert_eq!(tokens[1].value, Token::Integer);

        // Test integer literals - octal
        let source = "0o17 0O755";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Integer);
        assert_eq!(tokens[1].value, Token::Integer);

        // Test float literals
        let source = "3.14 2.5e10 1.0E-5";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Float);
        assert_eq!(tokens[1].value, Token::Float);
        assert_eq!(tokens[2].value, Token::Float);

        // Test boolean literals
        let source = "true false";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Boolean);
        assert_eq!(tokens[1].value, Token::Boolean);

        // Test null literal
        let source = "null";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens[0].value, Token::Null);
    }

    #[test]
    fn test_module_declaration() {
        let source = "module A.B.C";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].value, Token::Module);
        assert_eq!(tokens[1].value, Token::Identifier);
        assert_eq!(tokens[2].value, Token::Dot);
        assert_eq!(tokens[3].value, Token::Identifier);
        assert_eq!(tokens[4].value, Token::Dot);
        assert_eq!(tokens[5].value, Token::Identifier);
    }

    #[test]
    fn test_unicode_identifiers() {
        // Test various Unicode identifier patterns
        let source = "let café = 42";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].value, Token::Let);
        assert_eq!(tokens[1].value, Token::Identifier); // café
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::Integer);

        // Test Greek identifiers
        let source = "func αβγ() { }";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens[0].value, Token::Func);
        assert_eq!(tokens[1].value, Token::Identifier); // αβγ

        // Test mixed scripts
        let source = "let _hello世界 = 42";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens[1].value, Token::Identifier); // _hello世界
    }

    #[test]
    fn test_line_comments() {
        let source = r#"
            let x = 42; // This is a comment
            let y = 10; // Another comment
        "#;
        let tokens = filter_trivia(lex(source, 0).collect());

        // Comments should be skipped
        // Tokens: let x = 42 ; let y = 10 ;
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].value, Token::Let);
        assert_eq!(tokens[1].value, Token::Identifier); // x
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::Integer); // 42
        assert_eq!(tokens[4].value, Token::Semicolon);
        assert_eq!(tokens[5].value, Token::Let);
        assert_eq!(tokens[6].value, Token::Identifier); // y
        assert_eq!(tokens[7].value, Token::Equals);
        assert_eq!(tokens[8].value, Token::Integer); // 10
        assert_eq!(tokens[9].value, Token::Semicolon);
    }

    #[test]
    fn test_block_comments() {
        let source = r#"
            let x = /* comment */ 42;
            /* multi
               line
               comment */
            let y = 10;
        "#;
        let tokens = filter_trivia(lex(source, 0).collect());

        // Comments should be skipped
        // Tokens: let x = 42 ; let y = 10 ;
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].value, Token::Let);
        assert_eq!(tokens[1].value, Token::Identifier); // x
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::Integer); // 42
        assert_eq!(tokens[4].value, Token::Semicolon);
        assert_eq!(tokens[5].value, Token::Let);
        assert_eq!(tokens[6].value, Token::Identifier); // y
        assert_eq!(tokens[7].value, Token::Equals);
        assert_eq!(tokens[8].value, Token::Integer); // 10
        assert_eq!(tokens[9].value, Token::Semicolon);
    }

    #[test]
    fn test_nested_comments() {
        let source = r#"
            let x = /* outer /* inner */ still outer */ 42;
            let y = /* /* /* deeply */ nested */ comments */ 10;
        "#;
        let tokens = filter_trivia(lex(source, 0).collect());

        // All nested comments should be properly handled
        // Tokens: let x = 42 ; let y = 10 ;
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].value, Token::Let);
        assert_eq!(tokens[1].value, Token::Identifier); // x
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::Integer); // 42
        assert_eq!(tokens[4].value, Token::Semicolon);
        assert_eq!(tokens[5].value, Token::Let);
        assert_eq!(tokens[6].value, Token::Identifier); // y
        assert_eq!(tokens[7].value, Token::Equals);
        assert_eq!(tokens[8].value, Token::Integer); // 10
        assert_eq!(tokens[9].value, Token::Semicolon);
    }

    #[test]
    fn test_comments_dont_affect_strings() {
        let source = r#"let s = "// not a comment";"#;
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 5); // let s = "..." ;
        assert_eq!(tokens[0].value, Token::Let);
        assert_eq!(tokens[1].value, Token::Identifier); // s
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::String);
        assert_eq!(tokens[4].value, Token::Semicolon);
    }

    #[test]
    fn test_import_keyword() {
        let source = "import A.B.C";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].value, Token::Import);
        assert_eq!(tokens[1].value, Token::Identifier); // A
        assert_eq!(tokens[2].value, Token::Dot);
        assert_eq!(tokens[3].value, Token::Identifier); // B
        assert_eq!(tokens[4].value, Token::Dot);
        assert_eq!(tokens[5].value, Token::Identifier); // C
    }

    #[test]
    fn test_import_with_as() {
        let source = "import A.B.C as D";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].value, Token::Import);
        assert_eq!(tokens[1].value, Token::Identifier); // A
        assert_eq!(tokens[2].value, Token::Dot);
        assert_eq!(tokens[3].value, Token::Identifier); // B
        assert_eq!(tokens[4].value, Token::Dot);
        assert_eq!(tokens[5].value, Token::Identifier); // C
        assert_eq!(tokens[6].value, Token::As);
        assert_eq!(tokens[7].value, Token::Identifier); // D
    }

    #[test]
    fn test_import_with_list() {
        let source = "import A.B.C.(D, E)";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 12);
        assert_eq!(tokens[0].value, Token::Import);
        assert_eq!(tokens[1].value, Token::Identifier); // A
        assert_eq!(tokens[2].value, Token::Dot);
        assert_eq!(tokens[3].value, Token::Identifier); // B
        assert_eq!(tokens[4].value, Token::Dot);
        assert_eq!(tokens[5].value, Token::Identifier); // C
        assert_eq!(tokens[6].value, Token::Dot);
        assert_eq!(tokens[7].value, Token::LParen);
        assert_eq!(tokens[8].value, Token::Identifier); // D
        assert_eq!(tokens[9].value, Token::Comma);
        assert_eq!(tokens[10].value, Token::Identifier); // E
        assert_eq!(tokens[11].value, Token::RParen);
    }

    #[test]
    fn test_type_alias_declaration() {
        let source = "type Alias = Aliased;";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].value, Token::Type);
        assert_eq!(tokens[1].value, Token::Identifier); // Alias
        assert_eq!(tokens[2].value, Token::Equals);
        assert_eq!(tokens[3].value, Token::Identifier); // Aliased
        assert_eq!(tokens[4].value, Token::Semicolon);
    }

    #[test]
    fn test_type_alias_with_visibility() {
        let source = "public type Alias = Aliased;";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].value, Token::Public);
        assert_eq!(tokens[1].value, Token::Type);
        assert_eq!(tokens[2].value, Token::Identifier); // Alias
        assert_eq!(tokens[3].value, Token::Equals);
        assert_eq!(tokens[4].value, Token::Identifier); // Aliased
        assert_eq!(tokens[5].value, Token::Semicolon);
    }

    #[test]
    fn test_in_keyword() {
        // Test `in` keyword for closure parameters
        let source = "{ (x) in x }";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].value, Token::LBrace);
        assert_eq!(tokens[1].value, Token::LParen);
        assert_eq!(tokens[2].value, Token::Identifier); // x
        assert_eq!(tokens[3].value, Token::RParen);
        assert_eq!(tokens[4].value, Token::In);
        assert_eq!(tokens[5].value, Token::Identifier); // x
        assert_eq!(tokens[6].value, Token::RBrace);

        // Ensure `in` is not confused with identifiers starting with "in"
        let source = "in inside inner";
        let tokens = filter_trivia(lex(source, 0).collect());

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].value, Token::In);
        assert_eq!(tokens[1].value, Token::Identifier); // inside
        assert_eq!(tokens[2].value, Token::Identifier); // inner
    }

    #[test]
    fn test_char_literals() {
        // Basic character literal
        let source = "'a'";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::Char);

        // Character with escape sequence
        let source = r"'\n' '\t' '\\'";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].value, Token::Char);
        assert_eq!(tokens[1].value, Token::Char);
        assert_eq!(tokens[2].value, Token::Char);

        // Unicode character
        let source = "'Ω' '日' '🦅'";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].value, Token::Char);
        assert_eq!(tokens[1].value, Token::Char);
        assert_eq!(tokens[2].value, Token::Char);

        // Unicode escape
        let source = r"'\u{1F600}'";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::Char);

        // Empty character literal (lexer accepts it, semantic layer validates)
        let source = "''";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::Char);

        // Multiple characters (lexer accepts, semantic layer validates)
        let source = "'ab'";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::Char);
    }

    #[test]
    fn test_raw_strings() {
        // Basic raw string with 3 quotes
        let source = r#""""hello world""""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Raw string with newlines
        let source = "\"\"\"hello\nworld\"\"\"";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Raw string with 4 quotes (allows 3 quotes inside)
        let source = r#"""""hello """ world"""""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Raw string with backslashes (no escape processing)
        let source = r#""""hello\nworld""""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Empty raw string
        let source = r#""""""""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Regular string is still recognized
        let source = r#""hello""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_interpolation_basic() {
        // Simple interpolation
        let source = r#""Hello \(name)!""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        // Verify the full string is captured
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Multiple interpolations
        let source = r#""\(a) and \(b)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[0].span.range(), 0..source.len());
    }

    #[test]
    fn test_string_interpolation_nested_strings() {
        // Nested string in interpolation: "\(dict["key"])"
        let source = r#""\(dict["key"])""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // More complex: "\(a["b"]["c"])"
        let source = r#""\(a["b"]["c"])""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[0].span.range(), 0..source.len());
    }

    #[test]
    fn test_string_interpolation_nested_interpolation() {
        // Nested interpolation: "\("inner \(x)")"
        let source = r#""\("inner \(x)")""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[0].span.range(), 0..source.len());
    }

    #[test]
    fn test_string_interpolation_with_expressions() {
        // Interpolation with function call
        let source = r#""\(foo(a, b))""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Interpolation with array subscript
        let source = r#""\(arr[0])""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Interpolation with arithmetic
        let source = r#""\(a + b * c)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Interpolation with closure
        let source = r#""\(items.map { x in x * 2 })""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_interpolation_with_format_spec() {
        // Format specifier
        let source = r#""\(x:>8)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Hex format
        let source = r#""\(n:08x)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_interpolation_edge_cases() {
        // Empty interpolation (will be caught as error later)
        let source = r#""\()""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Escaped backslash before paren (not interpolation)
        let source = r#""\\(not interpolation)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Consecutive interpolations
        let source = r#""\(a)\(b)\(c)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Interpolation at boundaries
        let source = r#""\(x)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_interpolation_with_char_literal() {
        // Char literal inside interpolation
        let source = r#""\(c == 'x')""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Char literal with escape
        let source = r#""\(c == '\n')""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_interpolation_with_comments() {
        // Line comment in interpolation
        let source = "\"\\(x // comment\n)\"";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Block comment in interpolation
        let source = r#""\(x /* comment */ + y)""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_string_after_interpolated_string() {
        // Ensure next string is correctly tokenized
        let source = r#""\(x)" "y""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[1].value, Token::String);
    }
}
