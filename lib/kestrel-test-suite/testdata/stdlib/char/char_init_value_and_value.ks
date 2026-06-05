// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Construct Char from UInt32 code point (unchecked) and read it back
            let cp = 65;  // 'A' = U+0041 = 65
            let c = std.text.Char(unchecked: cp);

            // value() should return the same code point
            if c.value() != cp { return 1 }

            // Should behave identically to a char literal 'A'
            if c.isEqual(to: 'A') == false { return 2 }

            // Try a non-ASCII code point: U+00E9 = 233 (e-acute)
            let cp2 = 233;
            let c2 = std.text.Char(unchecked: cp2);
            if c2.value() != cp2 { return 3 }

            // ASCII '0' = 48
            let cp3 = 48;
            let c3 = std.text.Char(unchecked: cp3);
            if c3.isEqual(to: '0') == false { return 4 }
            if c3.value() != cp3 { return 5 }

            // Null character = 0
            let cp4 = 0;
            let c4 = std.text.Char(unchecked: cp4);
            if c4.isEqual(to: '\0') == false { return 6 }
            if c4.value() != cp4 { return 7 }

            // Failable init: valid value succeeds
            let valid = std.text.Char(65);
            if valid.isNone() { return 8 }
            if valid.unwrap().isEqual(to: 'A') == false { return 9 }

            // Failable init: surrogate rejected
            let surrogate = std.text.Char(0xD800);
            if surrogate.isSome() { return 10 }

            // Failable init: above U+10FFFF rejected
            let tooBig = std.text.Char(0x110000);
            if tooBig.isSome() { return 11 }

            0
        }
