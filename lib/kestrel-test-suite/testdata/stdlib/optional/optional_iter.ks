// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            // Test iter on Some - yields 1 element
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            var iter = some.iter();
            let first = iter.next();
            if first.isNone() { return 1 }
            if first.unwrap() != 42 { return 2 }

            // Second call should return None
            let second = iter.next();
            if second.isSome() { return 3 }

            // Test iter on None - yields 0 elements
            let none: std.result.Optional[std.num.Int64] = .None;
            var iter2 = none.iter();
            let noneFirst = iter2.next();
            if noneFirst.isSome() { return 4 }

            0
        }
