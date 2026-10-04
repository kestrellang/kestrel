//! Parse diagnostics: every syntax error has a stable code and a handwritten
//! message. Codes live in the `E8xx` "syntax" family (see
//! `docs/error-codes.md`); this file is the only place they are assigned.

use kestrel_syntax_tree::SyntaxKind;

/// Syntax-error codes. One per *kind* of mistake, so a test can pin the
/// family with `// ERROR(E801)` regardless of wording.
pub mod codes {
    /// A specific token was required and something else was found.
    pub const UNEXPECTED_TOKEN: &str = "E800";
    /// A statement is missing its terminating `;`.
    pub const MISSING_SEMICOLON: &str = "E801";
    /// A `(`, `[` or `{` was never closed.
    pub const UNCLOSED_DELIMITER: &str = "E802";
    /// An expression was required.
    pub const EXPECTED_EXPRESSION: &str = "E803";
    /// `.` with no member name after it.
    pub const EXPECTED_MEMBER_NAME: &str = "E804";
    /// `throw` with no value.
    pub const THROW_WITHOUT_VALUE: &str = "E805";
    /// A top-level or type-body declaration was required.
    pub const EXPECTED_DECLARATION: &str = "E806";
    /// A type was required.
    pub const EXPECTED_TYPE: &str = "E807";
    /// A pattern was required.
    pub const EXPECTED_PATTERN: &str = "E808";
    /// A name (identifier) was required.
    pub const EXPECTED_NAME: &str = "E809";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntaxError {
    pub code: &'static str,
    pub message: String,
    pub range: std::ops::Range<usize>,
}

impl SyntaxError {
    pub fn new(code: &'static str, message: impl Into<String>, range: std::ops::Range<usize>) -> Self {
        Self {
            code,
            message: message.into(),
            range,
        }
    }

    /// "expected `X`, found Y" — or, for a closing delimiter or `;`, the
    /// dedicated code.
    pub fn expected_tokens(
        kinds: &[SyntaxKind],
        found: Option<SyntaxKind>,
        range: std::ops::Range<usize>,
    ) -> Self {
        let code = match kinds {
            [SyntaxKind::Semicolon] => codes::MISSING_SEMICOLON,
            [SyntaxKind::RParen | SyntaxKind::RBrace | SyntaxKind::RBracket] => {
                codes::UNCLOSED_DELIMITER
            },
            [SyntaxKind::Identifier] => codes::EXPECTED_NAME,
            _ => codes::UNEXPECTED_TOKEN,
        };
        let expected = kinds
            .iter()
            .map(|k| format!("`{}`", kind_spelling(*k)))
            .collect::<Vec<_>>();
        let expected = match expected.as_slice() {
            [one] if *kinds == [SyntaxKind::Identifier] => {
                let _ = one;
                "a name".to_string()
            },
            [one] => one.clone(),
            [rest @ .., last] => format!("{} or {}", rest.join(", "), last),
            [] => "something else".to_string(),
        };
        Self::new(code, format!("expected {expected}, found {}", describe(found)), range)
    }

    /// "expected <what>, found Y".
    pub fn expected_what(
        what: &str,
        found: Option<SyntaxKind>,
        range: std::ops::Range<usize>,
    ) -> Self {
        let code = match what {
            "expression" => codes::EXPECTED_EXPRESSION,
            "declaration" | "member declaration" => codes::EXPECTED_DECLARATION,
            "type" => codes::EXPECTED_TYPE,
            "pattern" => codes::EXPECTED_PATTERN,
            _ => codes::UNEXPECTED_TOKEN,
        };
        Self::new(code, format!("expected {what}, found {}", describe(found)), range)
    }
}

/// How a token is named in a diagnostic: `'('`, `'func'`, `identifier`,
/// `end of file`.
pub(crate) fn describe(kind: Option<SyntaxKind>) -> String {
    let Some(kind) = kind else {
        return "end of file".to_string();
    };
    match kind {
        SyntaxKind::Identifier => "identifier".to_string(),
        SyntaxKind::String => "string".to_string(),
        SyntaxKind::RawString => "raw string".to_string(),
        SyntaxKind::Char => "character".to_string(),
        SyntaxKind::Integer => "integer".to_string(),
        SyntaxKind::Float => "float".to_string(),
        SyntaxKind::Boolean => "boolean".to_string(),
        other => format!("'{}'", kind_spelling(other)),
    }
}

/// The fixed spelling of a keyword or punctuation kind.
pub(crate) fn kind_spelling(kind: SyntaxKind) -> &'static str {
    use SyntaxKind as K;
    match kind {
        K::Identifier => "identifier",
        K::String => "string",
        K::RawString => "raw string",
        K::Char => "character",
        K::Integer => "integer",
        K::Float => "float",
        K::Boolean => "boolean",
        K::Null => "null",
        K::Some => "some",
        K::As => "as",
        K::Break => "break",
        K::Case => "case",
        K::Consuming => "consuming",
        K::Continue => "continue",
        K::Deinit => "deinit",
        K::Else => "else",
        K::Enum => "enum",
        K::Extend => "extend",
        K::For => "for",
        K::Fileprivate => "fileprivate",
        K::Func => "func",
        K::If => "if",
        K::Import => "import",
        K::Indirect => "indirect",
        K::Loop => "loop",
        K::Init => "init",
        K::Internal => "internal",
        K::Let => "let",
        K::Module => "module",
        K::Mutating => "mutating",
        K::Private => "private",
        K::Protocol => "protocol",
        K::Public => "public",
        K::Return => "return",
        K::Throw => "throw",
        K::Try => "try",
        K::Throws => "throws",
        K::Static => "static",
        K::Struct => "struct",
        K::Type => "type",
        K::Var => "var",
        K::Where => "where",
        K::While => "while",
        K::In => "in",
        K::Match => "match",
        K::Guard => "guard",
        K::Get => "get",
        K::Set => "set",
        K::Subscript => "subscript",
        K::And => "and",
        K::Not => "not",
        K::Or => "or",
        K::LParen => "(",
        K::RParen => ")",
        K::LBrace => "{",
        K::RBrace => "}",
        K::LBracket => "[",
        K::RBracket => "]",
        K::Semicolon => ";",
        K::Comma => ",",
        K::Dot => ".",
        K::Colon => ":",
        K::Question => "?",
        K::Bang => "!",
        K::Underscore => "_",
        K::DotDotEquals => "..=",
        K::DotDotLess => "..<",
        K::DotDot => "..",
        K::LessLessEquals => "<<=",
        K::GreaterGreaterEquals => ">>=",
        K::LessLess => "<<",
        K::GreaterGreater => ">>",
        K::LessEquals => "<=",
        K::GreaterEquals => ">=",
        K::EqualsEquals => "==",
        K::BangEquals => "!=",
        K::QuestionQuestion => "??",
        K::Arrow => "->",
        K::FatArrow => "=>",
        K::PlusEquals => "+=",
        K::MinusEquals => "-=",
        K::StarEquals => "*=",
        K::SlashEquals => "/=",
        K::PercentEquals => "%=",
        K::AmpersandEquals => "&=",
        K::PipeEquals => "|=",
        K::CaretEquals => "^=",
        K::Equals => "=",
        K::Plus => "+",
        K::Minus => "-",
        K::Star => "*",
        K::Slash => "/",
        K::Percent => "%",
        K::Ampersand => "&",
        K::Pipe => "|",
        K::Caret => "^",
        K::Less => "<",
        K::Greater => ">",
        K::At => "@",
        _ => "token",
    }
}
