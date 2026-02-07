use kestrel_test_suite::*;

#[test]
fn init_from_iterable() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(2);
            arr.append(3);

            let mySet = std.collections.Set[std.num.Int64](from: arr);
            if mySet.count != 3 { return 1 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_basic() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test init() and isEmpty
            var s = std.collections.Set[std.num.Int64]();
            if s.isEmpty == false { return 1 }
            if s.count != 0 { return 2 }

            // Test insert() - returns true for new element
            let inserted1 = s.insert(10);
            if inserted1 == false { return 3 }
            if s.count != 1 { return 4 }

            // Test insert() - returns false for existing element
            let inserted2 = s.insert(10);
            if inserted2 { return 5 }
            if s.count != 1 { return 6 }

            // Test contains()
            s.insert(20);
            s.insert(30);
            if s.contains(10) == false { return 7 }
            if s.contains(20) == false { return 8 }
            if s.contains(999) { return 9 }

            // Test isEmpty after inserts
            if s.isEmpty { return 10 }

            // Test remove() - returns true for existing element
            let removed1 = s.remove(20);
            if removed1 == false { return 11 }
            if s.count != 2 { return 12 }
            if s.contains(20) { return 13 }

            // Test remove() - returns false for missing element
            let removed2 = s.remove(999);
            if removed2 { return 14 }

            // Test clear()
            s.clear();
            if s.count != 0 { return 15 }
            if s.isEmpty == false { return 16 }
            if s.contains(10) { return 17 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Setup two sets: a = {1, 2, 3}, b = {2, 3, 4}
            var a = std.collections.Set[std.num.Int64]();
            let _ = a.insert(1);
            let _ = a.insert(2);
            let _ = a.insert(3);

            var b = std.collections.Set[std.num.Int64]();
            let _ = b.insert(2);
            let _ = b.insert(3);
            let _ = b.insert(4);

            // Test union() - non-mutating
            let u = a.union(b);
            if u.count != 4 { return 1 }
            if u.contains(1) == false { return 2 }
            if u.contains(4) == false { return 3 }

            // Test intersection() - non-mutating
            let inter = a.intersection(b);
            if inter.count != 2 { return 4 }
            if inter.contains(2) == false { return 5 }
            if inter.contains(3) == false { return 6 }
            if inter.contains(1) { return 7 }

            // Test difference() - non-mutating
            let diff = a.difference(b);
            if diff.count != 1 { return 8 }
            if diff.contains(1) == false { return 9 }
            if diff.contains(2) { return 10 }

            // Test symmetricDifference() - non-mutating
            let symDiff = a.symmetricDifference(b);
            if symDiff.count != 2 { return 11 }
            if symDiff.contains(1) == false { return 12 }
            if symDiff.contains(4) == false { return 13 }
            if symDiff.contains(2) { return 14 }

            // Test formUnion() - mutating
            var fu = std.collections.Set[std.num.Int64]();
            let _ = fu.insert(1);
            let _ = fu.insert(2);
            fu.formUnion(b);
            if fu.count != 4 { return 15 }
            if fu.contains(4) == false { return 16 }

            // Test formIntersection() - mutating
            var fi = std.collections.Set[std.num.Int64]();
            let _ = fi.insert(1);
            let _ = fi.insert(2);
            let _ = fi.insert(3);
            fi.formIntersection(b);
            if fi.count != 2 { return 17 }
            if fi.contains(2) == false { return 18 }
            if fi.contains(1) { return 19 }

            // Test formDifference() - mutating
            var fd = std.collections.Set[std.num.Int64]();
            let _ = fd.insert(1);
            let _ = fd.insert(2);
            let _ = fd.insert(3);
            fd.formDifference(b);
            if fd.count != 1 { return 20 }
            if fd.contains(1) == false { return 21 }

            // Test formSymmetricDifference() - mutating
            var fsd = std.collections.Set[std.num.Int64]();
            let _ = fsd.insert(1);
            let _ = fsd.insert(2);
            let _ = fsd.insert(3);
            fsd.formSymmetricDifference(b);
            if fsd.count != 2 { return 22 }
            if fsd.contains(1) == false { return 23 }
            if fsd.contains(4) == false { return 24 }

            // Original sets unchanged
            if a.count != 3 { return 25 }
            if b.count != 3 { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_relations() {
    Test::new(
        r#"module Test

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
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_transforms() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

            // Test filter(matching:)
            let evens = s.filter(matching: { (x) in x % 2 == 0 });
            if evens.count != 2 { return 1 }
            if evens.contains(2) == false { return 2 }
            if evens.contains(4) == false { return 3 }
            if evens.contains(1) { return 4 }

            // Test map()
            let doubled = s.map({ (x) in x * 2 });
            if doubled.count != 5 { return 5 }
            if doubled.contains(2) == false { return 6 }
            if doubled.contains(10) == false { return 7 }

            // Test map() with collisions - duplicates removed
            let modThree = s.map({ (x) in x % 3 });
            // 1%3=1, 2%3=2, 3%3=0, 4%3=1, 5%3=2 -> {0, 1, 2}
            if modThree.count != 3 { return 8 }

            // Test toArray()
            let arr = s.toArray();
            if arr.count != 5 { return 9 }

            // Test sorted() - returns array (note: sort not yet implemented, returns unsorted)
            let sorted = s.sorted();
            if sorted.count != 5 { return 10 }

            // Test min() and max()
            let minVal = s.min();
            if minVal.isNone() { return 11 }
            if minVal.unwrap() != 1 { return 12 }

            let maxVal = s.max();
            if maxVal.isNone() { return 13 }
            if maxVal.unwrap() != 5 { return 14 }

            // Test filter on original set unchanged
            if s.count != 5 { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_init_empty() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test init() creates an empty set
            let s = std.collections.Set[std.num.Int64]();
            if s.count != 0 { return 1 }
            if s.isEmpty == false { return 2 }
            if s.contains(0) { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_searching_predicates() {
    Test::new(
        r#"module Test

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
            if s.all(satisfy: { (x) in x > 0 }) == false { return 6 }

            // Test all(satisfy:) - false
            if s.all(satisfy: { (x) in x > 2 }) { return 7 }

            // Test all(satisfy:) - empty set (vacuous truth)
            let empty = std.collections.Set[std.num.Int64]();
            if empty.all(satisfy: { (x) in false }) == false { return 8 }

            // Test any(satisfy:) - true
            if s.any(satisfy: { (x) in x == 3 }) == false { return 9 }

            // Test any(satisfy:) - false
            if s.any(satisfy: { (x) in x > 100 }) { return 10 }

            // Test any(satisfy:) - empty set
            if empty.any(satisfy: { (x) in true }) { return 11 }

            // Test countWhere(predicate:)
            let evenCount = s.countWhere({ (x) in x % 2 == 0 });
            if evenCount != 2 { return 12 }

            // countWhere with all matching
            let allCount = s.countWhere({ (x) in x > 0 });
            if allCount != 5 { return 13 }

            // countWhere with none matching
            let noneCount = s.countWhere({ (x) in x > 100 });
            if noneCount != 0 { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_capacity_management() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test capacity property on init(capacity:)
            var s = std.collections.Set[std.num.Int64](capacity: 100);
            if s.capacity < 100 { return 1 }
            if s.count != 0 { return 2 }

            // Test reserveCapacity
            var s2 = std.collections.Set[std.num.Int64]();
            s2.reserveCapacity(50);
            if s2.capacity < 50 { return 3 }

            // reserveCapacity doesn't shrink
            let capBefore = s2.capacity;
            s2.reserveCapacity(10);
            if s2.capacity < capBefore { return 4 }

            // Test shrinkToFit
            var s3 = std.collections.Set[std.num.Int64](capacity: 100);
            let _ = s3.insert(1);
            let _ = s3.insert(2);
            let capBeforeShrink = s3.capacity;
            s3.shrinkToFit();
            if s3.capacity > capBeforeShrink { return 5 }
            // Elements should still be there after shrink
            if s3.count != 2 { return 6 }
            if s3.contains(1) == false { return 7 }
            if s3.contains(2) == false { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_insert_contents_of() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);

            // Insert contents of an array
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(1); // duplicate

            s.insert(contentsOf: arr);

            if s.count != 4 { return 1 }
            if s.contains(1) == false { return 2 }
            if s.contains(2) == false { return 3 }
            if s.contains(3) == false { return 4 }
            if s.contains(4) == false { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_retain_and_remove_all() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test retain(matching:)
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

            s.retain(matching: { (x) in x % 2 == 0 });
            if s.count != 2 { return 1 }
            if s.contains(2) == false { return 2 }
            if s.contains(4) == false { return 3 }
            if s.contains(1) { return 4 }
            if s.contains(3) { return 5 }

            // Test removeAll(matching:)
            var s2 = std.collections.Set[std.num.Int64]();
            let _ = s2.insert(1);
            let _ = s2.insert(2);
            let _ = s2.insert(3);
            let _ = s2.insert(4);
            let _ = s2.insert(5);

            s2.removeAll(matching: { (x) in x % 2 == 0 });
            if s2.count != 3 { return 6 }
            if s2.contains(1) == false { return 7 }
            if s2.contains(3) == false { return 8 }
            if s2.contains(5) == false { return 9 }
            if s2.contains(2) { return 10 }
            if s2.contains(4) { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(10);
            let _ = s.insert(20);
            let _ = s.insert(30);

            // Iterate and collect sum to verify all elements are visited
            var sum: std.num.Int64 = 0;
            var iter = s.iter();
            while let .Some(elem) = iter.next() {
                sum = sum + elem;
            }
            if sum != 60 { return 1 }

            // Verify iter count
            var count: std.num.Int64 = 0;
            var iter2 = s.iter();
            while let .Some(_) = iter2.next() {
                count = count + 1;
            }
            if count != 3 { return 2 }

            // Empty set iteration
            let empty = std.collections.Set[std.num.Int64]();
            var iter3 = empty.iter();
            if let .Some(_) = iter3.next() {
                return 3
            }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_equals() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var a = std.collections.Set[std.num.Int64]();
            let _ = a.insert(1);
            let _ = a.insert(2);
            let _ = a.insert(3);

            // Same elements, different insertion order
            var b = std.collections.Set[std.num.Int64]();
            let _ = b.insert(3);
            let _ = b.insert(1);
            let _ = b.insert(2);

            if a.equals(b) == false { return 1 }

            // Different sizes
            var c = std.collections.Set[std.num.Int64]();
            let _ = c.insert(1);
            let _ = c.insert(2);

            if a.equals(c) { return 2 }

            // Different elements, same size
            var d = std.collections.Set[std.num.Int64]();
            let _ = d.insert(1);
            let _ = d.insert(2);
            let _ = d.insert(4);

            if a.equals(d) { return 3 }

            // Both empty
            let e1 = std.collections.Set[std.num.Int64]();
            let e2 = std.collections.Set[std.num.Int64]();
            if e1.equals(e2) == false { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - codegen type mismatch in monomorphization of Set.sum() method.
// The Addable extension on Set requires complex constraint resolution.
#[test]
fn set_sum() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);

            let total = s.sum();
            if total != 6 { return 1 }

            // Empty set sum
            let empty = std.collections.Set[std.num.Int64]();
            let emptySum = empty.sum();
            if emptySum != 0 { return 2 }

            // Single element
            var single = std.collections.Set[std.num.Int64]();
            let _ = single.insert(42);
            if single.sum() != 42 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_compact_map() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);
            let _ = s.insert(3);
            let _ = s.insert(4);
            let _ = s.insert(5);

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
                let r: std.num.Int64? = .None;
                r
            });
            if allNone.count != 0 { return 6 }

            // compactMap where all return Some
            let allSome = s.compactMap({ (x) in .Some(x) });
            if allSome.count != 5 { return 7 }

            // compactMap on empty set
            let empty = std.collections.Set[std.num.Int64]();
            let emptyResult = empty.compactMap({ (x) in .Some(x * 10) });
            if emptyResult.count != 0 { return 8 }

            // compactMap that produces duplicate values (set deduplicates)
            // All values map to the same value
            let collapsed = s.compactMap({ (x) in .Some(1) });
            if collapsed.count != 1 { return 9 }
            if collapsed.contains(1) == false { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn set_flat_map() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var s = std.collections.Set[std.num.Int64]();
            let _ = s.insert(1);
            let _ = s.insert(2);

            // flatMap: transform returns Set, results are unioned
            let result = s.flatMap({ (x) in
                var inner = std.collections.Set[std.num.Int64]();
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
            var s2 = std.collections.Set[std.num.Int64]();
            let _ = s2.insert(1);
            let _ = s2.insert(2);
            let _ = s2.insert(3);
            let overlap = s2.flatMap({ (x) in
                var inner = std.collections.Set[std.num.Int64]();
                let _ = inner.insert(x);
                let _ = inner.insert(x + 1);
                inner
            });
            // {1,2} union {2,3} union {3,4} = {1, 2, 3, 4}
            if overlap.count != 4 { return 6 }
            if overlap.contains(1) == false { return 7 }
            if overlap.contains(4) == false { return 8 }

            // flatMap on empty set
            let empty = std.collections.Set[std.num.Int64]();
            let emptyResult = empty.flatMap({ (x) in
                var inner = std.collections.Set[std.num.Int64]();
                let _ = inner.insert(x);
                inner
            });
            if emptyResult.count != 0 { return 9 }

            // flatMap where transform returns empty sets
            let emptyInner = s.flatMap({ (x) in
                std.collections.Set[std.num.Int64]()
            });
            if emptyInner.count != 0 { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
