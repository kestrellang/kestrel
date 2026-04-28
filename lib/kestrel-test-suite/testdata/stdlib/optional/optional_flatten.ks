// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test flatten on Some(Some(value))
            let nested: std.result.Optional[std.result.Optional[std.numeric.Int64]] = .Some(.Some(42));
            let flat = nested.flatten();
            if flat.isNone() { return 1 }
            if flat.unwrap() != 42 { return 2 }

            // Test flatten on Some(None)
            let someNone: std.result.Optional[std.result.Optional[std.numeric.Int64]] = .Some(.None);
            let flat2 = someNone.flatten();
            if flat2.isSome() { return 3 }

            // Test flatten on None
            let none: std.result.Optional[std.result.Optional[std.numeric.Int64]] = .None;
            let flat3 = none.flatten();
            if flat3.isSome() { return 4 }

            0
        }
