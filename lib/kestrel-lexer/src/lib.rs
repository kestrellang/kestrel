pub use kestrel_span::{Span, Spanned};
use logos::Logos;

mod modal;
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

/// Parse a pound-prefixed raw string. The regex matches `#+"` (one or more
/// pounds + one quote). The slice consumed so far is `#`...`#"` (N pounds +
/// 1 quote). We then peek the remainder to determine single-line vs
/// multi-line and to find the matching closer (`"` + N pounds for
/// single-line, `"""` + N pounds for multi-line).
///
/// Forms:
/// - `#"..."#`  — single-line raw, N=1 pound
/// - `##"..."##` — single-line raw, N=2 pounds (lets you embed `"#`)
/// - `#"""\n...\n"""#` — multi-line raw
///
/// 3+ consecutive quotes after the pound prefix → multi-line opener; 1 or 2
/// → single-line opener.
fn parse_pound_string(lex: &mut logos::Lexer<Token>) -> bool {
    let slice = lex.slice();
    let pound_count = slice.chars().take_while(|&c| c == '#').count();
    let remainder = lex.remainder();

    // Peek for additional opening quotes (the regex matched 1 quote already).
    let extra_open_quotes = remainder.chars().take_while(|&c| c == '"').count();
    let total_open_quotes = 1 + extra_open_quotes;

    if total_open_quotes >= 3 {
        // Multi-line raw. Opener is exactly `"""` (3 quotes); any further
        // quotes are content. Consume 2 more opener quotes (1 was matched).
        scan_raw_close(
            lex,
            pound_count,
            /* opener_quote_count = */ 3,
            /* extra_consumed = */ 2,
        )
    } else {
        // Single-line raw. Opener is exactly `"` (1 quote). No extra quotes
        // to consume from the regex match.
        scan_raw_close(
            lex,
            pound_count,
            /* opener_quote_count = */ 1,
            /* extra_consumed = */ 0,
        )
    }
}

/// Scan for the closing delimiter of a raw string and bump the lexer past it.
/// The closer is `quote_count` quotes followed by `pound_count` pounds.
/// `extra_consumed` is the number of opener bytes (after the regex slice) we
/// need to skip before content begins (used for the 2 extra `"`s in a
/// multi-line opener).
fn scan_raw_close(
    lex: &mut logos::Lexer<Token>,
    pound_count: usize,
    quote_count: usize,
    extra_consumed: usize,
) -> bool {
    let remainder = lex.remainder();
    let mut offset = extra_consumed;
    let mut consecutive_quotes = 0;

    let bytes = remainder.as_bytes();
    while offset < bytes.len() {
        let b = bytes[offset];
        offset += 1;

        if b == b'"' {
            consecutive_quotes += 1;
            if consecutive_quotes >= quote_count {
                // Need exactly `pound_count` pounds immediately after.
                let pounds_start = offset;
                let mut pounds_seen = 0;
                while pounds_seen < pound_count
                    && pounds_start + pounds_seen < bytes.len()
                    && bytes[pounds_start + pounds_seen] == b'#'
                {
                    pounds_seen += 1;
                }
                if pounds_seen == pound_count {
                    lex.bump(offset + pound_count);
                    return true;
                }
                // Not enough pounds — keep scanning (consecutive_quotes stays).
            }
        } else {
            consecutive_quotes = 0;
        }
    }

    // Unterminated — consume the rest. Downstream surfaces the error.
    lex.bump(remainder.len());
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
    #[regex(r"[ \t\f]+")]
    Whitespace,

    #[regex(r"\r\n|\n|\r")]
    Newline,

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

    // ===== Cooked strings (see `modal.rs`) =====
    // Logos only recognises the opener; `lex` scans the body in string mode.
    // A string without `\(…)` holes comes out as ONE `String` token (its
    // whole source text); a string with holes comes out as
    // `StringStart (StringFragment | InterpStart <hole tokens>
    // (Colon FormatSpec)? InterpEnd)* StringEnd`.
    /// A complete cooked string literal with no interpolation hole.
    String,

    /// `"` or `"""` opening an interpolated string.
    #[regex(r#""("")?"#)]
    StringStart,

    /// Literal text between holes (escapes still encoded).
    StringFragment,

    /// `\(` opening an interpolation hole.
    InterpStart,

    /// `)` closing an interpolation hole.
    InterpEnd,

    /// The format specification after a hole's top-level `:`, e.g. `08x`.
    FormatSpec,

    /// `"` or `"""` closing an interpolated string.
    StringEnd,

    // Character literals - single quotes with escape support
    #[regex(r#"'([^'\\]|\\(.|\r|\n))*'"#)]
    Char,

    // Raw string literals — `#`-prefixed forms with no escape processing and
    // no interpolation. Pound count must match between opener and closer.
    //   `#"..."#`  — single-line
    //   `#"""...\n..."""#` — multi-line
    //   `##"..."##`, etc. — escalate the pound count to embed `"#` literally.
    #[regex(r##"#+""##, parse_pound_string, priority = 2)]
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

    #[token("some")]
    Some,

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

impl Token {
    /// Whether this token is trivia — carried through to the CST for fidelity,
    /// but skipped by every grammar rule.
    ///
    /// **This is the only definition of the trivia set.** `SyntaxKind::is_trivia`
    /// is its image under `From<Token>`, pinned by a test in `kestrel-syntax-tree`.
    /// Splitting a new kind out of one of these (a `DocComment` distinct from
    /// `LineComment`, say) is a one-line change here; open-coding the set at a
    /// call site instead means the new kind stops being skipped at that one site
    /// only, and the tokens silently vanish from the tree.
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
        )
    }

    /// Trivia that does not end a line — everything in [`Token::is_trivia`]
    /// except `Newline`, for the grammar positions where a line break is
    /// significant (statement ends, `}` placement).
    pub fn is_inline_trivia(&self) -> bool {
        self.is_trivia() && !matches!(self, Token::Newline)
    }

    /// Whether this token is a keyword that can appear as a parameter label.
    /// Excludes `Mutating` and `Consuming` — they're parsed as access modes.
    pub fn is_label_keyword(&self) -> bool {
        matches!(
            self,
            Token::As
                | Token::And
                | Token::Break
                | Token::Case
                | Token::Continue
                | Token::Deinit
                | Token::Else
                | Token::Enum
                | Token::Extend
                | Token::Fileprivate
                | Token::For
                | Token::Func
                | Token::Get
                | Token::Guard
                | Token::If
                | Token::Import
                | Token::In
                | Token::Indirect
                | Token::Init
                | Token::Internal
                | Token::Let
                | Token::Loop
                | Token::Match
                | Token::Module
                | Token::Not
                | Token::Or
                | Token::Private
                | Token::Protocol
                | Token::Public
                | Token::Return
                | Token::Set
                | Token::Static
                | Token::Struct
                | Token::Subscript
                | Token::Throw
                | Token::Throws
                | Token::Try
                | Token::Type
                | Token::Var
                | Token::Where
                | Token::While
        )
    }
}

pub type SpannedToken = Spanned<Token>;

/// Lex source code and return an iterator of tokens with their spans.
///
/// The `file_id` is embedded in each token's span for use in diagnostics.
/// Cooked strings are lexed modally (`modal.rs`): interpolation holes are
/// ordinary tokens between `InterpStart` and `InterpEnd`.
pub fn lex(
    source: &str,
    file_id: usize,
) -> impl Iterator<Item = Result<SpannedToken, Spanned<()>>> + '_ {
    modal::lex_modal(source)
        .into_iter()
        .map(move |(token, span)| {
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
            .filter(|t| !t.value.is_trivia())
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
    fn test_multiline_cooked_strings() {
        // `"""..."""` is now multi-line COOKED (escapes + interpolation).
        // The single-token kind is `String` — multi-line-ness is determined
        // downstream from the token text.
        let source = "\"\"\"\nhello\nworld\n\"\"\"";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Empty multi-line: `""""""` is opener `"""` + closer `"""` with
        // no body. Whether it's *valid* (Swift requires newlines) is a
        // downstream question; the lexer just consumes 6 quotes as one
        // String token.
        let source = r#""""""""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Multi-line with backslash escapes — these are now PROCESSED
        // (compare with the raw form below).
        let source = "\"\"\"\nhello\\nworld\n\"\"\"";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);

        // Single-line `"..."` still works.
        let source = r#""hello""#;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::String);
    }

    #[test]
    fn test_raw_strings_pound_prefixed() {
        // Single-line raw, 1 pound: `#"hello"#`
        let source = r##"#"hello"#"##;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Empty single-line raw: `#""#`
        let source = r##"#""#"##;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Multi-line raw: `#"""\n...\n"""#`
        let source = "#\"\"\"\nhello\nworld\n\"\"\"#";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Backslashes are NOT escaped in raw forms.
        let source = r##"#"hello\nworld"#"##;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);

        // Pound escalation: `##"contains \"# pound-quote"##`
        let source = r###"##"contains "# pound-quote"##"###;
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);
        assert_eq!(tokens[0].span.range(), 0..source.len());

        // Multi-line raw can embed `"""` when pound-escalated.
        let source = "##\"\"\"\nthree quotes \"\"\" inline\n\"\"\"##";
        let tokens = filter_trivia(lex(source, 0).collect());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, Token::RawString);
        assert_eq!(tokens[0].span.range(), 0..source.len());
    }

    /// Non-trivia token kinds of `source`, with each token's text.
    fn kinds(source: &str) -> Vec<(Token, &str)> {
        filter_trivia(lex(source, 0).collect())
            .into_iter()
            .map(|t| (t.value, &source[t.span.range()]))
            .collect()
    }

    /// Lexing must cover the source exactly: every byte in exactly one token
    /// (trivia included), in order.
    fn assert_lossless(source: &str) {
        let mut pos = 0;
        for t in lex(source, 0) {
            let span = match t {
                Ok(t) => t.span,
                Err(e) => e.span,
            };
            assert_eq!(span.start, pos, "gap or overlap in {source:?}");
            pos = span.end;
        }
        assert_eq!(pos, source.len(), "lexing stopped early in {source:?}");
    }

    use Token as T;

    #[test]
    fn plain_strings_stay_one_token() {
        for source in [
            r#""hello""#,
            r#""""#,
            r#""a\"b""#,
            r#""\\(not a hole)""#,
            "\"\"\"\n x\n \"\"\"",
        ] {
            assert_lossless(source);
            assert_eq!(kinds(source), vec![(T::String, source)], "{source}");
        }
    }

    #[test]
    fn test_string_interpolation_basic() {
        let source = r#""Hello \(name)!""#;
        assert_lossless(source);
        assert_eq!(
            kinds(source),
            vec![
                (T::StringStart, "\""),
                (T::StringFragment, "Hello "),
                (T::InterpStart, "\\("),
                (T::Identifier, "name"),
                (T::InterpEnd, ")"),
                (T::StringFragment, "!"),
                (T::StringEnd, "\""),
            ]
        );

        let source = r#""\(a) and \(b)""#;
        assert_lossless(source);
        let k: Vec<_> = kinds(source).into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            k,
            vec![
                T::StringStart,
                T::InterpStart,
                T::Identifier,
                T::InterpEnd,
                T::StringFragment,
                T::InterpStart,
                T::Identifier,
                T::InterpEnd,
                T::StringEnd,
            ]
        );
    }

    #[test]
    fn test_string_interpolation_nested_strings() {
        // A string nested in a hole is lexed by the same scanner.
        let source = r#""\(dict["key"])""#;
        assert_lossless(source);
        assert_eq!(
            kinds(source),
            vec![
                (T::StringStart, "\""),
                (T::InterpStart, "\\("),
                (T::Identifier, "dict"),
                (T::LBracket, "["),
                (T::String, "\"key\""),
                (T::RBracket, "]"),
                (T::InterpEnd, ")"),
                (T::StringEnd, "\""),
            ]
        );
    }

    #[test]
    fn test_string_interpolation_nested_interpolation() {
        let source = r#""\("inner \(x)")""#;
        assert_lossless(source);
        let k: Vec<_> = kinds(source).into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            k,
            vec![
                T::StringStart,
                T::InterpStart,
                T::StringStart,
                T::StringFragment,
                T::InterpStart,
                T::Identifier,
                T::InterpEnd,
                T::StringEnd,
                T::InterpEnd,
                T::StringEnd,
            ]
        );
    }

    #[test]
    fn test_string_interpolation_with_expressions() {
        for source in [
            r#""\(foo(a, b))""#,
            r#""\(a + b * c)""#,
            r#""\(items.map { x in x * 2 })""#,
        ] {
            assert_lossless(source);
            let toks = kinds(source);
            assert_eq!(toks.first().unwrap().0, T::StringStart, "{source}");
            assert_eq!(toks.last().unwrap().0, T::StringEnd, "{source}");
            assert_eq!(toks[toks.len() - 2].0, T::InterpEnd, "{source}");
        }
    }

    #[test]
    fn test_string_interpolation_with_format_spec() {
        let source = r#""\(n:08x)""#;
        assert_lossless(source);
        assert_eq!(
            kinds(source),
            vec![
                (T::StringStart, "\""),
                (T::InterpStart, "\\("),
                (T::Identifier, "n"),
                (T::Colon, ":"),
                (T::FormatSpec, "08x"),
                (T::InterpEnd, ")"),
                (T::StringEnd, "\""),
            ]
        );
    }

    #[test]
    fn colon_inside_brackets_is_not_a_format_spec() {
        // Audit H4: `"\([1: 2].count)"` — the `:` belongs to a dictionary.
        let source = r#""\([1: 2].count)""#;
        assert_lossless(source);
        let toks = kinds(source);
        assert!(toks.iter().all(|(k, _)| *k != T::FormatSpec), "{toks:?}");
        assert!(toks.iter().any(|(k, _)| *k == T::Colon));
    }

    #[test]
    fn test_string_interpolation_edge_cases() {
        // Empty hole: diagnosed by the parser.
        let k: Vec<_> = kinds(r#""\()""#).into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            k,
            vec![T::StringStart, T::InterpStart, T::InterpEnd, T::StringEnd]
        );

        // Consecutive holes.
        let k: Vec<_> = kinds(r#""\(a)\(b)""#).into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            k,
            vec![
                T::StringStart,
                T::InterpStart,
                T::Identifier,
                T::InterpEnd,
                T::InterpStart,
                T::Identifier,
                T::InterpEnd,
                T::StringEnd,
            ]
        );
    }

    #[test]
    fn test_string_interpolation_with_char_literal() {
        for source in [r#""\(c == ')')""#, r#""\(c == '\n')""#] {
            assert_lossless(source);
            let toks = kinds(source);
            assert!(toks.iter().any(|(k, _)| *k == T::Char), "{source}");
            assert_eq!(toks.last().unwrap().0, T::StringEnd, "{source}");
        }
    }

    #[test]
    fn test_string_interpolation_with_comments() {
        for source in ["\"\\(x // comment\n)\"", r#""\(x /* ) */ + y)""#] {
            assert_lossless(source);
            assert_eq!(kinds(source).last().unwrap().0, T::StringEnd, "{source}");
        }
    }

    #[test]
    fn test_string_after_interpolated_string() {
        let source = r#""\(x)" "y""#;
        assert_lossless(source);
        let toks = kinds(source);
        assert_eq!(toks.last().unwrap(), &(T::String, "\"y\""));
    }

    #[test]
    fn multiline_interpolated_string() {
        let source = "\"\"\"\n  a \\(x) \"quoted\"\n  \"\"\"";
        assert_lossless(source);
        let toks = kinds(source);
        assert_eq!(toks.first().unwrap(), &(T::StringStart, "\"\"\""));
        assert_eq!(toks.last().unwrap(), &(T::StringEnd, "\"\"\""));
    }

    #[test]
    fn unterminated_single_line_string_stops_at_the_line_break() {
        let source = "let s = \"abc\nlet t = 1";
        assert_lossless(source);
        let toks = kinds(source);
        assert_eq!(toks[3], (T::String, "\"abc"));
        // The next line is code again.
        assert_eq!(toks[4].0, T::Let);
    }

    #[test]
    fn single_line_string_may_span_lines_when_closed() {
        let source = "\"abc\ndef\"";
        assert_eq!(kinds(source), vec![(T::String, source)]);
    }
}
