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
