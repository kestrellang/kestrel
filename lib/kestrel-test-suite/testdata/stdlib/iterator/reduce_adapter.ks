// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // ---- reduce() ----
            let sum = [1, 2, 3, 4].iter().reduce(by: { (a, b) in a + b });
            if sum.isNone() { return 1 }
            if sum.unwrap() != 10 { return 2 }

            // reduce on single element
            let single = [42].iter().reduce(by: { (a, b) in a + b });
            if single.isNone() { return 3 }
            if single.unwrap() != 42 { return 4 }

            // reduce on empty returns None
            let empty = std.collections.Array[std.numeric.Int64]();
            let none = empty.iter().reduce(by: { (a, b) in a + b });
            if none.isSome() { return 5 }

            // reduce for max
            let maxVal = [3, 1, 4, 1, 5].iter().reduce(by: { (a, b) in if a > b { a } else { b } });
            if maxVal.unwrap() != 5 { return 6 }

            0
        }
