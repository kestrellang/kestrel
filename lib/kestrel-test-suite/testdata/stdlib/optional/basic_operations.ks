// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Test isSome/isNone
            if someOpt.isSome() == false { return 1 }
            if someOpt.isNone() { return 2 }
            if none.isSome() { return 3 }
            if none.isNone() == false { return 4 }

            // Test unwrap
            if someOpt.unwrap() != 42 { return 5 }

            // Test unwrap(or:)
            if someOpt.unwrap(or: 0) != 42 { return 6 }
            if none.unwrap(or: 99) != 99 { return 7 }

            0
        }
