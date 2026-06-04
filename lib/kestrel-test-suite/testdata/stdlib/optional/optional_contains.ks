// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Some(42) contains 42
            if someOpt.contains(42) == false { return 1 }

            // Some(42) does not contain 99
            if someOpt.contains(99) { return 2 }

            // None does not contain anything
            if none.contains(42) { return 3 }

            0
        }
