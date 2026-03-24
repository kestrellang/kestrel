// test: execution
// stdlib: true

module Test

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
