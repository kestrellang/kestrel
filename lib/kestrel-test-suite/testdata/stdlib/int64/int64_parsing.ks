// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let p1 = std.numeric.Int64(parsing: "42", radix: 10);
            if p1.isNone() { return 1 }
            if p1.unwrap() != 42 { return 2 }

            let p7 = std.numeric.Int64(parsing: "ff", radix: 16);
            if p7.isNone() { return 3 }
            if p7.unwrap() != 255 { return 4 }

            // #170: the no-radix init must reach minValue. Its magnitude is
            // maxValue+1, so a signed accumulator capped at maxValue rejects it.
            let pmin = std.numeric.Int64(parsing: "-9223372036854775808");
            if pmin.isNone() { return 5 }
            if pmin.unwrap() != std.numeric.Int64.minValue { return 6 }

            let pmax = std.numeric.Int64(parsing: "9223372036854775807");
            if pmax.isNone() { return 7 }
            if pmax.unwrap() != std.numeric.Int64.maxValue { return 8 }

            // Just past the ends must still be rejected.
            if std.numeric.Int64(parsing: "9223372036854775808").isSome() { return 9 }
            if std.numeric.Int64(parsing: "-9223372036854775809").isSome() { return 10 }

            0
        }
