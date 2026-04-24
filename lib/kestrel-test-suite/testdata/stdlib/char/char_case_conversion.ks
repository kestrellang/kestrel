// test: execution
// stdlib: true

module Test

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
