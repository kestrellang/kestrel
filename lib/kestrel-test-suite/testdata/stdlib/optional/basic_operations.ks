// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.numeric.Int64] = .Some(42);
            let none: std.result.Optional[std.numeric.Int64] = .None;

            // Test isSome/isNone
            if some.isSome() == false { return 1 }
            if some.isNone() { return 2 }
            if none.isSome() { return 3 }
            if none.isNone() == false { return 4 }

            // Test unwrap
            if some.unwrap() != 42 { return 5 }

            // Test unwrapOr
            if some.unwrapOr(0) != 42 { return 6 }
            if none.unwrapOr(99) != 99 { return 7 }

            0
        }
