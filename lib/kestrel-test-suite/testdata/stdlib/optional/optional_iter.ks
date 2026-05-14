// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test iter on Some - yields 1 element
            let someOpt: std.result.Optional[std.numeric.Int64] = .Some(42);
            var iter = someOpt.iter();
            let first = iter.next();
            if first.isNone() { return 1 }
            if first.unwrap() != 42 { return 2 }

            // Second call should return None
            let second = iter.next();
            if second.isSome() { return 3 }

            // Test iter on None - yields 0 elements
            let none: std.result.Optional[std.numeric.Int64] = .None;
            var iter2 = none.iter();
            let noneFirst = iter2.next();
            if noneFirst.isSome() { return 4 }

            0
        }
