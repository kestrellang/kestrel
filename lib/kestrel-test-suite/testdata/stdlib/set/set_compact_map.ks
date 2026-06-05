// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            var s = std.collections.Set[std.numeric.Int64]();
             s.insert(1);
             s.insert(2);
             s.insert(3);
             s.insert(4);
             s.insert(5);

            // compactMap: transform returns Optional, keeps only Somes
            // Keep even numbers doubled, discard odd numbers
            let result = s.compactMap({ (x) in
                if x % 2 == 0 {
                    .Some(x * 2)
                } else {
                    .None
                }
            });
            if result.count != 2 { return 1 }
            if result.contains(4) == false { return 2 }  // 2 * 2
            if result.contains(8) == false { return 3 }  // 4 * 2
            if result.contains(1) { return 4 }
            if result.contains(2) { return 5 }

            // compactMap where all return None
            let allNone = s.compactMap({ (x) in
                let r: std.numeric.Int64? = .None;
                r
            });
            if allNone.count != 0 { return 6 }

            // compactMap where all return Some
            let allSome = s.compactMap({ (x) in .Some(x) });
            if allSome.count != 5 { return 7 }

            // compactMap on empty set
            let empty = std.collections.Set[std.numeric.Int64]();
            let emptyResult = empty.compactMap({ (x) in .Some(x * 10) });
            if emptyResult.count != 0 { return 8 }

            // compactMap that produces duplicate values (set deduplicates)
            // All values map to the same value
            let collapsed = s.compactMap({ (x) in .Some(1) });
            if collapsed.count != 1 { return 9 }
            if collapsed.contains(1) == false { return 10 }

            0
        }
