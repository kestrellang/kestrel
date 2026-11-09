module js

// Tokens produced by the lexer.

/// What a token is. Keywords lex as `Name` and the parser checks their
/// text: most of JavaScript's keywords are contextual (`let`, `async`,
/// `of`, `get`, …), so one rule for all of them is simpler.
@derive(Equatable)
enum TokenKind {
    case End
    /// An identifier or keyword. `value` is its name with escapes decoded.
    case Name
    /// `#name`. `value` is the name without the `#`.
    case PrivateName
    /// Punctuation or an operator. `value` is its text.
    case Punct
    /// A numeric literal. `number` is its value.
    case Number
    /// A BigInt literal. `value` is its digits in base 10.
    case BigInt
    /// A string literal. `value` is the decoded string.
    case String
    /// A whole template with no substitutions: `` `abc` ``.
    case Template
    /// A template's first part, up to `${`.
    case TemplateHead
    /// The part between two substitutions, `}…${`.
    case TemplateMiddle
    /// The last part, `}…` `` ` ``.
    case TemplateTail
    /// A regular expression literal. `value` is the body, `flags` the flags.
    case Regex
}

struct Token {
    var kind: TokenKind
    /// Start and end, as code point offsets into the source.
    var start: Int64
    var end: Int64
    /// A line terminator came between the previous token and this one.
    /// Needed for automatic semicolon insertion and restricted productions.
    var newlineBefore: Bool
    var value: String
    var number: Float64
    /// Regex flags.
    var flags: String
    /// The name was written with a `\u` escape, so it can't act as a keyword.
    var escaped: Bool
    /// A legacy octal literal (`010`, `08`) or an octal escape in a string
    /// (`"\1"`), both errors in strict mode.
    var legacyOctal: Bool
    /// The template part has an invalid escape: allowed in tagged templates
    /// (the cooked value is undefined), an error everywhere else.
    var badEscape: Bool
    /// A template part's raw text.
    var raw: String

    init(kind kind: TokenKind, start start: Int64) {
        self.kind = kind;
        self.start = start;
        self.end = start;
        self.newlineBefore = false;
        self.value = "";
        self.number = 0.0;
        self.flags = "";
        self.escaped = false;
        self.legacyOctal = false;
        self.badEscape = false;
        self.raw = "";
    }

    /// This is the punctuator `text`.
    func isPunct(text: String) -> Bool {
        self.kind == TokenKind.Punct and self.value == text
    }

    /// This is the name `text`, written without escapes, so it reads as
    /// that keyword.
    func isKeyword(text: String) -> Bool {
        self.kind == TokenKind.Name and not self.escaped and self.value == text
    }

    /// This is the name `text`, escaped or not.
    func isName(text: String) -> Bool {
        self.kind == TokenKind.Name and self.value == text
    }
}
