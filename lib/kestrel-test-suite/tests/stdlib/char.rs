use kestrel_test_suite::*;

#[test]
fn char_classification() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- isAscii() ----
            let a: std.text.Char = 'a';
            if a.isAscii() == false { return 1 }

            // ---- isAlphabetic() ----
            if a.isAlphabetic() == false { return 2 }
            let z: std.text.Char = 'Z';
            if z.isAlphabetic() == false { return 3 }
            let digit: std.text.Char = '5';
            if digit.isAlphabetic() { return 4 }

            // ---- isDigit() ----
            if digit.isDigit() == false { return 5 }
            let zero: std.text.Char = '0';
            if zero.isDigit() == false { return 6 }
            let nine: std.text.Char = '9';
            if nine.isDigit() == false { return 7 }
            if a.isDigit() { return 8 }

            // ---- isAlphanumeric() ----
            if a.isAlphanumeric() == false { return 9 }
            if digit.isAlphanumeric() == false { return 10 }
            let space: std.text.Char = ' ';
            if space.isAlphanumeric() { return 11 }

            // ---- isWhitespace() ----
            if space.isWhitespace() == false { return 12 }
            let tab: std.text.Char = '\t';
            if tab.isWhitespace() == false { return 13 }
            let newline: std.text.Char = '\n';
            if newline.isWhitespace() == false { return 14 }
            if a.isWhitespace() { return 15 }

            // ---- isControl() ----
            let nul: std.text.Char = '\0';
            if nul.isControl() == false { return 16 }
            if newline.isControl() == false { return 17 }
            if a.isControl() { return 18 }

            // ---- isUppercase() / isLowercase() ----
            if a.isLowercase() == false { return 19 }
            if a.isUppercase() { return 20 }
            if z.isUppercase() == false { return 21 }
            if z.isLowercase() { return 22 }
            if digit.isUppercase() { return 23 }
            if digit.isLowercase() { return 24 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_case_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- toUppercase() ----
            let a: std.text.Char = 'a';
            let upper = a.toUppercase();
            if upper.equals('A') == false { return 1 }

            // ---- toLowercase() ----
            let big: std.text.Char = 'Z';
            let lower = big.toLowercase();
            if lower.equals('z') == false { return 2 }

            // Already uppercase
            let big2: std.text.Char = 'A';
            if big2.toUppercase().equals('A') == false { return 3 }

            // Already lowercase
            if a.toLowercase().equals('a') == false { return 4 }

            // Non-letter stays the same
            let digit: std.text.Char = '5';
            if digit.toUppercase().equals('5') == false { return 5 }
            if digit.toLowercase().equals('5') == false { return 6 }

            // ---- toTitlecase() ----
            let tc = a.toTitlecase();
            if tc.equals('A') == false { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_utf8_length() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ASCII char = 1 byte
            let a: std.text.Char = 'a';
            if a.utf8Length() != 1 { return 1 }

            // Space = 1 byte
            let space: std.text.Char = ' ';
            if space.utf8Length() != 1 { return 2 }

            // DEL (0x7F) = 1 byte
            let del: std.text.Char = '\x7F';
            if del.utf8Length() != 1 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_digit_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- digitValue() ----
            let zero: std.text.Char = '0';
            let dv0 = zero.digitValue();
            if dv0.isNone() { return 1 }
            if dv0.unwrap() != std.num.UInt32(intLiteral: 0) { return 2 }

            let five: std.text.Char = '5';
            let dv5 = five.digitValue();
            if dv5.isNone() { return 3 }
            if dv5.unwrap() != std.num.UInt32(intLiteral: 5) { return 4 }

            let nine: std.text.Char = '9';
            let dv9 = nine.digitValue();
            if dv9.isNone() { return 5 }
            if dv9.unwrap() != std.num.UInt32(intLiteral: 9) { return 6 }

            // Non-digit returns None
            let a: std.text.Char = 'a';
            if a.digitValue().isSome() { return 7 }

            // ---- fromDigit() ----
            let c0 = std.text.Char.fromDigit(std.num.UInt32(intLiteral: 0));
            if c0.isNone() { return 8 }
            if c0.unwrap().equals('0') == false { return 9 }

            let c7 = std.text.Char.fromDigit(std.num.UInt32(intLiteral: 7));
            if c7.isNone() { return 10 }
            if c7.unwrap().equals('7') == false { return 11 }

            // Out of range returns None
            let c10 = std.text.Char.fromDigit(std.num.UInt32(intLiteral: 10));
            if c10.isSome() { return 12 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_equality_and_comparison() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.text.Char = 'a';
            let b: std.text.Char = 'b';
            let a2: std.text.Char = 'a';

            // ---- equals() ----
            if a.equals(a2) == false { return 1 }
            if a.equals(b) { return 2 }

            // ---- compare() ----
            let cmp = a.compare(b);
            if cmp != std.core.Ordering.Less { return 3 }

            let cmp2 = b.compare(a);
            if cmp2 != std.core.Ordering.Greater { return 4 }

            let cmp3 = a.compare(a2);
            if cmp3 != std.core.Ordering.Equal { return 5 }

            // ---- matches() ----
            if a.matches(a2) == false { return 6 }
            if a.matches(b) { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_init_value_and_value() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Construct Char from UInt32 code point and read it back
            let cp = std.num.UInt32(intLiteral: 65);  // 'A' = U+0041 = 65
            let c = std.text.Char(cp);

            // value() should return the same code point
            if c.value() != cp { return 1 }

            // Should behave identically to a char literal 'A'
            if c.equals('A') == false { return 2 }

            // Try a non-ASCII code point: U+00E9 = 233 (e-acute)
            let cp2 = std.num.UInt32(intLiteral: 233);
            let c2 = std.text.Char(cp2);
            if c2.value() != cp2 { return 3 }

            // ASCII '0' = 48
            let cp3 = std.num.UInt32(intLiteral: 48);
            let c3 = std.text.Char(cp3);
            if c3.equals('0') == false { return 4 }
            if c3.value() != cp3 { return 5 }

            // Null character = 0
            let cp4 = std.num.UInt32(intLiteral: 0);
            let c4 = std.text.Char(cp4);
            if c4.equals('\0') == false { return 6 }
            if c4.value() != cp4 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn char_hash() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.text.Char = 'a';
            let b: std.text.Char = 'b';

            // Hash 'a' and 'b' into separate hashers, verify they produce different values
            var hasher1 = std.collections.DefaultHasher();
            a.hash(into: hasher1);
            let hashA = hasher1.finish();

            var hasher2 = std.collections.DefaultHasher();
            b.hash(into: hasher2);
            let hashB = hasher2.finish();

            // Different chars should hash to different values
            if hashA == hashB { return 1 }

            // Same value should hash to same result (deterministic)
            var hasher3 = std.collections.DefaultHasher();
            a.hash(into: hasher3);
            let hashA2 = hasher3.finish();
            if hashA != hashA2 { return 2 }

            // Equal chars constructed differently should hash identically
            let a2 = std.text.Char(std.num.UInt32(intLiteral: 97));  // 'a' = 97
            var hasher4 = std.collections.DefaultHasher();
            a2.hash(into: hasher4);
            let hashA3 = hasher4.finish();
            if hashA != hashA3 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn grapheme_chars() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Single-char grapheme: chars() returns array with one element
            let g1 = std.text.Grapheme(char: 'x');
            let c1 = g1.chars();
            if c1.count != 1 { return 1 }
            if c1(unchecked: 0).equals('x') == false { return 2 }

            // Multi-char grapheme: chars() returns the array of chars
            var arr = std.collections.Array[std.text.Char]();
            arr.append('a');
            arr.append('b');
            arr.append('c');
            let g2 = std.text.Grapheme(chars: arr);
            let c2 = g2.chars();
            if c2.count != 3 { return 3 }
            if c2(unchecked: 0).equals('a') == false { return 4 }
            if c2(unchecked: 1).equals('b') == false { return 5 }
            if c2(unchecked: 2).equals('c') == false { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn grapheme_basic() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- Grapheme from single char ----
            let g = std.text.Grapheme(char: 'a');
            if g.charCount() != 1 { return 1 }

            // ---- firstChar() ----
            let fc = g.firstChar();
            if fc.isNone() { return 2 }
            if fc.unwrap().equals('a') == false { return 3 }

            // ---- isAscii() ----
            if g.isAscii() == false { return 4 }

            // ---- utf8Length() ----
            if g.utf8Length() != 1 { return 5 }

            // ---- equals() ----
            let g2 = std.text.Grapheme(char: 'a');
            if g.equals(g2) == false { return 6 }

            let g3 = std.text.Grapheme(char: 'b');
            if g.equals(g3) { return 7 }

            // ---- Grapheme from multiple chars ----
            var chars = std.collections.Array[std.text.Char]();
            chars.append('a');
            chars.append('b');
            let gMulti = std.text.Grapheme(chars: chars);
            if gMulti.charCount() != 2 { return 8 }
            if gMulti.firstChar().unwrap().equals('a') == false { return 9 }
            if gMulti.utf8Length() != 2 { return 10 }

            // Multi-char grapheme is not ASCII
            if gMulti.isAscii() { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
