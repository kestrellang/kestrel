// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // ---- digitValue() ----
            let zero: std.text.Char = '0';
            let dv0 = zero.digitValue();
            if dv0.isNone() { return 1 }
            if dv0.unwrap() != 0 { return 2 }

            let five: std.text.Char = '5';
            let dv5 = five.digitValue();
            if dv5.isNone() { return 3 }
            if dv5.unwrap() != 5 { return 4 }

            let nine: std.text.Char = '9';
            let dv9 = nine.digitValue();
            if dv9.isNone() { return 5 }
            if dv9.unwrap() != 9 { return 6 }

            // Non-digit returns None
            let a: std.text.Char = 'a';
            if a.digitValue().isSome() { return 7 }

            // ---- Char(fromDigit:) ----
            let c0 = std.text.Char(fromDigit: 0);
            if c0.isNone() { return 8 }
            if c0.unwrap().isEqual(to: '0') == false { return 9 }

            let c7 = std.text.Char(fromDigit: 7);
            if c7.isNone() { return 10 }
            if c7.unwrap().isEqual(to: '7') == false { return 11 }

            // Out of range returns None
            let c10 = std.text.Char(fromDigit: 10);
            if c10.isSome() { return 12 }

            0
        }
