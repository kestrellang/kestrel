use kestrel_test_suite::*;

#[test]
fn bytes_view_basic() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- bytes.count() ----
            if s.bytes.count() != 5 { return 1 }

            // ---- bytes.isEmpty() ----
            if s.bytes.isEmpty() { return 2 }

            // Empty string bytes view
            let empty = std.text.String();
            if empty.bytes.isEmpty() == false { return 3 }
            if empty.bytes.count() != 0 { return 4 }

            // ---- bytes.byteAt() ----
            // 'h' = 104
            let b0 = s.bytes.byteAt(0);
            if b0.isNone() { return 5 }
            let byteH: std.num.UInt8 = 104;
            if b0.unwrap() != byteH { return 6 }

            // 'e' = 101
            let b1 = s.bytes.byteAt(1);
            if b1.isNone() { return 7 }
            let byteE: std.num.UInt8 = 101;
            if b1.unwrap() != byteE { return 8 }

            // Out of bounds returns None
            let bOob = s.bytes.byteAt(100);
            if bOob.isSome() { return 9 }

            // Negative index returns None
            let bNeg = s.bytes.byteAt(-1);
            if bNeg.isSome() { return 10 }

            // ---- bytes.byteAtUnchecked() ----
            // 'o' = 111
            let bu = s.bytes.byteAtUnchecked(4);
            let byteO: std.num.UInt8 = 111;
            if bu != byteO { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn bytes_view_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "abc";

            // ---- bytes.iter() ----
            let byteArr = s.bytes.iter().collect();
            if byteArr.count != 3 { return 1 }
            // 'a' = 97
            let byteA: std.num.UInt8 = 97;
            if byteArr(unchecked: 0) != byteA { return 2 }
            // 'b' = 98
            let byteB: std.num.UInt8 = 98;
            if byteArr(unchecked: 1) != byteB { return 3 }
            // 'c' = 99
            let byteC: std.num.UInt8 = 99;
            if byteArr(unchecked: 2) != byteC { return 4 }

            // Empty string yields empty iter
            let empty = std.text.String();
            let emptyBytes = empty.bytes.iter().collect();
            if emptyBytes.count != 0 { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn bytes_view_substring() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- bytes.substring(from:to:) ----
            let sub = s.bytes.substring(from: 0, to: 5);
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.bytes.substring(from: 6, to: 11);
            if sub2.equals("world") == false { return 2 }

            // ---- bytes.substring(checked:to:) ----
            let checked = s.bytes.substring(checked: 0, to: 5);
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.bytes.substring(checked: 0, to: 100);
            if oob.isSome() { return 5 }

            // Negative start returns None
            let neg = s.bytes.substring(checked: -1, to: 5);
            if neg.isSome() { return 6 }

            // Start > end returns None
            let rev = s.bytes.substring(checked: 5, to: 3);
            if rev.isSome() { return 7 }

            // Empty range
            let empty = s.bytes.substring(checked: 3, to: 3);
            if empty.isNone() { return 8 }
            if empty.unwrap().isEmpty == false { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn chars_view_iter_and_count() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- chars.count() ----
            if s.chars.count() != 5 { return 1 }

            // ---- chars.iter() ----
            let charArr = s.chars.iter().collect();
            if charArr.count != 5 { return 2 }
            if charArr(unchecked: 0).equals('h') == false { return 3 }
            if charArr(unchecked: 4).equals('o') == false { return 4 }

            // Empty string
            let empty = std.text.String();
            if empty.chars.count() != 0 { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Stdlib bug - CharsView.substring(checked:to:) returns None for empty range
// at non-zero offset (e.g., start=3, to=3). The loop increments charIndex past start
// before checking if charIndex == end, so when start == end > 0, foundEnd is never set.
#[test]
fn chars_view_substring() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello world";

            // ---- chars.substring(from:to:) ---- (character indices, not bytes)
            let sub = s.chars.substring(from: 0, to: 5);
            if sub.equals("hello") == false { return 1 }

            let sub2 = s.chars.substring(from: 6, to: 11);
            if sub2.equals("world") == false { return 2 }

            // ---- chars.substring(checked:to:) ----
            let checked = s.chars.substring(checked: 0, to: 5);
            if checked.isNone() { return 3 }
            if checked.unwrap().equals("hello") == false { return 4 }

            // Out of bounds returns None
            let oob = s.chars.substring(checked: 0, to: 100);
            if oob.isSome() { return 5 }

            // Empty range
            let empty = s.chars.substring(checked: 3, to: 3);
            if empty.isNone() { return 6 }
            if empty.unwrap().isEmpty == false { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn graphemes_view() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- graphemes.count() ----
            if s.graphemes.count() != 5 { return 1 }

            // ---- graphemes.iter() ----
            let gs = s.graphemes.iter().collect();
            if gs.count != 5 { return 2 }

            // Each grapheme is a single ASCII char
            let first = gs(unchecked: 0);
            if first.charCount() != 1 { return 3 }
            if first.firstChar().unwrap().equals('h') == false { return 4 }

            let last = gs(unchecked: 4);
            if last.firstChar().unwrap().equals('o') == false { return 5 }

            // Empty string
            let empty = std.text.String();
            if empty.graphemes.count() != 0 { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn lines_view() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- lines.iter() with \n ----
            let s: std.text.String = "hello\nworld\nfoo";
            let lineArr = s.lines.iter().collect();
            if lineArr.count != 3 { return 1 }
            if lineArr(unchecked: 0).equals("hello") == false { return 2 }
            if lineArr(unchecked: 1).equals("world") == false { return 3 }
            if lineArr(unchecked: 2).equals("foo") == false { return 4 }

            // Single line (no newline)
            let single: std.text.String = "just one line";
            let singleLines = single.lines.iter().collect();
            if singleLines.count != 1 { return 5 }
            if singleLines(unchecked: 0).equals("just one line") == false { return 6 }

            // Trailing newline yields empty last line
            let trailing: std.text.String = "a\nb\n";
            let trailingLines = trailing.lines.iter().collect();
            if trailingLines.count != 2 { return 7 }
            if trailingLines(unchecked: 0).equals("a") == false { return 8 }
            if trailingLines(unchecked: 1).equals("b") == false { return 9 }

            // Empty string yields no lines
            let empty = std.text.String();
            let emptyLines = empty.lines.iter().collect();
            if emptyLines.count != 0 { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn string_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let s: std.text.String = "hello";

            // ---- iter() on String ----
            let chars = s.iter().collect();
            if chars.count != 5 { return 1 }
            if chars(unchecked: 0).equals('h') == false { return 2 }
            if chars(unchecked: 4).equals('o') == false { return 3 }

            // iter() with map
            let upper = s.iter().map({ (c) in c.toUppercase() }).collect();
            if upper.count != 5 { return 4 }
            if upper(unchecked: 0).equals('H') == false { return 5 }

            // iter() with filter
            let vowels = s.iter().filter({ (c) in
                c.equals('a') or c.equals('e') or c.equals('i') or c.equals('o') or c.equals('u')
            }).collect();
            if vowels.count != 2 { return 6 }

            // Empty string iter
            let empty = std.text.String();
            if empty.iter().count() != 0 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
