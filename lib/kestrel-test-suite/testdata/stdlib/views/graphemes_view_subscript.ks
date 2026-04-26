// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "abcde";

            // ---- view(i) ----
            let g0 = s.graphemes(0);
            if g0.firstChar().unwrap().equals('a') == false { return 1 }
            let g4 = s.graphemes(4);
            if g4.firstChar().unwrap().equals('e') == false { return 2 }

            // ---- view(checked: i) ----
            let gc = s.graphemes(checked: 2);
            if gc.isNone() { return 3 }
            if gc.unwrap().firstChar().unwrap().equals('c') == false { return 4 }

            let gOob = s.graphemes(checked: 100);
            if gOob.isSome() { return 5 }

            let gNeg = s.graphemes(checked: -1);
            if gNeg.isSome() { return 6 }

            // ---- view(clamped: i) on non-empty view ----
            let gcl = s.graphemes(clamped: 2);
            if gcl.unwrap().firstChar().unwrap().equals('c') == false { return 7 }

            let gNegClamp = s.graphemes(clamped: -10);
            if gNegClamp.unwrap().firstChar().unwrap().equals('a') == false { return 8 }

            let gOverClamp = s.graphemes(clamped: 100);
            if gOverClamp.unwrap().firstChar().unwrap().equals('e') == false { return 9 }

            // ---- view(clamped:) on empty view returns None ----
            let empty = std.text.String();
            let emptyClamp = empty.graphemes(clamped: 0);
            if emptyClamp.isSome() { return 10 }

            // ---- range subscripts (substring) ----
            // Range[Int64]
            let sub = s.graphemes(std.core.Range[std.num.Int64](0, 3));
            if sub.equals("abc") == false { return 11 }

            let subMid = s.graphemes(std.core.Range[std.num.Int64](1, 4));
            if subMid.equals("bcd") == false { return 12 }

            // checked range - valid
            let subChecked = s.graphemes(checked: std.core.Range[std.num.Int64](0, 5));
            if subChecked.isNone() { return 13 }
            if subChecked.unwrap().equals("abcde") == false { return 14 }

            // checked range - out of bounds
            let subOob = s.graphemes(checked: std.core.Range[std.num.Int64](0, 100));
            if subOob.isSome() { return 15 }

            // checked range - reversed
            let subRev = s.graphemes(checked: std.core.Range[std.num.Int64](4, 2));
            if subRev.isSome() { return 16 }

            // clamped range
            let subClamp = s.graphemes(clamped: std.core.Range[std.num.Int64](-5, 100));
            if subClamp.equals("abcde") == false { return 17 }

            // ClosedRange[Int64]
            let subClosed = s.graphemes(std.core.ClosedRange[std.num.Int64](1, 3));
            if subClosed.equals("bcd") == false { return 18 }

            0
        }
