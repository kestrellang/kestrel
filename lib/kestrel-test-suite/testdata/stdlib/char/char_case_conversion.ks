// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ---- uppercased() ----
            let a: std.text.Char = 'a';
            let upper = a.uppercased();
            if upper.isEqual(to: 'A') == false { return 1 }

            // ---- lowercased() ----
            let big: std.text.Char = 'Z';
            let lower = big.lowercased();
            if lower.isEqual(to: 'z') == false { return 2 }

            // Already uppercase
            let big2: std.text.Char = 'A';
            if big2.uppercased().isEqual(to: 'A') == false { return 3 }

            // Already lowercase
            if a.lowercased().isEqual(to: 'a') == false { return 4 }

            // Non-letter stays the same
            let digit: std.text.Char = '5';
            if digit.uppercased().isEqual(to: '5') == false { return 5 }
            if digit.lowercased().isEqual(to: '5') == false { return 6 }

            // ---- titlecased() ----
            let tc = a.titlecased();
            if tc.isEqual(to: 'A') == false { return 7 }

            0
        }
