// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let s: std.text.String = "abcde";

            // chars()(i) - panic-on-oob index
            let c0 = s.chars(0);
            if c0.isEqual(to: 'a') == false { return 1 }
            let c4 = s.chars(4);
            if c4.isEqual(to: 'e') == false { return 2 }

            // chars()(clamped: i) - normal index (returns Char?, .None on empty view)
            let cc1 = s.chars(clamped: 2);
            if cc1.unwrap().isEqual(to: 'c') == false { return 8 }

            // chars()(clamped: i) - negative clamped to 0
            let ccNeg = s.chars(clamped: -10);
            if ccNeg.unwrap().isEqual(to: 'a') == false { return 9 }

            // chars()(clamped: i) - past end clamped to last
            let ccOver = s.chars(clamped: 100);
            if ccOver.unwrap().isEqual(to: 'e') == false { return 10 }

            0
        }
