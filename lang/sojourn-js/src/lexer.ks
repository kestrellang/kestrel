module js

// The lexer turns source text into tokens on demand.
//
// JavaScript can't be tokenized without knowing the grammar: `/` starts a
// regex in some places and divides in others, and `}` either closes a
// block or resumes a template. The lexer always reads `/` and `}` as
// punctuation, and the parser asks it to read the same spot again
// (`rescanRegex`, `rescanTemplate`) when the grammar wants the other
// meaning.

/// A syntax error or an early error, with the code point offset where it
/// was found.
struct SyntaxError {
    var message: String
    var position: Int64
    /// An early error: the text matches the grammar but breaks a static
    /// rule (a duplicate `let`, `break` outside a loop, …).
    var early: Bool
}

func syntaxError(message: String, at position: Int64) -> SyntaxError {
    SyntaxError(message: message, position: position, early: false)
}

func earlyError(message: String, at position: Int64) -> SyntaxError {
    SyntaxError(message: message, position: position, early: true)
}

/// Every punctuator, longest first within each leading character, so the
/// first match is the longest.
let punctuators: [String] = [
    ">>>=", "...", "===", "!==", "**=", "<<=", ">>=", ">>>", "&&=", "||=", "??=",
    "=>", "==", "!=", "<=", ">=", "&&", "||", "??", "?.", "++", "--", "+=", "-=",
    "*=", "/=", "%=", "&=", "|=", "^=", "<<", ">>", "**",
    "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+", "-", "*", "/", "%",
    "&", "|", "^", "!", "~", "?", ":", "=", ".", "@"
];

struct Lexer {
    /// The source as code points.
    var src: Array[Int64]
    var pos: Int64
    /// Treat `<!--` and a line-leading `-->` as line comments, as scripts
    /// (not modules) do for web compatibility (Annex B).
    var htmlComments: Bool

    init(text text: String) {
        var cps: [Int64] = [];
        for c in text.chars {
            cps.append(Int64(from: c.value()));
        }
        self.src = cps;
        self.pos = 0;
        self.htmlComments = false;
    }

    /// The code point at `i`, or -1 past the end.
    func at(i: Int64) -> Int64 {
        if i < self.src.count { self.src(i) } else { -1 }
    }

    func peek() -> Int64 { self.at(self.pos) }

    func peekAt(offset: Int64) -> Int64 { self.at(self.pos + offset) }

    /// The source text from `start` to `end`.
    func text(start: Int64, end: Int64) -> String {
        var out = "";
        for i in start..<end { appendCodePoint(out, self.src(i)); }
        out
    }

    /// Skips a `#!` line at the very start of the source.
    mutating func skipHashbang() {
        if self.pos == 0 and self.peek() == 35 and self.peekAt(1) == 33 {
            while self.pos < self.src.count and not isLineTerminator(self.peek()) {
                self.pos = self.pos + 1;
            }
        }
    }

    /// Skips whitespace and comments. Returns whether a line terminator
    /// was crossed (a multi-line comment containing one counts).
    mutating func skipTrivia() -> Result[Bool, SyntaxError] {
        var newline = false;
        let atInputStart = self.pos == 0;
        while self.pos < self.src.count {
            let c = self.peek();
            let htmlOpen = c == 60 and self.peekAt(1) == 33 and self.peekAt(2) == 45 and self.peekAt(3) == 45;
            let htmlClose = c == 45 and self.peekAt(1) == 45 and self.peekAt(2) == 62 and (newline or atInputStart);
            if self.htmlComments and (htmlOpen or htmlClose) {
                while self.pos < self.src.count and not isLineTerminator(self.peek()) {
                    self.pos = self.pos + 1;
                }
            } else if isWhitespace(c) {
                self.pos = self.pos + 1;
            } else if isLineTerminator(c) {
                self.pos = self.pos + 1;
                newline = true;
            } else if c == 47 and self.peekAt(1) == 47 {
                // `// …` to the end of the line.
                self.pos = self.pos + 2;
                while self.pos < self.src.count and not isLineTerminator(self.peek()) {
                    self.pos = self.pos + 1;
                }
            } else if c == 47 and self.peekAt(1) == 42 {
                // `/* … */`.
                let start = self.pos;
                self.pos = self.pos + 2;
                var closed = false;
                while self.pos < self.src.count {
                    let d = self.peek();
                    if d == 42 and self.peekAt(1) == 47 {
                        self.pos = self.pos + 2;
                        closed = true;
                        break;
                    }
                    if isLineTerminator(d) { newline = true; }
                    self.pos = self.pos + 1;
                }
                if not closed { throw syntaxError("unterminated comment", at: start); }
            } else {
                break;
            }
        }
        .Ok(newline)
    }

    /// Reads the next token, treating `/` as division and `}` as a brace.
    mutating func next() -> Result[Token, SyntaxError] {
        let newline = try self.skipTrivia();
        var tok = try self.scan();
        tok.newlineBefore = newline;
        .Ok(tok)
    }

    mutating func scan() -> Result[Token, SyntaxError] {
        let start = self.pos;
        let c = self.peek();
        if c < 0 {
            return .Ok(Token(kind: .End, start: start));
        }
        if isIdStart(c) or c == 92 {
            return self.scanName(start, .Name);
        }
        if c == 35 {
            // `#name`
            self.pos = self.pos + 1;
            let d = self.peek();
            if not (isIdStart(d) or d == 92) { throw syntaxError("unexpected '#'", at: start); }
            return self.scanName(start, .PrivateName);
        }
        if isDecimalDigit(c) or (c == 46 and isDecimalDigit(self.peekAt(1))) {
            return self.scanNumber(start);
        }
        if c == 34 or c == 39 {
            return self.scanString(start, c);
        }
        if c == 96 {
            self.pos = self.pos + 1;
            return self.scanTemplate(start, true);
        }
        // `?.` followed by a digit is `?` then a number: `a?.5:b`.
        if c == 63 and self.peekAt(1) == 46 and isDecimalDigit(self.peekAt(2)) {
            self.pos = self.pos + 1;
            var tok = Token(kind: .Punct, start: start);
            tok.value = "?";
            tok.end = self.pos;
            return .Ok(tok);
        }
        for p in punctuators {
            if self.startsWith(p) {
                self.pos = self.pos + p.bytes.count;
                var tok = Token(kind: .Punct, start: start);
                tok.value = p;
                tok.end = self.pos;
                return .Ok(tok);
            }
        }
        throw syntaxError("unexpected character", at: start)
    }

    /// The source at `pos` starts with ASCII text `p`.
    func startsWith(p: String) -> Bool {
        var i = self.pos;
        for b in p.bytes {
            if self.at(i) != Int64(from: b) { return false; }
            i = i + 1;
        }
        true
    }

    // ------------------------------------------------------------------
    // Names
    // ------------------------------------------------------------------

    /// Reads an identifier at `pos`, decoding `\u` escapes.
    mutating func scanName(start: Int64, kind: TokenKind) -> Result[Token, SyntaxError] {
        var tok = Token(kind: kind, start: start);
        var name = "";
        var first = true;
        while true {
            let c = self.peek();
            if c == 92 {
                let at = self.pos;
                if self.peekAt(1) != 117 { throw syntaxError("invalid escape in identifier", at: at); }
                self.pos = self.pos + 2;
                let cp = try self.scanUnicodeEscapeBody(at);
                let ok = if first { isIdStart(cp) } else { isIdPart(cp) };
                if not ok { throw syntaxError("invalid character in identifier", at: at); }
                appendCodePoint(name, cp);
                tok.escaped = true;
            } else if (first and isIdStart(c)) or (not first and isIdPart(c)) {
                appendCodePoint(name, c);
                self.pos = self.pos + 1;
            } else {
                break;
            }
            first = false;
        }
        tok.value = name;
        tok.end = self.pos;
        .Ok(tok)
    }

    /// Reads the part of a `\u` escape after `\u`: four hex digits or
    /// `{…}`. Returns the code point.
    mutating func scanUnicodeEscapeBody(at: Int64) -> Result[Int64, SyntaxError] {
        if self.peek() == 123 {
            self.pos = self.pos + 1;
            var value: Int64 = 0;
            var digits = 0;
            while self.peek() != 125 {
                let h = hexValue(self.peek());
                if h < 0 { throw syntaxError("invalid Unicode escape", at: at); }
                value = value * 16 + h;
                if value > 0x10FFFF { throw syntaxError("Unicode escape out of range", at: at); }
                digits = digits + 1;
                self.pos = self.pos + 1;
            }
            if digits == 0 { throw syntaxError("invalid Unicode escape", at: at); }
            self.pos = self.pos + 1;
            return .Ok(value);
        }
        var value: Int64 = 0;
        for _ in 0..<4 {
            let h = hexValue(self.peek());
            if h < 0 { throw syntaxError("invalid Unicode escape", at: at); }
            value = value * 16 + h;
            self.pos = self.pos + 1;
        }
        .Ok(value)
    }

    // ------------------------------------------------------------------
    // Numbers
    // ------------------------------------------------------------------

    mutating func scanNumber(start: Int64) -> Result[Token, SyntaxError] {
        var tok = Token(kind: .Number, start: start);
        let c = self.peek();
        let next = self.peekAt(1);
        if c == 48 and (next == 120 or next == 88 or next == 111 or next == 79 or next == 98 or next == 66) {
            let base: Int64 = if next == 120 or next == 88 { 16 } else if next == 111 or next == 79 { 8 } else { 2 };
            self.pos = self.pos + 2;
            let digits = try self.scanDigits(base, false);
            if digits.isEmpty { throw syntaxError("missing digits after base prefix", at: start); }
            var value: Float64 = 0.0;
            for d in digits.chars {
                value = value * Float64(from: base) + Float64(from: hexValue(Int64(from: d.value())));
            }
            tok.number = value;
            if self.peek() == 110 {
                self.pos = self.pos + 1;
                tok.kind = .BigInt;
                tok.value = bigIntDecimal(digits, base);
            }
            return self.finishNumber(tok);
        }
        if c == 48 and isDecimalDigit(next) {
            // Legacy octal (`017`), or a decimal with a leading zero (`019`).
            tok.legacyOctal = true;
            self.pos = self.pos + 1;
            var digits = "";
            var octal = true;
            while isDecimalDigit(self.peek()) {
                if self.peek() >= 56 { octal = false; }
                appendCodePoint(digits, self.peek());
                self.pos = self.pos + 1;
            }
            if self.peek() == 95 { throw syntaxError("numeric separator after a leading zero", at: self.pos); }
            if octal {
                var value: Float64 = 0.0;
                for d in digits.chars {
                    value = value * 8.0 + Float64(from: Int64(from: d.value()) - 48);
                }
                tok.number = value;
                return self.finishNumber(tok);
            }
            // `08.5` and `09e1` are decimal.
            var text = "0";
            text.append(digits);
            try self.scanFraction(text);
            tok.number = Float64(parsing: text) ?? 0.0;
            return self.finishNumber(tok);
        }
        var text = try self.scanDigits(10, false);
        if text.isEmpty { text = "0"; }
        if self.peek() == 110 and self.at(start) != 46 {
            self.pos = self.pos + 1;
            tok.kind = .BigInt;
            tok.value = bigIntDecimal(text, 10);
            return self.finishNumber(tok);
        }
        try self.scanFraction(text);
        tok.number = Float64(parsing: text) ?? 0.0;
        self.finishNumber(tok)
    }

    /// Reads `.digits` and `e±digits` after an integer part into `text`.
    mutating func scanFraction(mutating text: String) -> Result[(), SyntaxError] {
        if self.peek() == 46 {
            self.pos = self.pos + 1;
            text.append(".");
            if self.peek() == 95 { throw syntaxError("numeric separator after '.'", at: self.pos); }
            let frac = try self.scanDigits(10, true);
            text.append(if frac.isEmpty { "0" } else { frac });
        }
        let e = self.peek();
        if e == 101 or e == 69 {
            self.pos = self.pos + 1;
            text.append("e");
            let sign = self.peek();
            if sign == 43 or sign == 45 {
                appendCodePoint(text, sign);
                self.pos = self.pos + 1;
            }
            let exp = try self.scanDigits(10, false);
            if exp.isEmpty { throw syntaxError("missing exponent", at: self.pos); }
            text.append(exp);
        }
        .Ok(())
    }

    /// Reads digits in `base`, allowing single `_` separators between
    /// digits. Returns the digits without separators.
    mutating func scanDigits(base: Int64, afterDot: Bool) -> Result[String, SyntaxError] {
        var out = "";
        var lastWasDigit = false;
        var sawSeparator = false;
        while true {
            let c = self.peek();
            if c == 95 {
                if not lastWasDigit { throw syntaxError("misplaced numeric separator", at: self.pos); }
                lastWasDigit = false;
                sawSeparator = true;
                self.pos = self.pos + 1;
                continue;
            }
            let v = hexValue(c);
            if v < 0 or v >= base { break; }
            appendCodePoint(out, c);
            lastWasDigit = true;
            self.pos = self.pos + 1;
        }
        if sawSeparator and not lastWasDigit {
            throw syntaxError("misplaced numeric separator", at: self.pos - 1);
        }
        .Ok(out)
    }

    /// A number can't run straight into an identifier or another digit.
    mutating func finishNumber(consuming tok: Token) -> Result[Token, SyntaxError] {
        let c = self.peek();
        if isIdStart(c) or isDecimalDigit(c) or c == 92 {
            throw syntaxError("identifier starts immediately after a number", at: self.pos);
        }
        tok.end = self.pos;
        .Ok(tok)
    }

    // ------------------------------------------------------------------
    // Strings
    // ------------------------------------------------------------------

    mutating func scanString(start: Int64, quote: Int64) -> Result[Token, SyntaxError] {
        var tok = Token(kind: .String, start: start);
        self.pos = self.pos + 1;
        var out = "";
        while true {
            let c = self.peek();
            if c < 0 or c == 10 or c == 13 { throw syntaxError("unterminated string", at: start); }
            if c == quote {
                self.pos = self.pos + 1;
                break;
            }
            if c == 92 {
                let esc = try self.scanEscape(false);
                if esc.octal { tok.legacyOctal = true; }
                if esc.codePoint >= 0 { appendCodePoint(out, esc.codePoint); }
                continue;
            }
            appendCodePoint(out, c);
            self.pos = self.pos + 1;
        }
        tok.value = out;
        tok.end = self.pos;
        .Ok(tok)
    }

    /// Reads an escape sequence at `\`. Line continuations yield code
    /// point -1 (nothing). In a template, legacy octal and `\8` escapes
    /// are invalid rather than legacy.
    mutating func scanEscape(inTemplate: Bool) -> Result[Escape, SyntaxError] {
        let at = self.pos;
        self.pos = self.pos + 1;
        let c = self.peek();
        if c < 0 { throw syntaxError("unterminated escape", at: at); }
        self.pos = self.pos + 1;
        if c == 13 {
            if self.peek() == 10 { self.pos = self.pos + 1; }
            return .Ok(Escape(codePoint: -1, octal: false, invalid: false));
        }
        if isLineTerminator(c) { return .Ok(Escape(codePoint: -1, octal: false, invalid: false)); }
        if c == 110 { return .Ok(Escape(codePoint: 10, octal: false, invalid: false)); }
        if c == 116 { return .Ok(Escape(codePoint: 9, octal: false, invalid: false)); }
        if c == 114 { return .Ok(Escape(codePoint: 13, octal: false, invalid: false)); }
        if c == 98 { return .Ok(Escape(codePoint: 8, octal: false, invalid: false)); }
        if c == 102 { return .Ok(Escape(codePoint: 12, octal: false, invalid: false)); }
        if c == 118 { return .Ok(Escape(codePoint: 11, octal: false, invalid: false)); }
        if c == 120 {
            let h1 = hexValue(self.peek());
            let h2 = hexValue(self.peekAt(1));
            if h1 < 0 or h2 < 0 {
                if inTemplate { return .Ok(Escape(codePoint: -1, octal: false, invalid: true)); }
                throw syntaxError("invalid hexadecimal escape", at: at);
            }
            self.pos = self.pos + 2;
            return .Ok(Escape(codePoint: h1 * 16 + h2, octal: false, invalid: false));
        }
        if c == 117 {
            if inTemplate {
                // An invalid escape is not an error in a tagged template, so
                // try it and report rather than throw.
                let save = self.pos;
                match self.scanUnicodeEscapeBody(at) {
                    .Ok(cp) => return .Ok(Escape(codePoint: self.joinSurrogates(cp), octal: false, invalid: false)),
                    .Err(_) => {
                        self.pos = save;
                        return .Ok(Escape(codePoint: -1, octal: false, invalid: true));
                    }
                }
            }
            let cp = try self.scanUnicodeEscapeBody(at);
            return .Ok(Escape(codePoint: self.joinSurrogates(cp), octal: false, invalid: false));
        }
        if c >= 48 and c <= 55 {
            // `\0` not followed by a digit is NUL; anything else is octal.
            if c == 48 and not isDecimalDigit(self.peek()) {
                return .Ok(Escape(codePoint: 0, octal: false, invalid: false));
            }
            if inTemplate { return .Ok(Escape(codePoint: -1, octal: false, invalid: true)); }
            var value = c - 48;
            let maxDigits = if c <= 51 { 3 } else { 2 };
            var n = 1;
            while n < maxDigits and self.peek() >= 48 and self.peek() <= 55 {
                value = value * 8 + self.peek() - 48;
                self.pos = self.pos + 1;
                n = n + 1;
            }
            return .Ok(Escape(codePoint: value, octal: true, invalid: false));
        }
        if c == 56 or c == 57 {
            if inTemplate { return .Ok(Escape(codePoint: -1, octal: false, invalid: true)); }
            return .Ok(Escape(codePoint: c, octal: true, invalid: false));
        }
        .Ok(Escape(codePoint: c, octal: false, invalid: false))
    }

    /// After reading a high surrogate escape, a following `\uDC00`–`\uDFFF`
    /// escape joins it into one code point, as it does in a JS string.
    mutating func joinSurrogates(high: Int64) -> Int64 {
        if high < 0xD800 or high > 0xDBFF { return high; }
        if self.peek() != 92 or self.peekAt(1) != 117 { return high; }
        let save = self.pos;
        self.pos = self.pos + 2;
        match self.scanUnicodeEscapeBody(save) {
            .Ok(low) => {
                if low >= 0xDC00 and low <= 0xDFFF {
                    return 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                }
            },
            .Err(_) => {}
        }
        self.pos = save;
        high
    }

    // ------------------------------------------------------------------
    // Templates
    // ------------------------------------------------------------------

    /// Reads template characters up to `` ` `` or `${`. `pos` is just past
    /// the opening `` ` `` or `}`.
    mutating func scanTemplate(start: Int64, head: Bool) -> Result[Token, SyntaxError] {
        var tok = Token(kind: .Template, start: start);
        var cooked = "";
        var raw = "";
        while true {
            let c = self.peek();
            if c < 0 { throw syntaxError("unterminated template", at: start); }
            if c == 96 {
                self.pos = self.pos + 1;
                tok.kind = if head { TokenKind.Template } else { TokenKind.TemplateTail };
                break;
            }
            if c == 36 and self.peekAt(1) == 123 {
                self.pos = self.pos + 2;
                tok.kind = if head { TokenKind.TemplateHead } else { TokenKind.TemplateMiddle };
                break;
            }
            if c == 92 {
                let escStart = self.pos;
                let esc = try self.scanEscape(true);
                if esc.invalid { tok.badEscape = true; }
                if esc.codePoint >= 0 { appendCodePoint(cooked, esc.codePoint); }
                // Raw text keeps the escape as written, with CRLF as LF.
                var i = escStart;
                while i < self.pos {
                    let r = self.src(i);
                    if r == 13 {
                        raw.append("\n");
                        if self.at(i + 1) == 10 { i = i + 1; }
                    } else {
                        appendCodePoint(raw, r);
                    }
                    i = i + 1;
                }
                continue;
            }
            if c == 13 {
                // CR and CRLF both read as LF.
                self.pos = self.pos + 1;
                if self.peek() == 10 { self.pos = self.pos + 1; }
                cooked.append("\n");
                raw.append("\n");
                continue;
            }
            appendCodePoint(cooked, c);
            appendCodePoint(raw, c);
            self.pos = self.pos + 1;
        }
        tok.value = cooked;
        tok.raw = raw;
        tok.end = self.pos;
        .Ok(tok)
    }

    /// Reads a template continuation at `}` (`tok`), which the parser saw as
    /// a brace closing a `${…}` substitution.
    mutating func rescanTemplate(tok: Token) -> Result[Token, SyntaxError] {
        self.pos = tok.start + 1;
        var t = try self.scanTemplate(tok.start, false);
        t.newlineBefore = tok.newlineBefore;
        .Ok(t)
    }

    // ------------------------------------------------------------------
    // Regular expressions
    // ------------------------------------------------------------------

    /// Reads a regex literal at `/` or `/=` (`tok`), which the parser saw
    /// as division.
    mutating func rescanRegex(tok: Token) -> Result[Token, SyntaxError] {
        let start = tok.start;
        self.pos = start + 1;
        var inClass = false;
        while true {
            let c = self.peek();
            if c < 0 or isLineTerminator(c) { throw syntaxError("unterminated regular expression", at: start); }
            self.pos = self.pos + 1;
            if c == 92 {
                let d = self.peek();
                if d < 0 or isLineTerminator(d) { throw syntaxError("unterminated regular expression", at: start); }
                self.pos = self.pos + 1;
            } else if c == 91 {
                inClass = true;
            } else if c == 93 {
                inClass = false;
            } else if c == 47 and not inClass {
                break;
            }
        }
        let bodyEnd = self.pos - 1;
        let flagStart = self.pos;
        while isIdPart(self.peek()) or self.peek() == 92 {
            if self.peek() == 92 { throw syntaxError("escape in regular expression flags", at: self.pos); }
            self.pos = self.pos + 1;
        }
        var r = Token(kind: .Regex, start: start);
        r.newlineBefore = tok.newlineBefore;
        r.value = self.text(start + 1, bodyEnd);
        r.flags = self.text(flagStart, self.pos);
        r.end = self.pos;
        try validateRegexFlags(r.flags, start);
        .Ok(r)
    }
}

/// One decoded escape sequence.
struct Escape {
    /// The code point, or -1 for none (a line continuation, or invalid).
    var codePoint: Int64
    /// A legacy octal or `\8`/`\9` escape: an error in strict mode.
    var octal: Bool
    /// Not a valid escape (only reported inside templates).
    var invalid: Bool
}

/// Flags are from `dgimsuvy`, each at most once, and not both `u` and `v`.
func validateRegexFlags(flags: String, at: Int64) -> Result[(), SyntaxError] {
    var seen = "";
    for c in flags.chars {
        let s = c.toString();
        if not "dgimsuvy".contains(s) or seen.contains(s) {
            throw syntaxError("invalid regular expression flags", at: at);
        }
        seen.append(s);
    }
    if seen.contains("u") and seen.contains("v") {
        throw syntaxError("invalid regular expression flags", at: at);
    }
    .Ok(())
}

/// The decimal digits of a BigInt written in `base`, so `0x10n` and `16n`
/// read the same.
func bigIntDecimal(digits: String, base: Int64) -> String {
    // Little-endian base-10 digits.
    var acc: [Int64] = [0];
    for d in digits.chars {
        var carry = hexValue(Int64(from: d.value()));
        for i in 0..<acc.count {
            let v = acc(i) * base + carry;
            acc(i) = v % 10;
            carry = v / 10;
        }
        while carry > 0 {
            acc.append(carry % 10);
            carry = carry / 10;
        }
    }
    while acc.count > 1 and acc(acc.count - 1) == 0 { let _ = acc.pop(); }
    var out = "";
    var i = acc.count - 1;
    while i >= 0 {
        appendCodePoint(out, 48 + acc(i));
        i = i - 1;
    }
    out
}
