// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let p1 = std.numeric.Int64.parse("42", 10);
            if p1.isNone() { return 1 }
            if p1.unwrap() != 42 { return 2 }

            let p7 = std.numeric.Int64.parse("ff", 16);
            if p7.isNone() { return 3 }
            if p7.unwrap() != 255 { return 4 }

            0
        }
