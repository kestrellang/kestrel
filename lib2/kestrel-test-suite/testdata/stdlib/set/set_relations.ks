// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            var small = std.collections.Set[std.num.Int64]();
            let _ = small.insert(1);
            let _ = small.insert(2);

            var big = std.collections.Set[std.num.Int64]();
            let _ = big.insert(1);
            let _ = big.insert(2);
            let _ = big.insert(3);

            var equal = std.collections.Set[std.num.Int64]();
            let _ = equal.insert(1);
            let _ = equal.insert(2);

            var disjoint = std.collections.Set[std.num.Int64]();
            let _ = disjoint.insert(10);
            let _ = disjoint.insert(20);

            // Test isSubset(of:)
            if small.isSubset(of: big) == false { return 1 }
            if big.isSubset(of: small) { return 2 }
            // A set is a subset of itself
            if small.isSubset(of: equal) == false { return 3 }

            // Test isStrictSubset(of:)
            if small.isStrictSubset(of: big) == false { return 4 }
            // Not a strict subset of equal set
            if small.isStrictSubset(of: equal) { return 5 }

            // Test isSuperset(of:)
            if big.isSuperset(of: small) == false { return 6 }
            if small.isSuperset(of: big) { return 7 }
            // A set is a superset of itself
            if small.isSuperset(of: equal) == false { return 8 }

            // Test isStrictSuperset(of:)
            if big.isStrictSuperset(of: small) == false { return 9 }
            if big.isStrictSuperset(of: big) { return 10 }

            // Test isDisjoint(with:)
            if small.isDisjoint(with: disjoint) == false { return 11 }
            if small.isDisjoint(with: big) { return 12 }

            // Test empty set relations
            var empty = std.collections.Set[std.num.Int64]();
            if empty.isSubset(of: small) == false { return 13 }
            if empty.isDisjoint(with: small) == false { return 14 }

            0
        }
