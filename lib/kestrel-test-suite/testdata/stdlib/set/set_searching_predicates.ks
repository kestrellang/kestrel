// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

            // Test contains(matching:) - found
            if s.contains(matching: { (x) in x > 3 }) == false { return 1 }

            // Test contains(matching:) - not found
            if s.contains(matching: { (x) in x > 10 }) { return 2 }

            // Test first(matching:) - found
            let found = s.first(matching: { (x) in x > 4 });
            if found.isNone() { return 3 }
            if found.unwrap() != 5 { return 4 }

            // Test first(matching:) - not found
            let notFound = s.first(matching: { (x) in x > 100 });
            if notFound.isSome() { return 5 }

            // Test all(satisfy:) - true
            if s.all(satisfying: { (x) in x > 0 }) == false { return 6 }

            // Test all(satisfy:) - false
            if s.all(satisfying: { (x) in x > 2 }) { return 7 }

            // Test all(satisfy:) - empty set (vacuous truth)
            let empty = std.collections.Set[std.num.Int64]();
            if empty.all(satisfying: { (x) in false }) == false { return 8 }

            // Test any(satisfy:) - true
            if s.any(satisfying: { (x) in x == 3 }) == false { return 9 }

            // Test any(satisfy:) - false
            if s.any(satisfying: { (x) in x > 100 }) { return 10 }

            // Test any(satisfy:) - empty set
            if empty.any(satisfying: { (x) in true }) { return 11 }

            // Test countWhere(predicate:)
            let evenCount = s.countItems(matching: { (x) in x % 2 == 0 });
            if evenCount != 2 { return 12 }

            // countWhere with all matching
            let allCount = s.countItems(matching: { (x) in x > 0 });
            if allCount != 5 { return 13 }

            // countWhere with none matching
            let noneCount = s.countItems(matching: { (x) in x > 100 });
            if noneCount != 0 { return 14 }

            0
        }
