// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let a: std.numeric.Int64 = 42;
            let b: std.numeric.Int64 = 42;
            let c: std.numeric.Int64 = 43;

            // equals with same value
            if a.isEqual(to: b) == false { return 1 }

            // equals with different value
            if a.isEqual(to: c) { return 2 }

            // equals with zero
            let zero: std.numeric.Int64 = 0;
            if zero.isEqual(to: 0) == false { return 3 }
            if zero.isEqual(to: 1) { return 4 }

            // equals with negative values
            let neg: std.numeric.Int64 = -10;
            if neg.isEqual(to: -10) == false { return 5 }
            if neg.isEqual(to: 10) { return 6 }

            // equals is symmetric
            if a.isEqual(to: b) != b.isEqual(to: a) { return 7 }

            0
        }
