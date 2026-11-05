module regex

// Parsing patterns into a syntax tree. Nodes live in one array and refer
// to their children by index, which keeps the tree a plain value.

let nodeEmpty: Int64 = 0;
let nodeLiteral: Int64 = 1;
let nodeClass: Int64 = 2;
let nodeAny: Int64 = 3;
let nodeAssert: Int64 = 4;
let nodeGroup: Int64 = 5;
let nodeConcat: Int64 = 6;
let nodeAlternate: Int64 = 7;
let nodeRepeat: Int64 = 8;

// Assertion kinds.
let assertTextStart: Int64 = 0;
let assertTextEnd: Int64 = 1;
let assertLineStart: Int64 = 2;
let assertLineEnd: Int64 = 3;
let assertWordBoundary: Int64 = 4;
let assertNotWordBoundary: Int64 = 5;

let maxCodePoint: Int64 = 0x10FFFF;
/// The largest count allowed in `{n,m}`, as in RE2.
let maxRepeat: Int64 = 1000;

struct Node {
    var kind: Int64;
    /// Literal: the code point. Class: its index. Assert: the kind.
    /// Group: the capture index, or -1. Repeat: the minimum.
    var value: Int64;
    /// Repeat: the maximum, or -1 for no limit.
    var max: Int64;
    var greedy: Bool;
    /// Literal and class: match either case.
    var fold: Bool;
    /// Any: also match `\n`.
    var dotAll: Bool;
    var children: Array[Int64];

    init(kind kind: Int64, value value: Int64) {
        self.kind = kind;
        self.value = value;
        self.max = -1;
        self.greedy = true;
        self.fold = false;
        self.dotAll = false;
        self.children = [];
    }
}

/// A set of code points as sorted, non-overlapping inclusive ranges.
struct CharClass {
    /// lo0, hi0, lo1, hi1, ...
    var ranges: Array[Int64];

    init() { self.ranges = []; }

    mutating func add(lo: Int64, hi: Int64) {
        self.ranges.append(lo);
        self.ranges.append(hi);
    }

    mutating func addClass(other: CharClass) {
        self.ranges.append(contentsOf: other.ranges);
    }

    /// Sorts and merges the ranges.
    mutating func normalize() {
        let n = self.ranges.count / 2;
        var order: [Int64] = [];
        for i in 0..<n { order.append(i); }
        // Insertion sort by start: classes are small.
        for i in 1..<n {
            let key = order(i);
            var j = i - 1;
            while j >= 0 and self.ranges(order(j) * 2) > self.ranges(key * 2) {
                order(j + 1) = order(j);
                j = j - 1;
            }
            order(j + 1) = key;
        }
        var merged: [Int64] = [];
        for i in order {
            let lo = self.ranges(i * 2);
            let hi = self.ranges(i * 2 + 1);
            let last = merged.count - 1;
            if last > 0 and lo <= merged(last) + 1 {
                if hi > merged(last) { merged(last) = hi; }
            } else {
                merged.append(lo);
                merged.append(hi);
            }
        }
        self.ranges = merged;
    }

    /// Everything not in the class. Call after `normalize`.
    func negated() -> CharClass {
        var out = CharClass();
        var next: Int64 = 0;
        var i: Int64 = 0;
        while i < self.ranges.count {
            if self.ranges(i) > next { out.add(next, self.ranges(i) - 1); }
            next = self.ranges(i + 1) + 1;
            i = i + 2;
        }
        if next <= maxCodePoint { out.add(next, maxCodePoint); }
        out
    }

    func contains(c: Int64) -> Bool {
        // Binary search over the ranges.
        var lo: Int64 = 0;
        var hi = self.ranges.count / 2;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if c < self.ranges(mid * 2) {
                hi = mid;
            } else if c > self.ranges(mid * 2 + 1) {
                lo = mid + 1;
            } else {
                return true;
            }
        }
        false
    }
}

func digitClass() -> CharClass {
    var c = CharClass();
    c.add(48, 57);
    c
}

func wordClass() -> CharClass {
    var c = CharClass();
    c.add(48, 57);
    c.add(65, 90);
    c.add(95, 95);
    c.add(97, 122);
    c
}

func spaceClass() -> CharClass {
    var c = CharClass();
    c.add(9, 13);
    c.add(32, 32);
    c
}

/// A POSIX class name inside brackets (`[[:alpha:]]`), ASCII only.
func posixClass(name: String) -> CharClass? {
    var c = CharClass();
    if name == "alpha" { c.add(65, 90); c.add(97, 122); }
    else if name == "digit" { c.add(48, 57); }
    else if name == "alnum" { c.add(48, 57); c.add(65, 90); c.add(97, 122); }
    else if name == "upper" { c.add(65, 90); }
    else if name == "lower" { c.add(97, 122); }
    else if name == "space" { c.add(9, 13); c.add(32, 32); }
    else if name == "xdigit" { c.add(48, 57); c.add(65, 70); c.add(97, 102); }
    else if name == "punct" { c.add(33, 47); c.add(58, 64); c.add(91, 96); c.add(123, 126); }
    else if name == "word" { return .Some(wordClass()); }
    else if name == "blank" { c.add(9, 9); c.add(32, 32); }
    else if name == "cntrl" { c.add(0, 31); c.add(127, 127); }
    else if name == "print" { c.add(32, 126); }
    else if name == "graph" { c.add(33, 126); }
    else if name == "ascii" { c.add(0, 127); }
    else { return .None; }
    .Some(c)
}

/// The code point at `i` in UTF-8 `bytes` and its length. Malformed
/// bytes read as U+FFFD, one byte long.
func decodeUtf8(bytes: Array[UInt8], i: Int64) -> (Int64, Int64) {
    let b0 = Int64(from: bytes(i));
    if b0 < 0x80 { return (b0, 1); }
    let n = bytes.count;
    if b0 >= 0xC2 and b0 < 0xE0 and i + 1 < n {
        let b1 = Int64(from: bytes(i + 1));
        if (b1 & 0xC0) == 0x80 { return (((b0 & 0x1F) << 6) | (b1 & 0x3F), 2); }
    } else if b0 >= 0xE0 and b0 < 0xF0 and i + 2 < n {
        let b1 = Int64(from: bytes(i + 1));
        let b2 = Int64(from: bytes(i + 2));
        if (b1 & 0xC0) == 0x80 and (b2 & 0xC0) == 0x80 {
            let c = ((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
            if c >= 0x800 and (c < 0xD800 or c > 0xDFFF) { return (c, 3); }
        }
    } else if b0 >= 0xF0 and b0 < 0xF5 and i + 3 < n {
        let b1 = Int64(from: bytes(i + 1));
        let b2 = Int64(from: bytes(i + 2));
        let b3 = Int64(from: bytes(i + 3));
        if (b1 & 0xC0) == 0x80 and (b2 & 0xC0) == 0x80 and (b3 & 0xC0) == 0x80 {
            let c = ((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
            if c >= 0x10000 and c <= maxCodePoint { return (c, 4); }
        }
    }
    (0xFFFD, 1)
}

/// Simple case folding: the other case of `c`, or `c`.
func otherCase(c: Int64) -> Int64 {
    if c >= 65 and c <= 90 { return c + 32; }
    if c >= 97 and c <= 122 { return c - 32; }
    if c < 128 { return c; }
    guard let .Some(ch) = Char(UInt32(from: c)) else { return c; }
    let lower = Int64(from: ch.lowercased().value());
    if lower != c { return lower; }
    Int64(from: ch.uppercased().value())
}

struct Parser {
    let pattern: Array[UInt8];
    var pos: Int64;
    var nodes: Array[Node];
    var classes: Array[CharClass];
    /// Capture names by index; "" for unnamed and for group 0.
    var names: Array[String];
    var caseInsensitive: Bool;
    var multiLine: Bool;
    var dotAll: Bool;

    init(pattern pattern: String) {
        self.pattern = pattern.toBytes();
        self.pos = 0;
        self.nodes = [];
        self.classes = [];
        self.names = [""];
        self.caseInsensitive = false;
        self.multiLine = false;
        self.dotAll = false;
    }

    func error(message: String) -> RegexError { RegexError(message: message, position: self.pos) }

    var atEnd: Bool { self.pos >= self.pattern.count }

    func peek() -> Int64 { Int64(from: self.pattern(self.pos)) }

    /// The next code point, consumed.
    mutating func next() -> Int64 {
        let (c, len) = decodeUtf8(self.pattern, self.pos);
        self.pos = self.pos + len;
        c
    }

    mutating func add(node: Node) -> Int64 {
        self.nodes.append(node);
        self.nodes.count - 1
    }

    mutating func addClass(members: CharClass, negate: Bool) -> Int64 {
        var c = members;
        c.normalize();
        if negate { c = c.negated(); }
        self.classes.append(c);
        var node = Node(kind: nodeClass, value: self.classes.count - 1);
        node.fold = self.caseInsensitive;
        self.add(node)
    }

    /// The whole pattern.
    mutating func parse() -> Result[Int64, RegexError] {
        let root = try self.parseAlternation();
        if not self.atEnd {
            // Only an unmatched `)` stops alternation early.
            return .Err(self.error("unopened group"));
        }
        .Ok(root)
    }

    mutating func parseAlternation() -> Result[Int64, RegexError] {
        let first = try self.parseConcat();
        var branches: [Int64] = [first];
        while not self.atEnd and self.peek() == 124 {
            self.pos = self.pos + 1;
            branches.append(try self.parseConcat());
        }
        if branches.count == 1 { return .Ok(branches(0)); }
        var node = Node(kind: nodeAlternate, value: 0);
        node.children = branches;
        .Ok(self.add(node))
    }

    mutating func parseConcat() -> Result[Int64, RegexError] {
        var items: [Int64] = [];
        while not self.atEnd {
            let c = self.peek();
            if c == 124 or c == 41 { break; }
            // A flag group with no body changes the flags for the rest of
            // this group and produces no node.
            if let .Some(item) = try self.parseRepeat() { items.append(item); }
        }
        if items.count == 0 { return .Ok(self.add(Node(kind: nodeEmpty, value: 0))); }
        if items.count == 1 { return .Ok(items(0)); }
        var node = Node(kind: nodeConcat, value: 0);
        node.children = items;
        .Ok(self.add(node))
    }

    mutating func parseRepeat() -> Result[Int64?, RegexError] {
        let start = self.pos;
        guard let .Some(atom) = try self.parseAtom() else { return .Ok(.None); }
        var result = atom;
        var repeated = false;
        while not self.atEnd {
            let c = self.peek();
            var min: Int64 = 0;
            var max: Int64 = -1;
            if c == 42 {
                self.pos = self.pos + 1;
            } else if c == 43 {
                min = 1;
                self.pos = self.pos + 1;
            } else if c == 63 {
                max = 1;
                self.pos = self.pos + 1;
            } else if c == 123 {
                guard let .Some(bounds) = try self.parseCounted() else { break; }
                min = bounds.0;
                max = bounds.1;
            } else {
                break;
            }
            if repeated { return .Err(self.error("repeated repetition operator")); }
            repeated = true;
            let kind = self.nodes(atom).kind;
            if kind == nodeEmpty or kind == nodeAssert {
                return .Err(RegexError(message: "nothing to repeat", position: start));
            }
            var node = Node(kind: nodeRepeat, value: min);
            node.max = max;
            if not self.atEnd and self.peek() == 63 {
                node.greedy = false;
                self.pos = self.pos + 1;
            }
            node.children = [result];
            result = self.add(node);
        }
        .Ok(.Some(result))
    }

    /// `{n}`, `{n,}`, `{n,m}` or `{,m}` at `{`; `.None` (consuming nothing) if
    /// the brace isn't a valid count, so it reads as a literal.
    mutating func parseCounted() -> Result[(Int64, Int64)?, RegexError] {
        let start = self.pos;
        self.pos = self.pos + 1;
        // `{,n}` means `{0,n}`.
        var min: Int64 = 0;
        if let .Some(n) = self.parseNumber() {
            min = n;
        } else if self.atEnd or self.peek() != 44 {
            self.pos = start;
            return .Ok(.None);
        }
        var max = min;
        if not self.atEnd and self.peek() == 44 {
            self.pos = self.pos + 1;
            max = -1;
            if let .Some(m) = self.parseNumber() { max = m; }
        }
        if self.atEnd or self.peek() != 125 {
            self.pos = start;
            return .Ok(.None);
        }
        self.pos = self.pos + 1;
        if min > maxRepeat or max > maxRepeat {
            return .Err(RegexError(message: "repeat count over \(maxRepeat)", position: start));
        }
        if max >= 0 and max < min {
            return .Err(RegexError(message: "repeat range out of order", position: start));
        }
        .Ok(.Some((min, max)))
    }

    mutating func parseNumber() -> Int64? {
        var value: Int64 = 0;
        var digits: Int64 = 0;
        while not self.atEnd and self.peek() >= 48 and self.peek() <= 57 {
            if value < 100000 { value = value * 10 + self.peek() - 48; }
            self.pos = self.pos + 1;
            digits = digits + 1;
        }
        if digits == 0 { return .None; }
        .Some(value)
    }

    /// One atom, or `.None` for a flag-setting group like `(?i)`.
    mutating func parseAtom() -> Result[Int64?, RegexError] {
        let c = self.peek();
        if c == 40 { return self.parseGroup(); }
        if c == 42 or c == 43 or c == 63 {
            return .Err(self.error("nothing to repeat"));
        }
        if c == 91 {
            self.pos = self.pos + 1;
            return .Ok(.Some(try self.parseClass()));
        }
        if c == 46 {
            self.pos = self.pos + 1;
            var node = Node(kind: nodeAny, value: 0);
            node.dotAll = self.dotAll;
            return .Ok(.Some(self.add(node)));
        }
        if c == 94 {
            self.pos = self.pos + 1;
            var kind = assertTextStart;
            if self.multiLine { kind = assertLineStart; }
            return .Ok(.Some(self.add(Node(kind: nodeAssert, value: kind))));
        }
        if c == 36 {
            self.pos = self.pos + 1;
            var kind = assertTextEnd;
            if self.multiLine { kind = assertLineEnd; }
            return .Ok(.Some(self.add(Node(kind: nodeAssert, value: kind))));
        }
        if c == 92 { return .Ok(.Some(try self.parseEscape())); }
        let literal = self.next();
        .Ok(.Some(self.literal(literal)))
    }

    mutating func literal(c: Int64) -> Int64 {
        var node = Node(kind: nodeLiteral, value: c);
        node.fold = self.caseInsensitive and otherCase(c) != c;
        self.add(node)
    }

    mutating func parseGroup() -> Result[Int64?, RegexError] {
        let start = self.pos;
        self.pos = self.pos + 1;
        var capture: Int64 = -1;
        let saved = (self.caseInsensitive, self.multiLine, self.dotAll);
        if not self.atEnd and self.peek() == 63 {
            self.pos = self.pos + 1;
            if self.atEnd { return .Err(self.error("unclosed group")); }
            let c = self.peek();
            if c == 60 or c == 80 {
                // (?<name>...) or (?P<name>...)
                if c == 80 { self.pos = self.pos + 1; }
                if self.atEnd or self.peek() != 60 { return .Err(self.error("bad group name")); }
                self.pos = self.pos + 1;
                let name = try self.parseName();
                capture = self.names.count;
                self.names.append(name);
            } else {
                // Flags: (?i), (?-i), (?im-s:...).
                var on = true;
                loop {
                    if self.atEnd { return .Err(self.error("unclosed group")); }
                    let f = self.peek();
                    self.pos = self.pos + 1;
                    if f == 45 { on = false; }
                    else if f == 105 { self.caseInsensitive = on; }
                    else if f == 109 { self.multiLine = on; }
                    else if f == 115 { self.dotAll = on; }
                    else if f == 41 {
                        // Flags for the rest of the enclosing group.
                        return .Ok(.None);
                    }
                    else if f == 58 { break; }
                    else { return .Err(RegexError(message: "unknown flag", position: self.pos - 1)); }
                }
            }
        } else {
            capture = self.names.count;
            self.names.append("");
        }
        let body = try self.parseAlternation();
        if self.atEnd or self.peek() != 41 {
            return .Err(RegexError(message: "unclosed group", position: start));
        }
        self.pos = self.pos + 1;
        self.caseInsensitive = saved.0;
        self.multiLine = saved.1;
        self.dotAll = saved.2;
        var node = Node(kind: nodeGroup, value: capture);
        node.children = [body];
        .Ok(.Some(self.add(node)))
    }

    mutating func parseName() -> Result[String, RegexError] {
        var bytes: [UInt8] = [];
        while not self.atEnd and self.peek() != 62 {
            let c = self.peek();
            let ok = (c >= 48 and c <= 57) or (c >= 65 and c <= 90) or (c >= 97 and c <= 122) or c == 95;
            if not ok { return .Err(self.error("bad group name")); }
            bytes.append(self.pattern(self.pos));
            self.pos = self.pos + 1;
        }
        if self.atEnd or bytes.count == 0 { return .Err(self.error("bad group name")); }
        self.pos = self.pos + 1;
        let name = String(fromUtf8Lossy: bytes);
        for existing in self.names {
            if existing == name { return .Err(self.error("duplicate group name \(name)")); }
        }
        .Ok(name)
    }

    /// An escape outside brackets, at `\`.
    mutating func parseEscape() -> Result[Int64, RegexError] {
        let start = self.pos;
        self.pos = self.pos + 1;
        if self.atEnd { return .Err(RegexError(message: "trailing backslash", position: start)); }
        let c = self.peek();
        if c == 98 or c == 66 or c == 65 or c == 122 {
            self.pos = self.pos + 1;
            var kind = assertWordBoundary;
            if c == 66 { kind = assertNotWordBoundary; }
            if c == 65 { kind = assertTextStart; }
            if c == 122 { kind = assertTextEnd; }
            return .Ok(self.add(Node(kind: nodeAssert, value: kind)));
        }
        if let .Some(members) = try self.parseClassEscape() {
            return .Ok(self.addClass(members.0, members.1));
        }
        let literal = try self.parseLiteralEscape(start);
        .Ok(self.literal(literal))
    }

    /// `\d`, `\w`, `\s` and their negations at the letter: the members and
    /// whether to negate it. `.None` (consuming nothing) for other escapes.
    mutating func parseClassEscape() -> Result[(CharClass, Bool)?, RegexError] {
        let c = self.peek();
        var members = CharClass();
        if c == 100 or c == 68 { members = digitClass(); }
        else if c == 119 or c == 87 { members = wordClass(); }
        else if c == 115 or c == 83 { members = spaceClass(); }
        else { return .Ok(.None); }
        self.pos = self.pos + 1;
        .Ok(.Some((members, c == 68 or c == 87 or c == 83)))
    }

    /// A single-character escape at the character after `\`.
    mutating func parseLiteralEscape(start: Int64) -> Result[Int64, RegexError] {
        let c = self.peek();
        self.pos = self.pos + 1;
        if c == 110 { return .Ok(10); }
        if c == 116 { return .Ok(9); }
        if c == 114 { return .Ok(13); }
        if c == 102 { return .Ok(12); }
        if c == 118 { return .Ok(11); }
        if c == 97 { return .Ok(7); }
        if c == 48 { return .Ok(0); }
        if c == 120 { return self.parseHex(start); }
        // Any ASCII punctuation escapes itself.
        let isAlnum = (c >= 48 and c <= 57) or (c >= 65 and c <= 90) or (c >= 97 and c <= 122);
        if c < 128 and not isAlnum and c > 32 { return .Ok(c); }
        .Err(RegexError(message: "bad escape", position: start))
    }

    /// `\xHH` or `\x{H...}`, after the `x`.
    mutating func parseHex(start: Int64) -> Result[Int64, RegexError] {
        var value: Int64 = 0;
        var digits: Int64 = 0;
        let braced = not self.atEnd and self.peek() == 123;
        if braced { self.pos = self.pos + 1; }
        while not self.atEnd {
            let c = self.peek();
            if braced and c == 125 { break; }
            if not braced and digits == 2 { break; }
            var d: Int64 = -1;
            if c >= 48 and c <= 57 { d = c - 48; }
            if c >= 65 and c <= 70 { d = c - 55; }
            if c >= 97 and c <= 102 { d = c - 87; }
            if d < 0 { return .Err(RegexError(message: "bad hex escape", position: start)); }
            value = value * 16 + d;
            if value > maxCodePoint { return .Err(RegexError(message: "hex escape out of range", position: start)); }
            digits = digits + 1;
            self.pos = self.pos + 1;
        }
        if braced {
            if self.atEnd { return .Err(RegexError(message: "bad hex escape", position: start)); }
            self.pos = self.pos + 1;
        }
        if digits == 0 or (not braced and digits != 2) {
            return .Err(RegexError(message: "bad hex escape", position: start));
        }
        .Ok(value)
    }

    /// A bracketed class, after `[`.
    mutating func parseClass() -> Result[Int64, RegexError] {
        let start = self.pos - 1;
        var negate = false;
        if not self.atEnd and self.peek() == 94 {
            negate = true;
            self.pos = self.pos + 1;
        }
        var members = CharClass();
        var first = true;
        loop {
            if self.atEnd { return .Err(RegexError(message: "unclosed character class", position: start)); }
            let c = self.peek();
            if c == 93 and not first {
                self.pos = self.pos + 1;
                break;
            }
            first = false;
            // [:name:]
            if c == 91 and self.pos + 1 < self.pattern.count and self.pattern(self.pos + 1) == 58 {
                let close = self.findPosixEnd();
                if close > 0 {
                    var negated = false;
                    var nameStart = self.pos + 2;
                    if self.pattern(nameStart) == 94 {
                        negated = true;
                        nameStart = nameStart + 1;
                    }
                    var nameBytes: [UInt8] = [];
                    for i in nameStart..<close { nameBytes.append(self.pattern(i)); }
                    guard let .Some(posix) = posixClass(String(fromUtf8Lossy: nameBytes)) else {
                        return .Err(self.error("unknown POSIX class"));
                    }
                    var p = posix;
                    if negated {
                        p.normalize();
                        p = p.negated();
                    }
                    members.addClass(p);
                    self.pos = close + 2;
                    continue;
                }
            }
            let lo = try self.parseClassItem(members);
            if lo < 0 { continue; }
            // A range `lo-hi`, unless the `-` ends the class.
            if self.pos + 1 < self.pattern.count and self.peek() == 45 and self.pattern(self.pos + 1) != 93 {
                self.pos = self.pos + 1;
                let hi = try self.parseClassItem(members);
                if hi < 0 { return .Err(self.error("bad character class range")); }
                if hi < lo { return .Err(self.error("character class range out of order")); }
                members.add(lo, hi);
            } else {
                members.add(lo, lo);
            }
        }
        if self.caseInsensitive { members = foldClass(members); }
        .Ok(self.addClass(members, negate))
    }

    /// The index of `:]` closing a POSIX class at the current `[:`, or -1.
    func findPosixEnd() -> Int64 {
        var i = self.pos + 2;
        while i + 1 < self.pattern.count {
            if self.pattern(i) == 58 and self.pattern(i + 1) == 93 { return i; }
            if self.pattern(i) == 93 { return -1; }
            i = i + 1;
        }
        return -1;
    }

    /// One class member: returns its code point, or -1 after adding a
    /// whole members (`\d`) to `members` directly.
    mutating func parseClassItem(mutating members: CharClass) -> Result[Int64, RegexError] {
        if self.peek() != 92 { return .Ok(self.next()); }
        let start = self.pos;
        self.pos = self.pos + 1;
        if self.atEnd { return .Err(RegexError(message: "trailing backslash", position: start)); }
        if let .Some(found) = try self.parseClassEscape() {
            var escaped = found.0;
            if found.1 {
                escaped.normalize();
                escaped = escaped.negated();
            }
            members.addClass(escaped);
            return .Ok(-1);
        }
        if self.peek() == 98 {
            // `\b` in a class is a backspace.
            self.pos = self.pos + 1;
            return .Ok(8);
        }
        self.parseLiteralEscape(start)
    }
}

/// Adds the other case of every member, for `(?i)` classes. Large
/// ranges are left as they are past ASCII.
func foldClass(members: CharClass) -> CharClass {
    var out = members;
    var i: Int64 = 0;
    while i < members.ranges.count {
        let lo = members.ranges(i);
        let hi = members.ranges(i + 1);
        if hi - lo <= 512 {
            for c in lo..=hi {
                let other = otherCase(c);
                if other != c { out.add(other, other); }
            }
        }
        i = i + 2;
    }
    out
}
