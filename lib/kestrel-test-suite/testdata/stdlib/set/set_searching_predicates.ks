// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

            // Test contains(where:) - found
            if s.contains(where: { (x) in x > 3 }) == false { return 1 }

            // Test contains(where:) - not found
            if s.contains(where: { (x) in x > 10 }) { return 2 }

            // Test first(where:) - found
            let found = s.first(where: { (x) in x > 4 });
            if found.isNone() { return 3 }
            if found.unwrap() != 5 { return 4 }

            // Test first(where:) - not found
            let notFound = s.first(where: { (x) in x > 100 });
            if notFound.isSome() { return 5 }

            // Test all(where:) - true
            if s.all(where: { (x) in x > 0 }) == false { return 6 }

            // Test all(where:) - false
            if s.all(where: { (x) in x > 2 }) { return 7 }

            // Test all(where:) - empty set (vacuous truth)
            let empty = std.collections.Set[std.numeric.Int64]();
            if empty.all(where: { (x) in false }) == false { return 8 }

            // Test any(where:) - true
            if s.any(where: { (x) in x == 3 }) == false { return 9 }

            // Test any(where:) - false
            if s.any(where: { (x) in x > 100 }) { return 10 }

            // Test any(where:) - empty set
            if empty.any(where: { (x) in true }) { return 11 }

            // Test countWhere(predicate:)
            let evenCount = s.countItems(where: { (x) in x % 2 == 0 });
            if evenCount != 2 { return 12 }

            // countWhere with all matching
            let allCount = s.countItems(where: { (x) in x > 0 });
            if allCount != 5 { return 13 }

            // countWhere with none matching
            let noneCount = s.countItems(where: { (x) in x > 100 });
            if noneCount != 0 { return 14 }

            0
        }
