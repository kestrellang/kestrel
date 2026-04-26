// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- uppercased() ----
            let a: std.text.Char = 'a';
            let upper = a.uppercased();
            if upper.equals('A') == false { return 1 }

            // ---- lowercased() ----
            let big: std.text.Char = 'Z';
            let lower = big.lowercased();
            if lower.equals('z') == false { return 2 }

            // Already uppercase
            let big2: std.text.Char = 'A';
            if big2.uppercased().equals('A') == false { return 3 }

            // Already lowercase
            if a.lowercased().equals('a') == false { return 4 }

            // Non-letter stays the same
            let digit: std.text.Char = '5';
            if digit.uppercased().equals('5') == false { return 5 }
            if digit.lowercased().equals('5') == false { return 6 }

            // ---- titlecased() ----
            let tc = a.titlecased();
            if tc.equals('A') == false { return 7 }

            0
        }
