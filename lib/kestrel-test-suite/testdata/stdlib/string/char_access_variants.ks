// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let s: std.text.String = "abcde";

            // char(unchecked:) - same as char(at:)
            let c0 = s.char(unchecked: 0);
            if c0.equals('a') == false { return 1 }
            let c4 = s.char(unchecked: 4);
            if c4.equals('e') == false { return 2 }

            // char(wrapped:) - positive index
            let cw0 = s.char(wrapped: 0);
            if cw0.equals('a') == false { return 3 }

            // char(wrapped:) - negative index wraps to last
            let cwNeg1 = s.char(wrapped: -1);
            if cwNeg1.equals('e') == false { return 4 }

            // char(wrapped:) - -2 wraps to second-to-last
            let cwNeg2 = s.char(wrapped: -2);
            if cwNeg2.equals('d') == false { return 5 }

            // char(wrapped:) - overflow wraps around
            let cwOver = s.char(wrapped: 5);
            if cwOver.equals('a') == false { return 6 }

            let cwOver2 = s.char(wrapped: 7);
            if cwOver2.equals('c') == false { return 7 }

            // char(clamped:) - normal index
            let cc1 = s.char(clamped: 2);
            if cc1.equals('c') == false { return 8 }

            // char(clamped:) - negative clamped to 0
            let ccNeg = s.char(clamped: -10);
            if ccNeg.equals('a') == false { return 9 }

            // char(clamped:) - past end clamped to last
            let ccOver = s.char(clamped: 100);
            if ccOver.equals('e') == false { return 10 }

            0
        }
