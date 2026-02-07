use kestrel_test_suite::*;

#[test]
fn search_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // Test contains
            if s.contains("world") == false { return 1 }
            if s.contains("xyz") { return 2 }

            // Test find
            let pos = s.find("world");
            if pos.isNone() { return 3 }
            if pos.unwrap() != 6 { return 4 }

            // Test starts/ends with
            if s.starts(with: "hello") == false { return 5 }
            if s.starts(with: "world") { return 6 }

            // Test ends with
            if s.ends(with: "world") == false { return 7 }
            if s.ends(with: "hello") { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn manipulation() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test append
            var s = std.text.String();
            s.append("hello");
            s.append(" world");
            if s.byteCount != 11 { return 1 }

            // Test trim
            let padded: std.text.String = "  hello  ";
            let trimmed = padded.trimmed();
            if trimmed.byteCount != 5 { return 2 }

            // Test lowercase/uppercase
            let mixed: std.text.String = "HeLLo";
            let lower = mixed.lowercased();
            let upper = mixed.uppercased();
            if lower.equals("hello") == false { return 3 }
            if upper.equals("HELLO") == false { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_and_byte_access() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // Test first()
            let f = s.first();
            if f.isNone() { return 1 }
            if f.unwrap().equals('h') == false { return 2 }

            // Test last()
            let l = s.last();
            if l.isNone() { return 3 }
            if l.unwrap().equals('o') == false { return 4 }

            // Test first() and last() on empty string
            let empty = std.text.String();
            if empty.first().isSome() { return 5 }
            if empty.last().isSome() { return 6 }

            // Test char(at:)
            let c0 = s.char(at: 0);
            if c0.equals('h') == false { return 7 }
            let c4 = s.char(at: 4);
            if c4.equals('o') == false { return 8 }

            // Test char(checked:)
            let checked = s.char(checked: 2);
            if checked.isNone() { return 9 }
            if checked.unwrap().equals('l') == false { return 10 }

            // Test char(checked:) out of bounds
            let oob = s.char(checked: 100);
            if oob.isSome() { return 11 }

            // Test byteAt()
            let b0 = s.byteAt(0);
            if b0.isNone() { return 12 }
            // 'h' is ASCII 104
            if b0.unwrap() != std.num.UInt8(intLiteral: 104) { return 13 }

            // Test byteAt() out of bounds
            let bOob = s.byteAt(100);
            if bOob.isSome() { return 14 }

            // Test byteAtUnchecked()
            let bu = s.byteAtUnchecked(1);
            // 'e' is ASCII 101
            if bu != std.num.UInt8(intLiteral: 101) { return 15 }

            // Test count (Unicode code point count)
            if s.count != 5 { return 16 }
            let ascii: std.text.String = "abc";
            if ascii.count != 3 { return 17 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn mutation_and_clear() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test appendChar
            var s = std.text.String();
            s.appendChar('H');
            s.appendChar('i');
            if s.byteCount != 2 { return 1 }
            if s.equals("Hi") == false { return 2 }

            // Test appendByte
            var s2 = std.text.String();
            // Append ASCII 'A' (65)
            s2.appendByte(std.num.UInt8(intLiteral: 65));
            s2.appendByte(std.num.UInt8(intLiteral: 66));
            if s2.byteCount != 2 { return 3 }
            if s2.equals("AB") == false { return 4 }

            // Test clear()
            var s3: std.text.String = "hello world";
            if s3.isEmpty { return 5 }
            s3.clear();
            if s3.isEmpty == false { return 6 }
            if s3.byteCount != 0 { return 7 }

            // Test init(capacity:)
            var s4 = std.text.String(capacity: 64);
            if s4.capacity < 64 { return 8 }
            if s4.isEmpty == false { return 9 }
            if s4.byteCount != 0 { return 10 }

            // After appending, capacity should still be >= 64
            s4.append("test");
            if s4.byteCount != 4 { return 11 }
            if s4.capacity < 64 { return 12 }

            // Test that clear preserves capacity
            let capBefore = s4.capacity;
            s4.clear();
            if s4.capacity != capBefore { return 13 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn trimming() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- Mutating trim() ----
            var s1: std.text.String = "  hello  ";
            s1.trim();
            if s1.equals("hello") == false { return 1 }

            // ---- Mutating trimStart() ----
            var s2: std.text.String = "  hello  ";
            s2.trimStart();
            if s2.equals("hello  ") == false { return 2 }

            // ---- Mutating trimEnd() ----
            var s3: std.text.String = "  hello  ";
            s3.trimEnd();
            if s3.equals("  hello") == false { return 3 }

            // ---- Mutating trim(matching:) ----
            var s4: std.text.String = "xxhelloxx";
            s4.trim(matching: { (c) in c.equals('x') });
            if s4.equals("hello") == false { return 4 }

            // ---- Mutating trimStart(matching:) ----
            var s5: std.text.String = "xxhelloxx";
            s5.trimStart(matching: { (c) in c.equals('x') });
            if s5.equals("helloxx") == false { return 5 }

            // ---- Mutating trimEnd(matching:) ----
            var s6: std.text.String = "xxhelloxx";
            s6.trimEnd(matching: { (c) in c.equals('x') });
            if s6.equals("xxhello") == false { return 6 }

            // ---- Non-mutating trimmedStart() ----
            let s7: std.text.String = "  hello  ";
            let ts = s7.trimmedStart();
            if ts.equals("hello  ") == false { return 7 }
            // Original unchanged
            if s7.byteCount != 9 { return 8 }

            // ---- Non-mutating trimmedEnd() ----
            let s8: std.text.String = "  hello  ";
            let te = s8.trimmedEnd();
            if te.equals("  hello") == false { return 9 }

            // ---- Non-mutating trimmed(matching:) ----
            let s9: std.text.String = "..hello..";
            let tm = s9.trimmed(matching: { (c) in c.equals('.') });
            if tm.equals("hello") == false { return 10 }

            // ---- Non-mutating trimmedStart(matching:) ----
            let s10: std.text.String = "..hello..";
            let tsm = s10.trimmedStart(matching: { (c) in c.equals('.') });
            if tsm.equals("hello..") == false { return 11 }

            // ---- Non-mutating trimmedEnd(matching:) ----
            let s11: std.text.String = "..hello..";
            let tem = s11.trimmedEnd(matching: { (c) in c.equals('.') });
            if tem.equals("..hello") == false { return 12 }

            // ---- Trim with leading/trailing whitespace including newlines ----
            var s12: std.text.String = "  hello  ";
            s12.trimStart();
            s12.trimEnd();
            if s12.equals("hello") == false { return 13 }

            // ---- Trim on all-whitespace string ----
            var s13: std.text.String = "   ";
            s13.trim();
            if s13.isEmpty == false { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn case_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- lowercaseAscii() (mutating) ----
            var s1: std.text.String = "Hello WORLD";
            s1.lowercaseAscii();
            if s1.equals("hello world") == false { return 1 }

            // ---- uppercaseAscii() (mutating) ----
            var s2: std.text.String = "Hello world";
            s2.uppercaseAscii();
            if s2.equals("HELLO WORLD") == false { return 2 }

            // ---- lowercasedAscii() (non-mutating) ----
            let s3: std.text.String = "HELLO";
            let low = s3.lowercasedAscii();
            if low.equals("hello") == false { return 3 }
            // Original unchanged
            if s3.equals("HELLO") == false { return 4 }

            // ---- uppercasedAscii() (non-mutating) ----
            let s4: std.text.String = "hello";
            let up = s4.uppercasedAscii();
            if up.equals("HELLO") == false { return 5 }
            // Original unchanged
            if s4.equals("hello") == false { return 6 }

            // ---- titlecased() ----
            let s5: std.text.String = "hello world";
            let titled = s5.titlecased();
            if titled.equals("Hello World") == false { return 7 }

            // titlecased with multiple words
            let s6: std.text.String = "the quick brown fox";
            let titled2 = s6.titlecased();
            if titled2.equals("The Quick Brown Fox") == false { return 8 }

            // ---- equalsCaseInsensitive() ----
            let a: std.text.String = "Hello";
            let b: std.text.String = "hello";
            let c: std.text.String = "HELLO";
            let d: std.text.String = "world";
            if a.equalsCaseInsensitive(b) == false { return 9 }
            if a.equalsCaseInsensitive(c) == false { return 10 }
            if a.equalsCaseInsensitive(d) { return 11 }

            // ASCII case conversion preserves non-alpha chars
            let s7: std.text.String = "Hello 123!";
            if s7.lowercasedAscii().equals("hello 123!") == false { return 12 }
            if s7.uppercasedAscii().equals("HELLO 123!") == false { return 13 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn replacement_and_splitting() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- replaced() (non-mutating) ----
            let s1: std.text.String = "hello world hello";
            let r1 = s1.replaced("hello", with: "hi");
            if r1.equals("hi world hi") == false { return 1 }
            // Original unchanged
            if s1.equals("hello world hello") == false { return 2 }

            // Replace with longer string
            let s2: std.text.String = "aaa";
            let r2 = s2.replaced("a", with: "bb");
            if r2.equals("bbbbbb") == false { return 3 }

            // Replace with shorter string
            let s3: std.text.String = "hello";
            let r3 = s3.replaced("ll", with: "l");
            if r3.equals("helo") == false { return 4 }

            // Replace no match
            let s4: std.text.String = "hello";
            let r4 = s4.replaced("xyz", with: "abc");
            if r4.equals("hello") == false { return 5 }

            // ---- replace() (mutating) ----
            var s5: std.text.String = "foo bar foo";
            s5.replace("foo", with: "baz");
            if s5.equals("baz bar baz") == false { return 6 }

            // ---- split(separator:) ----
            let csv: std.text.String = "a,b,c";
            let parts = csv.split(",").collect();
            if parts.count != 3 { return 7 }
            if parts(unchecked: 0).equals("a") == false { return 8 }
            if parts(unchecked: 1).equals("b") == false { return 9 }
            if parts(unchecked: 2).equals("c") == false { return 10 }

            // Split with no separator found
            let noSep: std.text.String = "hello";
            let noParts = noSep.split(",").collect();
            if noParts.count != 1 { return 11 }
            if noParts(unchecked: 0).equals("hello") == false { return 12 }

            // Split with adjacent separators
            let adj: std.text.String = "a,,b";
            let adjParts = adj.split(",").collect();
            if adjParts.count != 3 { return 13 }
            if adjParts(unchecked: 1).equals("") == false { return 14 }

            // ---- split(matching:) ----
            let s6: std.text.String = "hello world\tthere";
            let wsParts = s6.split(matching: { (c) in c.isWhitespace() }).collect();
            if wsParts.count != 3 { return 15 }
            if wsParts(unchecked: 0).equals("hello") == false { return 16 }
            if wsParts(unchecked: 1).equals("world") == false { return 17 }
            if wsParts(unchecked: 2).equals("there") == false { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn searching_extended() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world hello";

            // ---- contains(matching:) ----
            let hasUpper = s.contains(matching: { (c) in c.isUppercase() });
            if hasUpper { return 1 }

            let hasLower = s.contains(matching: { (c) in c.isLowercase() });
            if hasLower == false { return 2 }

            // ---- find(matching:) ----
            let spacePos = s.find(matching: { (c) in c.equals(' ') });
            if spacePos.isNone() { return 3 }
            if spacePos.unwrap() != 5 { return 4 }

            // find(matching:) no match
            let noMatch = s.find(matching: { (c) in c.isDigit() });
            if noMatch.isSome() { return 5 }

            // ---- reverseFind() ----
            let lastHello = s.reverseFind("hello");
            if lastHello.isNone() { return 6 }
            if lastHello.unwrap() != 12 { return 7 }

            // reverseFind first occurrence
            let firstWorld = s.reverseFind("world");
            if firstWorld.isNone() { return 8 }
            if firstWorld.unwrap() != 6 { return 9 }

            // reverseFind no match
            let noRev = s.reverseFind("xyz");
            if noRev.isSome() { return 10 }

            // reverseFind empty string
            let emptyRev = s.reverseFind("");
            if emptyRev.isNone() { return 11 }
            // Should return length of string
            if emptyRev.unwrap() != 17 { return 12 }

            // ---- substringBytes(from:to:) ----
            let sub = s.substringBytes(from: 6, to: 11);
            if sub.equals("world") == false { return 13 }

            // substringBytes with invalid range
            let badSub = s.substringBytes(from: 10, to: 5);
            if badSub.isEmpty == false { return 14 }

            // substringBytes from start
            let prefix = s.substringBytes(from: 0, to: 5);
            if prefix.equals("hello") == false { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn repeating_and_padding() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- repeated(count:) ----
            let s1: std.text.String = "ab";
            let r1 = s1.repeated(3);
            if r1.equals("ababab") == false { return 1 }

            // Repeat zero times
            let r0 = s1.repeated(0);
            if r0.isEmpty == false { return 2 }

            // Repeat once
            let r1x = s1.repeated(1);
            if r1x.equals("ab") == false { return 3 }

            // ---- pad(start:with:) ----
            let s2: std.text.String = "hi";
            let ps = s2.pad(start: 5, with: '0');
            if ps.equals("000hi") == false { return 4 }

            // Pad when already long enough
            let s3: std.text.String = "hello";
            let ps2 = s3.pad(start: 3, with: '0');
            if ps2.equals("hello") == false { return 5 }

            // ---- pad(end:with:) ----
            let pe = s2.pad(end: 5, with: '.');
            if pe.equals("hi...") == false { return 6 }

            // Pad end when already long enough
            let pe2 = s3.pad(end: 3, with: '.');
            if pe2.equals("hello") == false { return 7 }

            // Pad start with space
            let s4: std.text.String = "42";
            let padded = s4.pad(start: 6, with: ' ');
            if padded.equals("    42") == false { return 8 }
            if padded.byteCount != 6 { return 9 }

            // Pad end with space
            let padded2 = s4.pad(end: 6, with: ' ');
            if padded2.equals("42    ") == false { return 10 }
            if padded2.byteCount != 6 { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn string_protocol_methods() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- compare() ----
            let a: std.text.String = "apple";
            let b: std.text.String = "banana";
            let cmp = a.compare(b);
            if cmp != std.core.Ordering.Less { return 1 }

            let cmp2 = b.compare(a);
            if cmp2 != std.core.Ordering.Greater { return 2 }

            let cmp3 = a.compare(a);
            if cmp3 != std.core.Ordering.Equal { return 3 }

            // ---- clone() ----
            let original: std.text.String = "hello";
            let cloned = original.clone();
            if cloned.equals("hello") == false { return 4 }

            // clone is COW - mutating clone doesn't affect original
            var mClone = original.clone();
            mClone.append(" world");
            if original.byteCount != 5 { return 5 }
            if mClone.byteCount != 11 { return 6 }

            // ---- add() ----
            let s1: std.text.String = "hello";
            let s2: std.text.String = " world";
            let combined = s1.add(s2);
            if combined.equals("hello world") == false { return 7 }
            // Originals unchanged
            if s1.byteCount != 5 { return 8 }
            if s2.byteCount != 6 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_access_variants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "abcde";

            // char(unchecked:) - same as char(at:)
            let c0 = s.char(unchecked: 0);
            if c0.equals('a') == false { return 1 }
            let c4 = s.char(unchecked: 4);
            if c4.equals('e') == false { return 2 }

            // char(wrapping:) - positive index
            let cw0 = s.char(wrapping: 0);
            if cw0.equals('a') == false { return 3 }

            // char(wrapping:) - negative index wraps to last
            let cwNeg1 = s.char(wrapping: -1);
            if cwNeg1.equals('e') == false { return 4 }

            // char(wrapping:) - -2 wraps to second-to-last
            let cwNeg2 = s.char(wrapping: -2);
            if cwNeg2.equals('d') == false { return 5 }

            // char(wrapping:) - overflow wraps around
            let cwOver = s.char(wrapping: 5);
            if cwOver.equals('a') == false { return 6 }

            let cwOver2 = s.char(wrapping: 7);
            if cwOver2.equals('c') == false { return 7 }

            // char(clamping:) - normal index
            let cc1 = s.char(clamping: 2);
            if cc1.equals('c') == false { return 8 }

            // char(clamping:) - negative clamped to 0
            let ccNeg = s.char(clamping: -10);
            if ccNeg.equals('a') == false { return 9 }

            // char(clamping:) - past end clamped to last
            let ccOver = s.char(clamping: 100);
            if ccOver.equals('e') == false { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn unicode_case_mutating() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // lowercase() mutating
            var s1: std.text.String = "HELLO WORLD";
            s1.lowercase();
            if s1.equals("hello world") == false { return 1 }

            // uppercase() mutating
            var s2: std.text.String = "hello world";
            s2.uppercase();
            if s2.equals("HELLO WORLD") == false { return 2 }

            // lowercase on already lowercase
            var s3: std.text.String = "already lower";
            s3.lowercase();
            if s3.equals("already lower") == false { return 3 }

            // uppercase on already uppercase
            var s4: std.text.String = "ALREADY UPPER";
            s4.uppercase();
            if s4.equals("ALREADY UPPER") == false { return 4 }

            // lowercase on mixed case
            var s5: std.text.String = "HeLLo WoRLd";
            s5.lowercase();
            if s5.equals("hello world") == false { return 5 }

            // uppercase on mixed case
            var s6: std.text.String = "HeLLo WoRLd";
            s6.uppercase();
            if s6.equals("HELLO WORLD") == false { return 6 }

            // lowercase on empty string
            var s7 = std.text.String();
            s7.lowercase();
            if s7.isEmpty == false { return 7 }

            // uppercase on empty string
            var s8 = std.text.String();
            s8.uppercase();
            if s8.isEmpty == false { return 8 }

            // lowercase preserves non-alpha chars
            var s9: std.text.String = "Hello 123!";
            s9.lowercase();
            if s9.equals("hello 123!") == false { return 9 }

            // uppercase preserves non-alpha chars
            var s10: std.text.String = "Hello 123!";
            s10.uppercase();
            if s10.equals("HELLO 123!") == false { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn string_format_options() {
    Test::new(
        r#"module Test

        func makeOpts(width: std.num.Int64, alignment: std.text.Alignment, fill: std.text.Char) -> std.text.FormatOptions {
            var opts = std.text.FormatOptions();
            opts.width = .Some(width);
            opts.alignment = alignment;
            opts.fill = fill;
            opts
        }

        func main() -> lang.i64 {
            let s: std.text.String = "test";

            // format() with no options returns the string itself
            let plain = s.format();
            if plain.equals("test") == false { return 1 }

            // format with width and left alignment
            let leftPadded = s.format(makeOpts(10, std.text.Alignment.Left, ' '));
            if leftPadded.equals("test      ") == false { return 2 }
            if leftPadded.count != 10 { return 3 }

            // format with width and right alignment
            let rightPadded = s.format(makeOpts(10, std.text.Alignment.Right, ' '));
            if rightPadded.equals("      test") == false { return 4 }
            if rightPadded.count != 10 { return 5 }

            // format with width and center alignment
            let centerPadded = s.format(makeOpts(10, std.text.Alignment.Center, ' '));
            if centerPadded.equals("   test   ") == false { return 6 }
            if centerPadded.count != 10 { return 7 }

            // format when string is already wider than width
            let noChange = s.format(makeOpts(2, std.text.Alignment.Left, ' '));
            if noChange.equals("test") == false { return 8 }

            // format with custom fill character
            let customFill = s.format(makeOpts(8, std.text.Alignment.Right, '-'));
            if customFill.equals("----test") == false { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn string_hash() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.text.String = "hello";
            let b: std.text.String = "world";

            // Hash different strings into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different strings should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Equal strings constructed independently should hash identically
            let a2: std.text.String = "hello";
            var hasher4 = std.collections.DefaultHasher();
            a2.hash(into: hasher4);
            let hashA3 = hasher4.finish();
            if hashA != hashA3 { return 3 }

            // Empty string should produce a valid hash
            let empty = std.text.String();
            var hasher5 = std.collections.DefaultHasher();
            empty.hash(into: hasher5);
            let hashEmpty = hasher5.finish();

            // Empty and non-empty should differ
            if hashEmpty == hashA { return 4 }

            // Strings that differ by one character should hash differently
            let c: std.text.String = "hellp";
            var hasher6 = std.collections.DefaultHasher();
            c.hash(into: hasher6);
            let hashC = hasher6.finish();
            if hashA == hashC { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
