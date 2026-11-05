module regex

/// A compiled regular expression.
///
/// Syntax: literals, `.`, classes (`[a-z]`, `[^0-9]`, `[[:alpha:]]`),
/// `\d \w \s` and their negations (ASCII), anchors `^ $ \A \z \b \B`,
/// groups `(…)`, `(?:…)`, `(?<name>…)`, alternation `|`, repetition
/// `* + ? {n} {n,} {n,m} {,m}` with lazy `?` forms, and flags `(?i)`, `(?m)`,
/// `(?s)` (also scoped, as in `(?i:…)`). There are no backreferences or
/// lookaround: they would rule out linear-time matching.
///
/// # Examples
///
/// ```
/// let word = try Regex.compile("\\b\\w+\\b");
/// for m in word.matches(in: "two words") { print(m.text); }
///
/// let swap = try Regex.compile("(\\w+) (\\w+)");
/// swap.replacingAll(in: "hello world", with: "$2 $1");   // "world hello"
/// ```
public struct Regex {
    /// The source pattern.
    public let pattern: String;
    let program: Program;
    /// Capture names by group index; "" for unnamed groups and group 0.
    let names: Array[String];

    init(pattern pattern: String, program program: Program, names names: Array[String]) {
        self.pattern = pattern;
        self.program = program;
        self.names = names;
    }

    /// Parses and compiles a pattern.
    public static func compile(pattern: String) -> Result[Regex, RegexError] {
        var parser = Parser(pattern: pattern);
        let root = try parser.parse();
        let program = try compileProgram(parser, root);
        .Ok(Regex(pattern: pattern, program: program, names: parser.names))
    }

    /// Capture groups in the pattern, not counting the whole match.
    public var captureCount: Int64 { self.names.count - 1 }

    /// Each capture group's name, "" for unnamed ones, in group order
    /// starting from group 1.
    public var captureNames: Array[String] {
        var out: [String] = [];
        for i in 1..<self.names.count { out.append(self.names(i)); }
        out
    }

    /// True if the pattern matches anywhere in `text`.
    public func isMatch(in text: String) -> Bool {
        var searcher = Searcher(program: self.program, text: text.toBytes());
        searcher.search(0, true).isSome
    }

    /// The leftmost match.
    public func firstMatch(in text: String) -> Match? {
        let bytes = text.toBytes();
        var searcher = Searcher(program: self.program, text: bytes);
        guard let .Some(slots) = searcher.search(0, false) else { return .None; }
        .Some(makeMatch(bytes, slots(0), slots(1)))
    }

    /// Every non-overlapping match, left to right.
    public func matches(in text: String) -> Array[Match] {
        let bytes = text.toBytes();
        var out: [Match] = [];
        for slots in self.searchAll(bytes, -1) {
            out.append(makeMatch(bytes, slots(0), slots(1)));
        }
        out
    }

    /// The leftmost match with its capture groups.
    public func captures(in text: String) -> Captures? {
        let bytes = text.toBytes();
        var searcher = Searcher(program: self.program, text: bytes);
        guard let .Some(slots) = searcher.search(0, false) else { return .None; }
        .Some(Captures(text: bytes, slots: slots, names: self.names))
    }

    /// Every non-overlapping match with its capture groups.
    public func allCaptures(in text: String) -> Array[Captures] {
        let bytes = text.toBytes();
        var out: [Captures] = [];
        for slots in self.searchAll(bytes, -1) {
            out.append(Captures(text: bytes, slots: slots, names: self.names));
        }
        out
    }

    /// Replaces the first match. In `template`, `$1` or `${1}` is a group,
    /// `${name}` or `$name` a named group, and `$$` a dollar sign.
    public func replacingFirst(in text: String, with template: String) -> String {
        self.replace(text, template, 1)
    }

    /// Replaces every match; see `replacingFirst` for the template.
    public func replacingAll(in text: String, with template: String) -> String {
        self.replace(text, template, -1)
    }

    /// Replaces every match with what `transform` returns for it.
    ///
    /// # Examples
    ///
    /// ```
    /// let shout = try Regex.compile("\\w+");
    /// shout.replacingAll(in: "hi there", using: { (c) in c.whole.text.uppercased() });
    /// ```
    public func replacingAll(in text: String, using transform: (Captures) -> String) -> String {
        let bytes = text.toBytes();
        var out: [UInt8] = [];
        var last: Int64 = 0;
        for slots in self.searchAll(bytes, -1) {
            appendRange(out, bytes, last, slots(0));
            out.append(contentsOf: transform(Captures(text: bytes, slots: slots, names: self.names)).toBytes());
            last = slots(1);
        }
        appendRange(out, bytes, last, bytes.count);
        String(fromUtf8Lossy: out)
    }

    /// The text between matches.
    ///
    /// # Examples
    ///
    /// ```
    /// try Regex.compile(",\\s*").split("a, b,c");   // ["a", "b", "c"]
    /// ```
    public func split(text: String) -> Array[String] {
        let bytes = text.toBytes();
        var out: [String] = [];
        var last: Int64 = 0;
        for slots in self.searchAll(bytes, -1) {
            out.append(substring(bytes, last, slots(0)));
            last = slots(1);
        }
        out.append(substring(bytes, last, bytes.count));
        out
    }

    /// `text` with every character that has a meaning in patterns escaped,
    /// so it matches literally.
    public static func escape(text: String) -> String {
        var out: [UInt8] = [];
        for b in text.bytes {
            if isMeta(b) { out.append(92); }
            out.append(b);
        }
        String(fromUtf8Lossy: out)
    }

    func replace(text: String, template: String, limit: Int64) -> String {
        let bytes = text.toBytes();
        let parts = parseTemplate(template.toBytes(), self.names);
        var out: [UInt8] = [];
        var last: Int64 = 0;
        for slots in self.searchAll(bytes, limit) {
            appendRange(out, bytes, last, slots(0));
            for part in parts {
                if part.group < 0 {
                    out.append(contentsOf: part.literal);
                } else if part.group < self.names.count and slots(part.group * 2) >= 0 {
                    appendRange(out, bytes, slots(part.group * 2), slots(part.group * 2 + 1));
                }
            }
            last = slots(1);
        }
        appendRange(out, bytes, last, bytes.count);
        String(fromUtf8Lossy: out)
    }

    /// Slots of up to `limit` successive matches (all when -1). After an
    /// empty match the search moves on one character, so it always ends.
    func searchAll(bytes: Array[UInt8], limit: Int64) -> Array[Array[Int64]] {
        var searcher = Searcher(program: self.program, text: bytes);
        var out: [Array[Int64]] = [];
        var start: Int64 = 0;
        var previousEnd: Int64 = -1;
        while start <= bytes.count {
            if limit >= 0 and out.count >= limit { break; }
            guard let .Some(slots) = searcher.search(start, false) else { break; }
            let s = slots(0);
            let e = slots(1);
            if s == e and s == previousEnd {
                // An empty match right after the previous match: skip it.
                if s >= bytes.count { break; }
                start = s + decodeUtf8(bytes, s).1;
                continue;
            }
            out.append(slots);
            previousEnd = e;
            if e > s {
                start = e;
            } else {
                if e >= bytes.count { break; }
                start = e + decodeUtf8(bytes, e).1;
            }
        }
        out
    }
}

/// One match: where it is in the text (byte offsets) and what it says.
public struct Match {
    /// Byte offset of the first matched byte.
    public let start: Int64;
    /// Byte offset just past the match.
    public let end: Int64;
    public let text: String;

    public init(start start: Int64, end end: Int64, text text: String) {
        self.start = start;
        self.end = end;
        self.text = text;
    }

    public var isEmpty: Bool { self.start == self.end }
}

/// A match with its capture groups. Group 0 is the whole match.
///
/// # Examples
///
/// ```
/// let c = try Regex.compile("(?<key>\\w+)=(\\d+)").captures(in: "x=42")?;
/// c.named("key")?.text;   // "x"
/// c.group(2)?.text;       // "42"
/// ```
public struct Captures {
    let text: Array[UInt8];
    let slots: Array[Int64];
    let names: Array[String];

    init(text text: Array[UInt8], slots slots: Array[Int64], names names: Array[String]) {
        self.text = text;
        self.slots = slots;
        self.names = names;
    }

    /// Groups including group 0.
    public var count: Int64 { self.slots.count / 2 }

    /// The whole match.
    public var whole: Match { makeMatch(self.text, self.slots(0), self.slots(1)) }

    /// Group `index`, or `.None` if it didn't take part in the match.
    public func group(index: Int64) -> Match? {
        if index < 0 or index >= self.count { return .None; }
        let s = self.slots(index * 2);
        let e = self.slots(index * 2 + 1);
        if s < 0 or e < 0 { return .None; }
        .Some(makeMatch(self.text, s, e))
    }

    /// The group with this name.
    public func named(name: String) -> Match? {
        for i in 1..<self.names.count {
            if self.names(i) == name { return self.group(i); }
        }
        .None
    }
}

func makeMatch(bytes: Array[UInt8], start: Int64, end: Int64) -> Match {
    Match(start: start, end: end, text: substring(bytes, start, end))
}

func substring(bytes: Array[UInt8], start: Int64, end: Int64) -> String {
    var out: [UInt8] = [];
    appendRange(out, bytes, start, end);
    String(fromUtf8Lossy: out)
}

func appendRange(mutating out: Array[UInt8], bytes: Array[UInt8], start: Int64, end: Int64) {
    var i = start;
    while i < end {
        out.append(bytes(i));
        i = i + 1;
    }
}

func isMeta(b: UInt8) -> Bool {
    // \ . + * ? ( ) | [ ] { } ^ $ # & - ~
    b == 92 or b == 46 or b == 43 or b == 42 or b == 63 or b == 40 or b == 41 or b == 124 or b == 91 or b == 93 or b == 123 or b == 125 or b == 94 or b == 36 or b == 35 or b == 38 or b == 45 or b == 126
}

/// A piece of a replacement template: literal bytes, or a group.
struct TemplatePart {
    let literal: Array[UInt8];
    /// The group index, or -1 for a literal.
    let group: Int64;
}

func isNameByte(b: UInt8) -> Bool {
    (b >= 48 and b <= 57) or (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or b == 95
}

/// Splits a template into literals and group references. Unknown names
/// expand to nothing.
func parseTemplate(template: Array[UInt8], names: Array[String]) -> Array[TemplatePart] {
    var parts: [TemplatePart] = [];
    var literal: [UInt8] = [];
    var i: Int64 = 0;
    let n = template.count;
    while i < n {
        let b = template(i);
        if b != 36 or i + 1 >= n {
            literal.append(b);
            i = i + 1;
            continue;
        }
        if template(i + 1) == 36 {
            literal.append(36);
            i = i + 2;
            continue;
        }
        var nameStart = i + 1;
        var nameEnd = nameStart;
        var next = nameStart;
        if template(nameStart) == 123 {
            nameStart = nameStart + 1;
            nameEnd = nameStart;
            while nameEnd < n and template(nameEnd) != 125 { nameEnd = nameEnd + 1; }
            if nameEnd >= n {
                // No closing brace: keep the text as it is.
                literal.append(b);
                i = i + 1;
                continue;
            }
            next = nameEnd + 1;
        } else {
            while nameEnd < n and isNameByte(template(nameEnd)) { nameEnd = nameEnd + 1; }
            next = nameEnd;
        }
        if nameEnd == nameStart {
            literal.append(b);
            i = i + 1;
            continue;
        }
        if literal.count > 0 {
            parts.append(TemplatePart(literal: literal, group: -1));
            literal = [];
        }
        var nameBytes: [UInt8] = [];
        for k in nameStart..<nameEnd { nameBytes.append(template(k)); }
        let name = String(fromUtf8Lossy: nameBytes);
        var group: Int64 = names.count;
        if let .Some(number) = Int64(parsing: name) {
            group = number;
        } else {
            for k in 1..<names.count {
                if names(k) == name { group = k; }
            }
        }
        parts.append(TemplatePart(literal: [], group: group));
        i = next;
    }
    if literal.count > 0 { parts.append(TemplatePart(literal: literal, group: -1)); }
    parts
}
