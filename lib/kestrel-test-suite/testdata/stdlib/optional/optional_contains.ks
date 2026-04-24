// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Some(42) contains 42
            if some.contains(42) == false { return 1 }

            // Some(42) does not contain 99
            if some.contains(99) { return 2 }

            // None does not contain anything
            if none.contains(42) { return 3 }

            0
        }
