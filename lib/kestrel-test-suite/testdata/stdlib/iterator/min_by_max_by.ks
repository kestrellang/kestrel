// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            // Test minBy - find element with minimum key
            var pairs = std.collections.Array[(std.numeric.Int64, std.numeric.Int64)]();
            pairs.append((1, 30));
            pairs.append((2, 10));
            pairs.append((3, 20));

            let minPair = pairs.iter().min(byKey:{ (p) in p.1 });
            if minPair.isNone() { return 1 }
            let minVal = minPair.unwrap();
            if minVal.0 != 2 { return 2 }
            if minVal.1 != 10 { return 3 }

            // Test maxBy - find element with maximum key
            let maxPair = pairs.iter().max(byKey:{ (p) in p.1 });
            if maxPair.isNone() { return 4 }
            let maxVal = maxPair.unwrap();
            if maxVal.0 != 1 { return 5 }
            if maxVal.1 != 30 { return 6 }

            // minBy on empty
            let emptyPairs = std.collections.Array[(std.numeric.Int64, std.numeric.Int64)]();
            let emptyMin = emptyPairs.iter().min(byKey:{ (p) in p.1 });
            if emptyMin.isSome() { return 7 }

            // maxBy on empty
            let emptyMax = emptyPairs.iter().max(byKey:{ (p) in p.1 });
            if emptyMax.isSome() { return 8 }

            // minBy on single element
            var singleArr = std.collections.Array[(std.numeric.Int64, std.numeric.Int64)]();
            singleArr.append((42, 99));
            let singleMin = singleArr.iter().min(byKey:{ (p) in p.1 });
            if singleMin.isNone() { return 9 }
            if singleMin.unwrap().0 != 42 { return 10 }

            0
        }
