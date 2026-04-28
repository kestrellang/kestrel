// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let a: std.numeric.Int64 = 10;
            let b: std.numeric.Int64 = 3;

            // Test arithmetic
            if a.add(b) != 13 { return 1 }
            if a.subtract(b) != 7 { return 2 }
            if a.multiply(b) != 30 { return 3 }
            if a.divide(b) != 3 { return 4 }
            if a.modulo(b) != 1 { return 5 }

            // Test negate and abs
            let neg: std.numeric.Int64 = -5;
            if neg.negate() != 5 { return 6 }
            if neg.abs() != 5 { return 7 }

            // Test comparison - a > b so compare should return Greater
            let cmp = a.compare(b);
            match cmp {
                .Greater => 0,
                _ => 8
            }
        }
