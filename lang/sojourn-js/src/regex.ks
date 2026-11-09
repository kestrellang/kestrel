module js

// A backtracking regular expression engine for JS patterns (ECMA-262
// §22.2), over UTF-16 code units.
//
// The parser compiles a pattern straight to a small program with
// relative jumps, so fragments compose by concatenation. The matcher runs
// it with an explicit backtrack stack: a choice point records where to
// resume, and every capture or register write records how to undo it.
// Lookarounds run as nested matches. Lookbehind tries each start position
// that ends at the current one, which matches what the spec does in
// all but the rarest capture cases.

enum RxInst {
    /// One character (a code point in unicode mode), canonicalized when
    /// `fold` (case-insensitive). The flag-dependent instructions carry
    /// the flags in effect where they appear, since `(?i:...)` and the
    /// like change them for part of a pattern.
    case Char(cp: Int64, fold: Bool)
    case AnyChar(dotAll: Bool)
    /// A class from `RxProgram.classes`.
    case Class(index: Int64, negate: Bool, fold: Bool)
    /// Try pc + first, and on failure pc + second.
    case Split(first: Int64, second: Int64)
    case Jump(Int64)
    case Save(Int64)
    /// Resets the captures of groups [from, to), for each loop iteration.
    case ClearCaptures(from: Int64, to: Int64)
    case AssertStart(multiline: Bool)
    case AssertEnd(multiline: Bool)
    case WordBoundary(negate: Bool, fold: Bool)
    case BackRef(group: Int64, fold: Bool)
    /// A lookaround whose program is the next `length` instructions.
    case Look(length: Int64, behind: Bool, negate: Bool)
    /// Loop registers: the position an iteration started at, so an
    /// iteration that matched nothing stops the loop.
    case SetMark(Int64)
    case CheckProgress(Int64)
    case Match
}

struct RxProgram {
    var insts: [RxInst]
    /// Sorted, merged [lo, hi] code point ranges, flattened.
    var classes: [[Int64]]
    /// Capturing groups, not counting the whole match.
    var groupCount: Int64
    /// Group names by group number ("" for unnamed groups).
    var names: [String]
    var registers: Int64
    var ignoreCase: Bool
    var multiline: Bool
    var dotAll: Bool
    var unicode: Bool
}

struct RxFlags {
    var hasIndices: Bool
    var global: Bool
    var ignoreCase: Bool
    var multiline: Bool
    var dotAll: Bool
    var unicode: Bool
    var unicodeSets: Bool
    var sticky: Bool
}

/// Validates a flags string.
func parseRxFlags(f: Units) -> Optional[RxFlags] {
    var r = RxFlags(hasIndices: false, global: false, ignoreCase: false, multiline: false, dotAll: false,
                    unicode: false, unicodeSets: false, sticky: false);
    for c in f {
        let x = Int64(from: c);
        if x == 100 {
            if r.hasIndices { return .None; }
            r.hasIndices = true;
        } else if x == 103 {
            if r.global { return .None; }
            r.global = true;
        } else if x == 105 {
            if r.ignoreCase { return .None; }
            r.ignoreCase = true;
        } else if x == 109 {
            if r.multiline { return .None; }
            r.multiline = true;
        } else if x == 115 {
            if r.dotAll { return .None; }
            r.dotAll = true;
        } else if x == 117 {
            if r.unicode { return .None; }
            r.unicode = true;
        } else if x == 118 {
            if r.unicodeSets { return .None; }
            r.unicodeSets = true;
        } else if x == 121 {
            if r.sticky { return .None; }
            r.sticky = true;
        } else {
            return .None;
        }
    }
    if r.unicode and r.unicodeSets { return .None; }
    .Some(r)
}

/// Compiles a pattern, or returns the SyntaxError message.
func compileRegex(pattern: Units, flags: RxFlags) -> Result[RxProgram, String] {
    let unicode = flags.unicode or flags.unicodeSets;
    var p = RxParser(src: pattern, pos: 0, unicode: unicode, ignoreCase: flags.ignoreCase,
                     multiline: flags.multiline, dotAll: flags.dotAll,
                     groupCount: 0, nextGroup: 1, names: [""], hasNames: false, classes: [], registers: 0);
    try p.prescan();
    let body = try p.parseDisjunction();
    if p.pos < p.src.count {
        return .Err(if Int64(from: p.src(p.pos)) == 41 { "Unmatched ')'" } else { "Unexpected character" });
    }
    var insts: [RxInst] = [.Save(0)];
    for i in body { insts.append(i); }
    insts.append(.Save(1));
    insts.append(.Match);
    .Ok(RxProgram(insts: insts, classes: p.classes, groupCount: p.groupCount, names: p.names,
                  registers: p.registers, ignoreCase: flags.ignoreCase, multiline: flags.multiline,
                  dotAll: flags.dotAll, unicode: unicode))
}

/// A parsed atom: its code and the capture groups it holds.
struct RxAtom {
    var code: [RxInst]
    var capFrom: Int64
    var capTo: Int64
    var quantifiable: Bool
}

/// One class member: a code point, or a set from a class escape.
struct RxClassAtom {
    var cp: Int64
    var ranges: [Int64]
    var isSet: Bool
}

struct RxParser {
    var src: Units
    var pos: Int64
    var unicode: Bool
    /// The flags in effect at `pos`.
    var ignoreCase: Bool
    var multiline: Bool
    var dotAll: Bool
    var groupCount: Int64
    var nextGroup: Int64
    var names: [String]
    var hasNames: Bool
    var classes: [[Int64]]
    var registers: Int64

    func peek() -> Int64 {
        if self.pos >= self.src.count { return -1; }
        Int64(from: self.src(self.pos))
    }

    func peekAt(offset: Int64) -> Int64 {
        let i = self.pos + offset;
        if i >= self.src.count { return -1; }
        Int64(from: self.src(i))
    }

    /// Reads one source character: a code point (pairs joined) in
    /// unicode mode, else a code unit.
    mutating func next() -> Int64 {
        let c = self.peek();
        self.pos = self.pos + 1;
        if self.unicode and c >= 0xD800 and c <= 0xDBFF {
            let d = self.peek();
            if d >= 0xDC00 and d <= 0xDFFF {
                self.pos = self.pos + 1;
                return 0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00);
            }
        }
        c
    }

    /// Counts the capturing groups and collects their names, which
    /// backreferences need before their groups appear.
    mutating func prescan() -> Result[(), String] {
        var i: Int64 = 0;
        var inClass = false;
        let n = self.src.count;
        while i < n {
            let c = Int64(from: self.src(i));
            if c == 92 {
                i = i + 2;
                continue;
            }
            if inClass {
                if c == 93 { inClass = false; }
            } else if c == 91 {
                inClass = true;
            } else if c == 40 {
                let c1 = if i + 1 < n { Int64(from: self.src(i + 1)) } else { -1 };
                let c2 = if i + 2 < n { Int64(from: self.src(i + 2)) } else { -1 };
                if c1 != 63 {
                    self.groupCount = self.groupCount + 1;
                } else if c2 == 60 {
                    let c3 = if i + 3 < n { Int64(from: self.src(i + 3)) } else { -1 };
                    if c3 != 61 and c3 != 33 {
                        self.groupCount = self.groupCount + 1;
                        self.hasNames = true;
                    }
                }
            }
            i = i + 1;
        }
        .Ok(())
    }

    mutating func parseDisjunction() -> Result[[RxInst], String] {
        var alts: [[RxInst]] = [];
        alts.append(try self.parseAlternative());
        while self.peek() == 124 {
            self.pos = self.pos + 1;
            alts.append(try self.parseAlternative());
        }
        if alts.count == 1 { return .Ok(alts(0)); }
        // Split to each alternative in turn; each jumps to the end.
        var total: Int64 = 0;
        for i in 0..<alts.count {
            total = total + alts(i).count + (if i + 1 < alts.count { 2 } else { 0 });
        }
        var out: [RxInst] = [];
        for i in 0..<alts.count {
            let a = alts(i);
            if i + 1 < alts.count {
                out.append(.Split(first: 1, second: a.count + 2));
                for x in a { out.append(x); }
                out.append(.Jump(total - out.count));
            } else {
                for x in a { out.append(x); }
            }
        }
        .Ok(out)
    }

    mutating func parseAlternative() -> Result[[RxInst], String] {
        var out: [RxInst] = [];
        while self.pos < self.src.count {
            let c = self.peek();
            if c == 124 or c == 41 { break; }
            let term = try self.parseTerm();
            for x in term { out.append(x); }
        }
        .Ok(out)
    }

    mutating func parseTerm() -> Result[[RxInst], String] {
        let c = self.peek();
        // Assertions.
        if c == 94 {
            self.pos = self.pos + 1;
            return self.unquantified([RxInst.AssertStart(multiline: self.multiline)]);
        }
        if c == 36 {
            self.pos = self.pos + 1;
            return self.unquantified([RxInst.AssertEnd(multiline: self.multiline)]);
        }
        if c == 92 and (self.peekAt(1) == 98 or self.peekAt(1) == 66) {
            let negate = self.peekAt(1) == 66;
            self.pos = self.pos + 2;
            return self.unquantified([RxInst.WordBoundary(negate: negate, fold: self.ignoreCase)]);
        }
        if c == 40 and self.peekAt(1) == 63 {
            let c2 = self.peekAt(2);
            let behind = c2 == 60 and (self.peekAt(3) == 61 or self.peekAt(3) == 33);
            if c2 == 61 or c2 == 33 or behind {
                let negate = if behind { self.peekAt(3) == 33 } else { c2 == 33 };
                self.pos = self.pos + (if behind { 4 } else { 3 });
                let capFrom = self.nextGroup;
                let sub = try self.parseDisjunction();
                if self.peek() != 41 { return .Err("Unterminated group"); }
                self.pos = self.pos + 1;
                var code: [RxInst] = [.Look(length: sub.count + 1, behind: behind, negate: negate)];
                for x in sub { code.append(x); }
                code.append(.Match);
                // Annex B lets a lookahead take a quantifier outside unicode mode.
                if behind or self.unicode { return self.unquantified(code); }
                let atom = RxAtom(code: code, capFrom: capFrom, capTo: self.nextGroup, quantifiable: true);
                return self.parseQuantifier(atom);
            }
        }
        let atom = try self.parseAtom();
        self.parseQuantifier(atom)
    }

    /// An assertion: a quantifier after it is an error.
    mutating func unquantified(code: [RxInst]) -> Result[[RxInst], String] {
        let c = self.peek();
        if c == 42 or c == 43 or c == 63 { return .Err("Nothing to repeat"); }
        if c == 123 and self.bracedQuantifierAhead() { return .Err("Nothing to repeat"); }
        .Ok(code)
    }

    /// Whether `{` starts `{n}`, `{n,}` or `{n,m}`.
    func bracedQuantifierAhead() -> Bool {
        var i = self.pos + 1;
        let n = self.src.count;
        var digits: Int64 = 0;
        while i < n and Int64(from: self.src(i)) >= 48 and Int64(from: self.src(i)) <= 57 {
            i = i + 1;
            digits = digits + 1;
        }
        if digits == 0 { return false; }
        if i < n and Int64(from: self.src(i)) == 44 {
            i = i + 1;
            while i < n and Int64(from: self.src(i)) >= 48 and Int64(from: self.src(i)) <= 57 { i = i + 1; }
        }
        i < n and Int64(from: self.src(i)) == 125
    }

    /// A decimal number, saturating at 2^31.
    mutating func parseDecimal() -> Int64 {
        var v: Int64 = 0;
        while self.peek() >= 48 and self.peek() <= 57 {
            v = v * 10 + self.peek() - 48;
            if v > 2147483648 { v = 2147483648; }
            self.pos = self.pos + 1;
        }
        v
    }

    mutating func parseQuantifier(atom: RxAtom) -> Result[[RxInst], String] {
        let c = self.peek();
        var min: Int64 = 0;
        var max: Int64 = -1;
        if c == 42 {
            self.pos = self.pos + 1;
        } else if c == 43 {
            self.pos = self.pos + 1;
            min = 1;
        } else if c == 63 {
            self.pos = self.pos + 1;
            max = 1;
        } else if c == 123 and self.bracedQuantifierAhead() {
            self.pos = self.pos + 1;
            min = self.parseDecimal();
            max = min;
            if self.peek() == 44 {
                self.pos = self.pos + 1;
                max = if self.peek() == 125 { -1 } else { self.parseDecimal() };
            }
            self.pos = self.pos + 1;
            if max >= 0 and max < min { return .Err("numbers out of order in {} quantifier"); }
        } else {
            return .Ok(atom.code);
        }
        if not atom.quantifiable { return .Err("Nothing to repeat"); }
        var greedy = true;
        if self.peek() == 63 {
            self.pos = self.pos + 1;
            greedy = false;
        }
        if atom.code.count * (if min > 0 { min } else { 1 }) > 1000000 { return .Err("Regular expression too large"); }
        .Ok(self.quantify(atom, min, max, greedy))
    }

    mutating func quantify(atom: RxAtom, min: Int64, max: Int64, greedy: Bool) -> [RxInst] {
        var piece: [RxInst] = [];
        if atom.capTo > atom.capFrom { piece.append(.ClearCaptures(from: atom.capFrom, to: atom.capTo)); }
        for x in atom.code { piece.append(x); }
        var out: [RxInst] = [];
        for _ in 0..<min {
            for x in piece { out.append(x); }
        }
        if max < 0 {
            let r = self.registers;
            self.registers = self.registers + 1;
            var body: [RxInst] = [.SetMark(r)];
            for x in piece { body.append(x); }
            body.append(.CheckProgress(r));
            let n = body.count;
            out.append(if greedy { RxInst.Split(first: 1, second: n + 2) } else { RxInst.Split(first: n + 2, second: 1) });
            for x in body { out.append(x); }
            out.append(.Jump(0 - n - 1));
            return out;
        }
        let k = max - min;
        let step = piece.count + 1;
        let total = k * step;
        for i in 0..<k {
            let remaining = total - i * step;
            out.append(if greedy { RxInst.Split(first: 1, second: remaining) } else { RxInst.Split(first: remaining, second: 1) });
            for x in piece { out.append(x); }
        }
        out
    }

    mutating func parseAtom() -> Result[RxAtom, String] {
        let c = self.peek();
        let capFrom = self.nextGroup;
        if c == 46 {
            self.pos = self.pos + 1;
            return .Ok(RxAtom(code: [RxInst.AnyChar(dotAll: self.dotAll)], capFrom: capFrom, capTo: capFrom, quantifiable: true));
        }
        if c == 40 {
            self.pos = self.pos + 1;
            var group: Int64 = -1;
            let saved = (self.ignoreCase, self.multiline, self.dotAll);
            if self.peek() == 63 {
                let m = self.peekAt(1);
                if m == 58 {
                    self.pos = self.pos + 2;
                } else if m == 105 or m == 109 or m == 115 or m == 45 {
                    self.pos = self.pos + 1;
                    try self.parseModifiers();
                } else if m == 60 {
                    self.pos = self.pos + 2;
                    let name = try self.parseGroupName();
                    if self.names.contains(name) { return .Err("Duplicate capture group name"); }
                    group = self.nextGroup;
                    self.nextGroup = self.nextGroup + 1;
                    self.names.append(name);
                } else {
                    return .Err("Invalid group");
                }
            } else {
                group = self.nextGroup;
                self.nextGroup = self.nextGroup + 1;
                self.names.append("");
            }
            let sub = try self.parseDisjunction();
            if self.peek() != 41 { return .Err("Unterminated group"); }
            self.pos = self.pos + 1;
            let (ic, ml, da) = saved;
            self.ignoreCase = ic;
            self.multiline = ml;
            self.dotAll = da;
            var code: [RxInst] = [];
            if group >= 0 { code.append(.Save(group * 2)); }
            for x in sub { code.append(x); }
            if group >= 0 { code.append(.Save(group * 2 + 1)); }
            return .Ok(RxAtom(code: code, capFrom: capFrom, capTo: self.nextGroup, quantifiable: true));
        }
        if c == 91 {
            let code = try self.parseClass();
            return .Ok(RxAtom(code: code, capFrom: capFrom, capTo: capFrom, quantifiable: true));
        }
        if c == 92 {
            self.pos = self.pos + 1;
            let code = try self.parseAtomEscape();
            return .Ok(RxAtom(code: code, capFrom: capFrom, capTo: capFrom, quantifiable: true));
        }
        if c == 42 or c == 43 or c == 63 { return .Err("Nothing to repeat"); }
        if c == 123 {
            if self.unicode or self.bracedQuantifierAhead() { return .Err("Nothing to repeat"); }
        }
        if self.unicode and (c == 125 or c == 93) { return .Err("Lone quantifier brackets"); }
        let cp = self.next();
        .Ok(RxAtom(code: self.charCode(cp), capFrom: capFrom, capTo: capFrom, quantifiable: true))
    }

    /// `{Name}` or `{Name=Value}` after `\p` (or `\P`, `negate`): the
    /// property's code points, complemented for `\P`.
    mutating func parsePropertyEscape(negate: Bool) -> Result[Array[Int64], String] {
        if self.peek() != 123 { return .Err("Invalid property name"); }
        self.pos = self.pos + 1;
        let name = self.propertyNamePart();
        var value = "";
        var hasValue = false;
        if self.peek() == 61 {
            self.pos = self.pos + 1;
            value = self.propertyNamePart();
            hasValue = true;
        }
        if self.peek() != 125 { return .Err("Invalid property name"); }
        self.pos = self.pos + 1;
        var id: Int64 = -1;
        if hasValue {
            if name == "General_Category" or name == "gc" {
                id = unicodeLookup(unicodeGcNames(), value);
            } else if name == "Script" or name == "sc" {
                id = unicodeLookup(unicodeScriptNames(), value);
            } else if name == "Script_Extensions" or name == "scx" {
                id = unicodeLookup(unicodeScriptExtensionNames(), value);
            }
        } else if not name.isEmpty {
            id = unicodeLookup(unicodeGcNames(), name);
            if id < 0 { id = unicodeLookup(unicodeBinaryNames(), name); }
        }
        if id < 0 { return .Err("Invalid property name"); }
        let ranges = decodeUnicodeTable(unicodeTable(id));
        .Ok(if negate { complementRanges(ranges, 0x10FFFF) } else { ranges })
    }

    /// Letters, digits and underscores of a property name or value.
    mutating func propertyNamePart() -> String {
        var out = "";
        while true {
            let c = self.peek();
            let ok = (c >= 65 and c <= 90) or (c >= 97 and c <= 122) or (c >= 48 and c <= 57) or c == 95;
            if not ok { break; }
            out.append(char: Char(unchecked: UInt32(truncating: c)));
            self.pos = self.pos + 1;
        }
        out
    }

    /// `ims-ims:` after `(?`: the flags a modifier group adds and removes,
    /// applied to the parser until the group ends.
    mutating func parseModifiers() -> Result[(), String] {
        var add: [Int64] = [];
        var remove: [Int64] = [];
        var removing = false;
        while true {
            let c = self.next();
            if c == 58 { break; }
            if c == 45 and not removing {
                removing = true;
                continue;
            }
            if c != 105 and c != 109 and c != 115 { return .Err("Invalid regular expression modifiers"); }
            if add.contains(c) or remove.contains(c) { return .Err("Repeated regular expression modifier"); }
            if removing { remove.append(c); } else { add.append(c); }
        }
        if removing and add.isEmpty and remove.isEmpty { return .Err("Empty regular expression modifiers"); }
        for c in add { self.setModifier(c, true); }
        for c in remove { self.setModifier(c, false); }
        .Ok(())
    }

    mutating func setModifier(c: Int64, on: Bool) {
        if c == 105 { self.ignoreCase = on; } else if c == 109 { self.multiline = on; } else { self.dotAll = on; }
    }

    /// A literal character's code: a non-unicode pattern matches an
    /// astral character as its two code units.
    func charCode(cp: Int64) -> [RxInst] {
        if cp > 0xFFFF and not self.unicode {
            let hi = 0xD800 + ((cp - 0x10000) >> 10);
            let lo = 0xDC00 + ((cp - 0x10000) & 0x3FF);
            return [RxInst.Char(cp: hi, fold: self.ignoreCase), RxInst.Char(cp: lo, fold: self.ignoreCase)];
        }
        [RxInst.Char(cp: if self.ignoreCase { canonicalize(cp, self.unicode) } else { cp }, fold: self.ignoreCase)]
    }

    /// `name>` after `(?<` or `\k<`.
    mutating func parseGroupName() -> Result[String, String] {
        var name = "";
        while true {
            if self.pos >= self.src.count { return .Err("Invalid capture group name"); }
            var c = self.next();
            if c == 62 { break; }
            if c == 92 {
                // \uXXXX and \u{...} spell identifier characters.
                if self.peek() != 117 { return .Err("Invalid capture group name"); }
                self.pos = self.pos + 1;
                let saved = self.unicode;
                self.unicode = true;
                let r = self.parseUnicodeEscape();
                self.unicode = saved;
                c = try r;
            }
            let ok = if name.isEmpty { isRxIdStart(c) } else { isRxIdPart(c) };
            if not ok { return .Err("Invalid capture group name"); }
            name.append(char: Char(unchecked: UInt32(truncating: c)));
        }
        if name.isEmpty { return .Err("Invalid capture group name"); }
        .Ok(name)
    }

    /// After `\` outside a class.
    mutating func parseAtomEscape() -> Result[[RxInst], String] {
        let c = self.peek();
        if c < 0 { return .Err("\\ at end of pattern"); }
        if c >= 49 and c <= 57 {
            let start = self.pos;
            let n = self.parseDecimal();
            if n <= self.groupCount { return .Ok([RxInst.BackRef(group: n, fold: self.ignoreCase)]); }
            if self.unicode { return .Err("Invalid escape"); }
            // Annex B: an octal escape, or 8 and 9 as themselves.
            self.pos = start;
            if c >= 56 {
                self.pos = self.pos + 1;
                return .Ok(self.charCode(c));
            }
            return .Ok(self.charCode(self.parseLegacyOctal()));
        }
        if c == 48 {
            self.pos = self.pos + 1;
            let d = self.peek();
            if d >= 48 and d <= 57 {
                if self.unicode { return .Err("Invalid decimal escape"); }
                self.pos = self.pos - 1;
                return .Ok(self.charCode(self.parseLegacyOctal()));
            }
            return .Ok(self.charCode(0));
        }
        if c == 107 {
            if self.unicode or self.hasNames {
                self.pos = self.pos + 1;
                if self.peek() != 60 { return .Err("Invalid named reference"); }
                self.pos = self.pos + 1;
                let name = try self.parseGroupName();
                for i in 1..<self.names.count {
                    if self.names(i) == name { return .Ok([RxInst.BackRef(group: i, fold: self.ignoreCase)]); }
                }
                // A name defined later in the pattern.
                let later = try self.groupNumberOf(name);
                return .Ok([RxInst.BackRef(group: later, fold: self.ignoreCase)]);
            }
            self.pos = self.pos + 1;
            return .Ok(self.charCode(107));
        }
        if c == 100 or c == 68 or c == 115 or c == 83 or c == 119 or c == 87 {
            self.pos = self.pos + 1;
            let ranges = classEscapeRanges(c, self.unicode, self.ignoreCase);
            self.classes.append(ranges);
            return .Ok([RxInst.Class(index: self.classes.count - 1, negate: false, fold: self.ignoreCase)]);
        }
        if (c == 112 or c == 80) and self.unicode {
            self.pos = self.pos + 1;
            let ranges = try self.parsePropertyEscape(c == 80);
            self.classes.append(ranges);
            return .Ok([RxInst.Class(index: self.classes.count - 1, negate: false, fold: self.ignoreCase)]);
        }
        let cp = try self.parseCharacterEscape(false);
        .Ok(self.charCode(cp))
    }

    /// Finds a named group by scanning the whole pattern for `(?<name>`.
    func groupNumberOf(name: String) -> Result[Int64, String] {
        var group: Int64 = 0;
        var i: Int64 = 0;
        var inClass = false;
        let n = self.src.count;
        while i < n {
            let c = Int64(from: self.src(i));
            if c == 92 {
                i = i + 2;
                continue;
            }
            if inClass {
                if c == 93 { inClass = false; }
            } else if c == 91 {
                inClass = true;
            } else if c == 40 {
                let c1 = if i + 1 < n { Int64(from: self.src(i + 1)) } else { -1 };
                let c2 = if i + 2 < n { Int64(from: self.src(i + 2)) } else { -1 };
                let c3 = if i + 3 < n { Int64(from: self.src(i + 3)) } else { -1 };
                if c1 != 63 {
                    group = group + 1;
                } else if c2 == 60 and c3 != 61 and c3 != 33 {
                    group = group + 1;
                    var j = i + 3;
                    var found = "";
                    while j < n and Int64(from: self.src(j)) != 62 {
                        found.append(char: Char(unchecked: UInt32(truncating: Int64(from: self.src(j)))));
                        j = j + 1;
                    }
                    if found == name { return .Ok(group); }
                }
            }
            i = i + 1;
        }
        .Err("Invalid named capture referenced")
    }

    /// Up to three octal digits, at most \377.
    mutating func parseLegacyOctal() -> Int64 {
        var v: Int64 = 0;
        var count: Int64 = 0;
        while count < 3 and self.peek() >= 48 and self.peek() <= 55 {
            let next = v * 8 + self.peek() - 48;
            if next > 255 { break; }
            v = next;
            self.pos = self.pos + 1;
            count = count + 1;
        }
        v
    }

    /// A CharacterEscape after `\`: control letters, hex and unicode
    /// escapes, and identity escapes.
    mutating func parseCharacterEscape(inClass: Bool) -> Result[Int64, String] {
        let c = self.peek();
        if c == 102 { self.pos = self.pos + 1; return .Ok(12); }
        if c == 110 { self.pos = self.pos + 1; return .Ok(10); }
        if c == 114 { self.pos = self.pos + 1; return .Ok(13); }
        if c == 116 { self.pos = self.pos + 1; return .Ok(9); }
        if c == 118 { self.pos = self.pos + 1; return .Ok(11); }
        if c == 99 {
            let l = self.peekAt(1);
            if (l >= 65 and l <= 90) or (l >= 97 and l <= 122) {
                self.pos = self.pos + 2;
                return .Ok(l % 32);
            }
            if not self.unicode and inClass and ((l >= 48 and l <= 57) or l == 95) {
                self.pos = self.pos + 2;
                return .Ok(l % 32);
            }
            if self.unicode { return .Err("Invalid unicode escape"); }
            // Annex B: the backslash is itself, and `c` is read next.
            return .Ok(92);
        }
        if c == 120 {
            self.pos = self.pos + 1;
            let h1 = hexDigitValue(self.peek());
            let h2 = hexDigitValue(self.peekAt(1));
            if h1 >= 0 and h2 >= 0 {
                self.pos = self.pos + 2;
                return .Ok(h1 * 16 + h2);
            }
            if self.unicode { return .Err("Invalid escape"); }
            return .Ok(120);
        }
        if c == 117 {
            self.pos = self.pos + 1;
            let start = self.pos;
            match self.parseUnicodeEscape() {
                .Ok(v) => return .Ok(v),
                .Err(e) => {
                    if self.unicode { return .Err(e); }
                    self.pos = start;
                    return .Ok(117);
                }
            }
        }
        if self.unicode {
            // Only syntax characters, `/`, and `-` in a class.
            let syntax = c == 94 or c == 36 or c == 92 or c == 46 or c == 42 or c == 43 or c == 63 or c == 40
                or c == 41 or c == 91 or c == 93 or c == 123 or c == 125 or c == 124 or c == 47;
            if syntax or (inClass and c == 45) {
                self.pos = self.pos + 1;
                return .Ok(c);
            }
            return .Err("Invalid escape");
        }
        .Ok(self.next())
    }

    /// After `\u`: four hex digits, a surrogate pair of them in unicode
    /// mode, or `{hex}` in unicode mode.
    mutating func parseUnicodeEscape() -> Result[Int64, String] {
        if self.unicode and self.peek() == 123 {
            self.pos = self.pos + 1;
            var v: Int64 = 0;
            var digits: Int64 = 0;
            while hexDigitValue(self.peek()) >= 0 {
                v = v * 16 + hexDigitValue(self.peek());
                if v > 0x10FFFF { return .Err("Invalid unicode escape"); }
                self.pos = self.pos + 1;
                digits = digits + 1;
            }
            if digits == 0 or self.peek() != 125 { return .Err("Invalid unicode escape"); }
            self.pos = self.pos + 1;
            return .Ok(v);
        }
        let v = self.hex4();
        if v < 0 { return .Err("Invalid unicode escape"); }
        if self.unicode and v >= 0xD800 and v <= 0xDBFF and self.peek() == 92 and self.peekAt(1) == 117 {
            let save = self.pos;
            self.pos = self.pos + 2;
            let w = self.hex4();
            if w >= 0xDC00 and w <= 0xDFFF { return .Ok(0x10000 + ((v - 0xD800) << 10) + (w - 0xDC00)); }
            self.pos = save;
        }
        .Ok(v)
    }

    mutating func hex4() -> Int64 {
        var v: Int64 = 0;
        for i in 0..<4 {
            let h = hexDigitValue(self.peekAt(i));
            if h < 0 { return -1; }
            v = v * 16 + h;
        }
        self.pos = self.pos + 4;
        v
    }

    mutating func parseClass() -> Result[[RxInst], String] {
        self.pos = self.pos + 1;
        var negate = false;
        if self.peek() == 94 {
            negate = true;
            self.pos = self.pos + 1;
        }
        var ranges: [Int64] = [];
        while true {
            if self.pos >= self.src.count { return .Err("Unterminated character class"); }
            if self.peek() == 93 {
                self.pos = self.pos + 1;
                break;
            }
            let a = try self.parseClassAtom();
            if self.peek() == 45 and self.peekAt(1) != 93 and self.peekAt(1) >= 0 {
                self.pos = self.pos + 1;
                let b = try self.parseClassAtom();
                if a.isSet or b.isSet {
                    if self.unicode { return .Err("Invalid character class"); }
                    addClassAtom(ranges, a);
                    ranges.append(45);
                    ranges.append(45);
                    addClassAtom(ranges, b);
                    continue;
                }
                if a.cp > b.cp { return .Err("Range out of order in character class"); }
                ranges.append(a.cp);
                ranges.append(b.cp);
                continue;
            }
            addClassAtom(ranges, a);
        }
        self.classes.append(normalizeRanges(ranges));
        .Ok([RxInst.Class(index: self.classes.count - 1, negate: negate, fold: self.ignoreCase)])
    }

    mutating func parseClassAtom() -> Result[RxClassAtom, String] {
        let c = self.peek();
        if c != 92 {
            let cp = self.next();
            return .Ok(RxClassAtom(cp: cp, ranges: [], isSet: false));
        }
        self.pos = self.pos + 1;
        let e = self.peek();
        if e < 0 { return .Err("\\ at end of pattern"); }
        if e == 98 {
            self.pos = self.pos + 1;
            return .Ok(RxClassAtom(cp: 8, ranges: [], isSet: false));
        }
        if e == 45 and self.unicode {
            self.pos = self.pos + 1;
            return .Ok(RxClassAtom(cp: 45, ranges: [], isSet: false));
        }
        if e == 100 or e == 68 or e == 115 or e == 83 or e == 119 or e == 87 {
            self.pos = self.pos + 1;
            return .Ok(RxClassAtom(cp: -1, ranges: classEscapeRanges(e, self.unicode, self.ignoreCase), isSet: true));
        }
        if (e == 112 or e == 80) and self.unicode {
            self.pos = self.pos + 1;
            return .Ok(RxClassAtom(cp: -1, ranges: try self.parsePropertyEscape(e == 80), isSet: true));
        }
        if e >= 48 and e <= 57 {
            if self.unicode {
                if e == 48 and not (self.peekAt(1) >= 48 and self.peekAt(1) <= 57) {
                    self.pos = self.pos + 1;
                    return .Ok(RxClassAtom(cp: 0, ranges: [], isSet: false));
                }
                return .Err("Invalid class escape");
            }
            if e >= 56 {
                self.pos = self.pos + 1;
                return .Ok(RxClassAtom(cp: e, ranges: [], isSet: false));
            }
            return .Ok(RxClassAtom(cp: self.parseLegacyOctal(), ranges: [], isSet: false));
        }
        let cp = try self.parseCharacterEscape(true);
        .Ok(RxClassAtom(cp: cp, ranges: [], isSet: false))
    }
}

func addClassAtom(mutating ranges: Array[Int64], a: RxClassAtom) {
    if a.isSet {
        for x in a.ranges { ranges.append(x); }
    } else {
        ranges.append(a.cp);
        ranges.append(a.cp);
    }
}

/// Sorts and merges flattened [lo, hi] pairs.
func normalizeRanges(r: Array[Int64]) -> Array[Int64] {
    var pairs: [Int64] = [];
    // Insertion sort by lo, since classes are small.
    var los: [Int64] = [];
    var his: [Int64] = [];
    var i: Int64 = 0;
    while i + 1 < r.count {
        let lo = r(i);
        let hi = r(i + 1);
        var at = los.count;
        while at > 0 and los(at - 1) > lo { at = at - 1; }
        los.insert(lo, at: at);
        his.insert(hi, at: at);
        i = i + 2;
    }
    for j in 0..<los.count {
        let n = pairs.count;
        if n > 0 and los(j) <= pairs(n - 1) + 1 {
            if his(j) > pairs(n - 1) { pairs(n - 1) = his(j); }
        } else {
            pairs.append(los(j));
            pairs.append(his(j));
        }
    }
    pairs
}

/// Everything not in `r` (normalized), up to `max`.
func complementRanges(r: Array[Int64], max: Int64) -> Array[Int64] {
    var out: [Int64] = [];
    var next: Int64 = 0;
    var i: Int64 = 0;
    while i + 1 < r.count {
        if r(i) > next {
            out.append(next);
            out.append(r(i) - 1);
        }
        next = r(i + 1) + 1;
        i = i + 2;
    }
    if next <= max {
        out.append(next);
        out.append(max);
    }
    out
}

/// \d \D \s \S \w \W as ranges.
func classEscapeRanges(c: Int64, unicode: Bool, ignoreCase: Bool) -> Array[Int64] {
    let max = if unicode { 0x10FFFF } else { 0xFFFF };
    let lower = c + 32;
    let isUpper = c < 97;
    let kind = if isUpper { lower } else { c };
    var r: [Int64] = [];
    if kind == 100 {
        r = [48, 57];
    } else if kind == 115 {
        r = [9, 13, 32, 32, 160, 160, 5760, 5760, 8192, 8202, 8232, 8233, 8239, 8239, 8287, 8287, 12288, 12288, 65279, 65279];
    } else {
        r = [48, 57, 65, 90, 95, 95, 97, 122];
        // ſ and K fold to s and k with unicode case-insensitive matching.
        if unicode and ignoreCase { r = [48, 57, 65, 90, 95, 95, 97, 122, 383, 383, 8490, 8490]; }
    }
    let n = normalizeRanges(r);
    if isUpper { complementRanges(n, max) } else { n }
}

func isRxIdStart(c: Int64) -> Bool {
    (c >= 65 and c <= 90) or (c >= 97 and c <= 122) or c == 36 or c == 95 or c >= 128
}

func isRxIdPart(c: Int64) -> Bool {
    isRxIdStart(c) or (c >= 48 and c <= 57) or c == 0x200C or c == 0x200D
}


/// Canonicalize (§22.2.2.7.3): simple uppercase outside unicode mode,
/// unless that maps non-ASCII to ASCII; a simple fold (lowercase of the
/// uppercase) in unicode mode.
func canonicalize(c: Int64, unicode: Bool) -> Int64 {
    if c < 128 {
        if unicode {
            return if c >= 65 and c <= 90 { c + 32 } else { c };
        }
        return if c >= 97 and c <= 122 { c - 32 } else { c };
    }
    if c >= 0xD800 and c <= 0xDFFF { return c; }
    var s = "";
    s.append(char: Char(unchecked: UInt32(truncating: c)));
    let up = s.uppercased();
    var count: Int64 = 0;
    var u: Int64 = c;
    for ch in up.chars {
        u = Int64(from: ch.value());
        count = count + 1;
    }
    if unicode {
        if count != 1 { u = c; }
        var t = "";
        t.append(char: Char(unchecked: UInt32(truncating: u)));
        var lowered: Int64 = u;
        var lc: Int64 = 0;
        for ch in t.lowercased().chars {
            lowered = Int64(from: ch.value());
            lc = lc + 1;
        }
        return if lc == 1 { lowered } else { u };
    }
    if count != 1 { return c; }
    if u < 128 { return c; }
    u
}

func isWordChar(c: Int64, unicodeIgnoreCase: Bool) -> Bool {
    if (c >= 48 and c <= 57) or (c >= 65 and c <= 90) or (c >= 97 and c <= 122) or c == 95 { return true; }
    unicodeIgnoreCase and (c == 383 or c == 8490)
}

func rxInRanges(r: Array[Int64], c: Int64) -> Bool {
    var i: Int64 = 0;
    while i + 1 < r.count {
        if c < r(i) { return false; }
        if c <= r(i + 1) { return true; }
        i = i + 2;
    }
    false
}

// ----------------------------------------------------------------------
// Matching
// ----------------------------------------------------------------------

/// A backtrack entry: kind 0 resumes at (pc, pos); 1 restores capture
/// `pc` to `pos`; 2 restores register `pc` to `pos`.
struct RxBacktrack {
    var kind: Int64
    var pc: Int64
    var pos: Int64
}

/// Tries the program at `start`: the capture positions, -1 for unset.
func rxMatchAt(p: RxProgram, s: Units, start: Int64) -> Optional[Array[Int64]] {
    var caps: [Int64] = [];
    for _ in 0..<((p.groupCount + 1) * 2) { caps.append(-1); }
    var regs: [Int64] = [];
    for _ in 0..<p.registers { regs.append(-1); }
    let end = rxRun(p, s, 0, start, caps, regs, -1);
    if end < 0 { return .None; }
    .Some(caps)
}

/// The character at `pos` and its width: a code point in unicode mode.
func rxCharAt(p: RxProgram, s: Units, pos: Int64) -> (Int64, Int64) {
    let c = Int64(from: s(pos));
    if p.unicode and c >= 0xD800 and c <= 0xDBFF and pos + 1 < s.count {
        let d = Int64(from: s(pos + 1));
        if d >= 0xDC00 and d <= 0xDFFF { return (0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00), 2); }
    }
    (c, 1)
}

func rxClassHas(p: RxProgram, index: Int64, c: Int64, fold: Bool) -> Bool {
    let r = p.classes(index);
    if rxInRanges(r, c) { return true; }
    if not fold { return false; }
    // Some member canonicalizes to the same character: try c's case
    // variants.
    let k = canonicalize(c, p.unicode);
    if k != c and rxInRanges(r, k) { return true; }
    var s = "";
    s.append(char: Char(unchecked: UInt32(truncating: c)));
    for ch in s.lowercased().chars {
        let l = Int64(from: ch.value());
        if l != c and canonicalize(l, p.unicode) == k and rxInRanges(r, l) { return true; }
    }
    for ch in s.uppercased().chars {
        let u = Int64(from: ch.value());
        if u != c and canonicalize(u, p.unicode) == k and rxInRanges(r, u) { return true; }
    }
    false
}

/// Runs from `pc` at `pos`; returns the end position, or -1. With
/// `endAt` ≥ 0 a match must end exactly there (for lookbehind).
func rxRun(p: RxProgram, s: Units, pc0: Int64, pos0: Int64, mutating caps: Array[Int64], mutating regs: Array[Int64], endAt: Int64) -> Int64 {
    var stack: [RxBacktrack] = [];
    var pc = pc0;
    var pos = pos0;
    let n = s.count;
    while true {
        let inst = p.insts(pc);
        var ok = true;
        match inst {
            .Char(c, fold) => {
                if pos >= n {
                    ok = false;
                } else {
                    let (x, w) = rxCharAt(p, s, pos);
                    let y = if fold { canonicalize(x, p.unicode) } else { x };
                    if y == c {
                        pos = pos + w;
                        pc = pc + 1;
                    } else {
                        ok = false;
                    }
                }
            },
            .AnyChar(dotAll) => {
                if pos >= n {
                    ok = false;
                } else {
                    let (x, w) = rxCharAt(p, s, pos);
                    if not dotAll and isLineTerminator(x) {
                        ok = false;
                    } else {
                        pos = pos + w;
                        pc = pc + 1;
                    }
                }
            },
            .Class(index, negate, fold) => {
                if pos >= n {
                    ok = false;
                } else {
                    let (x, w) = rxCharAt(p, s, pos);
                    if rxClassHas(p, index, x, fold) != negate {
                        pos = pos + w;
                        pc = pc + 1;
                    } else {
                        ok = false;
                    }
                }
            },
            .Split(first, second) => {
                stack.append(RxBacktrack(kind: 0, pc: pc + second, pos: pos));
                pc = pc + first;
            },
            .Jump(d) => pc = pc + d,
            .Save(slot) => {
                stack.append(RxBacktrack(kind: 1, pc: slot, pos: caps(slot)));
                caps(slot) = pos;
                pc = pc + 1;
            },
            .ClearCaptures(from, to) => {
                for slot in (from * 2)..<(to * 2) {
                    stack.append(RxBacktrack(kind: 1, pc: slot, pos: caps(slot)));
                    caps(slot) = -1;
                }
                pc = pc + 1;
            },
            .AssertStart(multiline) => {
                if pos == 0 or (multiline and isLineTerminator(Int64(from: s(pos - 1)))) { pc = pc + 1; } else { ok = false; }
            },
            .AssertEnd(multiline) => {
                if pos == n or (multiline and isLineTerminator(Int64(from: s(pos)))) { pc = pc + 1; } else { ok = false; }
            },
            .WordBoundary(negate, fold) => {
                let ui = p.unicode and fold;
                let a = pos > 0 and isWordChar(Int64(from: s(pos - 1)), ui);
                let b = pos < n and isWordChar(Int64(from: s(pos)), ui);
                if (a != b) != negate { pc = pc + 1; } else { ok = false; }
            },
            .BackRef(g, fold) => {
                let st = caps(g * 2);
                let en = caps(g * 2 + 1);
                if st < 0 or en < 0 {
                    pc = pc + 1;
                } else {
                    let len = en - st;
                    if pos + len > n {
                        ok = false;
                    } else {
                        var same = true;
                        for i in 0..<len {
                            let x = Int64(from: s(st + i));
                            let y = Int64(from: s(pos + i));
                            if x != y and (not fold or canonicalize(x, p.unicode) != canonicalize(y, p.unicode)) {
                                same = false;
                                break;
                            }
                        }
                        if same {
                            pos = pos + len;
                            pc = pc + 1;
                        } else {
                            ok = false;
                        }
                    }
                }
            },
            .Look(length, behind, negate) => {
                let snapshot = caps;
                var found = false;
                if behind {
                    var st = pos;
                    while st >= 0 {
                        if rxRun(p, s, pc + 1, st, caps, regs, pos) >= 0 {
                            found = true;
                            break;
                        }
                        caps = snapshot;
                        st = st - 1;
                    }
                } else {
                    found = rxRun(p, s, pc + 1, pos, caps, regs, -1) >= 0;
                }
                if negate {
                    caps = snapshot;
                    found = not found;
                } else if found {
                    // Keep the lookaround's captures, undoably.
                    for i in 0..<caps.count {
                        if caps(i) != snapshot(i) { stack.append(RxBacktrack(kind: 1, pc: i, pos: snapshot(i))); }
                    }
                } else {
                    caps = snapshot;
                }
                if found { pc = pc + 1 + length; } else { ok = false; }
            },
            .SetMark(r) => {
                stack.append(RxBacktrack(kind: 2, pc: r, pos: regs(r)));
                regs(r) = pos;
                pc = pc + 1;
            },
            .CheckProgress(r) => {
                if pos == regs(r) { ok = false; } else { pc = pc + 1; }
            },
            .Match => {
                if endAt < 0 or pos == endAt { return pos; }
                ok = false;
            }
        }
        if ok { continue; }
        // Backtrack: undo writes until a choice point.
        var resumed = false;
        while true {
            let top = stack.pop();
            let b = match top {
                .Some(x) => x,
                .None => break
            };
            if b.kind == 0 {
                pc = b.pc;
                pos = b.pos;
                resumed = true;
                break;
            }
            if b.kind == 1 { caps(b.pc) = b.pos; } else { regs(b.pc) = b.pos; }
        }
        if not resumed { return -1; }
    }
    return -1;
}

/// The table id `key` has in a `|name:id|` lookup string, or -1.
func unicodeLookup(names: String, key: String) -> Int64 {
    if key.isEmpty { return -1; }
    let hay = unitsOf(names);
    let at = unitsIndexOf(hay, unitsOf("|" + key + ":"), 0);
    if at < 0 { return -1; }
    var i = at + unitsOf(key).count + 2;
    var id: Int64 = 0;
    while i < hay.count and Int64(from: hay(i)) != 124 {
        id = id * 10 + Int64(from: hay(i)) - 48;
        i = i + 1;
    }
    id
}

/// A `unicodeTable` string as flattened [lo, hi] ranges.
func decodeUnicodeTable(s: String) -> Array[Int64] {
    var out: [Int64] = [];
    var vals: [Int64] = [];
    var cur: Int64 = 0;
    for u in unitsOf(s) {
        let c = Int64(from: u);
        if c == 44 {
            vals.append(cur);
            cur = 0;
        } else {
            cur = cur * 36 + (if c <= 57 { c - 48 } else { c - 87 });
        }
    }
    vals.append(cur);
    var prev: Int64 = -1;
    var i: Int64 = 0;
    while i + 1 < vals.count {
        let lo = prev + 1 + vals(i);
        let hi = lo + vals(i + 1);
        out.append(lo);
        out.append(hi);
        prev = hi;
        i = i + 2;
    }
    out
}
