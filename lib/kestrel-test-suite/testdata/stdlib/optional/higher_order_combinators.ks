// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.numeric.Int64] = .Some(10);

            // Test map
            let mapped = some.map({ (x) in x * 2 });
            if mapped.unwrap() != 20 { return 1 }

            // Test filter
            let filtered = some.filter({ (x) in x > 5 });
            if filtered.isNone() { return 2 }
            let filteredOut = some.filter({ (x) in x > 100 });
            if filteredOut.isSome() { return 3 }

            // Test flatMap
            let flatMapped = some.flatMap[std.numeric.Int64]({ (x) in .Some(x + 1) });
            if flatMapped.unwrap() != 11 { return 4 }

            0
        }
