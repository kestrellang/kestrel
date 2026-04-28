// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);

            // flatMap: transform returns Set, results are unioned
            let result = s.flatMap({ (x) in
                var inner = std.collections.Set[std.numeric.Int64]();
                let _ = inner.insert(x);
                let _ = inner.insert(x * 10);
                inner
            });
            // {1} union {10} union {2} union {20} = {1, 2, 10, 20}
            if result.count != 4 { return 1 }
            if result.contains(1) == false { return 2 }
            if result.contains(2) == false { return 3 }
            if result.contains(10) == false { return 4 }
            if result.contains(20) == false { return 5 }

            // flatMap with overlapping sets
            var s2 = std.collections.Set[std.numeric.Int64]();
            let _ = s2.insert(1);
            let _ = s2.insert(2);
            let _ = s2.insert(3);
            let overlap = s2.flatMap({ (x) in
                var inner = std.collections.Set[std.numeric.Int64]();
                let _ = inner.insert(x);
                let _ = inner.insert(x + 1);
                inner
            });
            // {1,2} union {2,3} union {3,4} = {1, 2, 3, 4}
            if overlap.count != 4 { return 6 }
            if overlap.contains(1) == false { return 7 }
            if overlap.contains(4) == false { return 8 }

            // flatMap on empty set
            let empty = std.collections.Set[std.numeric.Int64]();
            let emptyResult = empty.flatMap({ (x) in
                var inner = std.collections.Set[std.numeric.Int64]();
                let _ = inner.insert(x);
                inner
            });
            if emptyResult.count != 0 { return 9 }

            // flatMap where transform returns empty sets
            let emptyInner = s.flatMap({ (x) in
                std.collections.Set[std.numeric.Int64]()
            });
            if emptyInner.count != 0 { return 10 }

            0
        }
