use kestrel_test_suite::*;

#[test]
fn operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();

            // Test isEmpty initially
            if dict.isEmpty == false { return 1 }

            // Test insert and count
            let _ = dict.insert(1, 100);
            let _ = dict.insert(2, 200);
            if dict.count != 2 { return 2 }

            // Test contains
            if dict.contains(1) == false { return 3 }
            if dict.contains(999) { return 4 }

            // Test subscript access
            let val = dict(2);
            if val.isNone() { return 5 }
            if val.unwrap() != 200 { return 6 }

            // Test remove
            let removed = dict.remove(1);
            if removed.isNone() { return 7 }
            if removed.unwrap() != 100 { return 8 }
            if dict.count != 1 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_constructors() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test init(capacity:)
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 16);
            if dict.count != 0 { return 1 }
            if dict.isEmpty == false { return 2 }
            if dict.capacity < 16 { return 3 }

            // Test that capacity dict works normally after inserts
            let _ = dict.insert(1, 100);
            let _ = dict.insert(2, 200);
            if dict.count != 2 { return 4 }
            if dict(1).unwrap() != 100 { return 5 }
            if dict(2).unwrap() != 200 { return 6 }

            // Test init(capacity: 0) creates empty dictionary
            var dict2 = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 0);
            if dict2.isEmpty == false { return 7 }
            let _ = dict2.insert(5, 50);
            if dict2(5).unwrap() != 50 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: init(from:), init(uniqueKeysWithValues:), and init(grouping:by:) hit a codegen
// limitation with AssociatedTypeProjection for Iterable.Item on tuple iterables.
// These constructors compile but fail at codegen with:
//   "unsupported: index access on unsupported type: AssociatedTypeProjection"
// Tests should be added once this codegen limitation is resolved.

#[test]
fn dictionary_subscripts() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);

            // Test subscript(key:default:) - key exists
            let val1 = dict(1, default: 999);
            if val1 != 10 { return 1 }

            // Test subscript(key:default:) - key missing, returns default
            let val2 = dict(99, default: 999);
            if val2 != 999 { return 2 }

            // Test that default is NOT inserted
            if dict.contains(99) { return 3 }

            // Test subscript(key:inserting:) - key exists
            let val3 = dict(1, inserting: 555);
            if val3 != 10 { return 4 }

            // Test subscript(key:inserting:) - key missing, inserts default
            let val4 = dict(50, inserting: 500);
            if val4 != 500 { return 5 }
            // Key should now exist
            if dict.contains(50) == false { return 6 }
            if dict(50).unwrap() != 500 { return 7 }

            // Test subscript(unwrap:) - key exists
            let val5 = dict(unwrap: 2);
            if val5 != 20 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_mutation() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test clear()
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);
            dict.clear();
            if dict.count != 0 { return 1 }
            if dict.isEmpty == false { return 2 }

            // Test update(key:with:) - key exists
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let updated = dict.update(1, with: { (v) in v * 10 });
            if updated == false { return 3 }
            if dict(1).unwrap() != 100 { return 4 }

            // Test update(key:with:) - key missing
            let notUpdated = dict.update(99, with: { (v) in v * 10 });
            if notUpdated { return 5 }

            // Test upsert(key:default:with:) - key exists
            dict.upsert(2, default: 0, with: { (v) in v + 5 });
            if dict(2).unwrap() != 25 { return 6 }

            // Test upsert(key:default:with:) - key missing
            dict.upsert(99, default: 0, with: { (v) in v + 5 });
            if dict(99).unwrap() != 5 { return 7 }

            // Test merge()
            var base = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = base.insert(1, 10);
            let _ = base.insert(2, 20);
            var other = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = other.insert(2, 200);
            let _ = other.insert(3, 300);
            base.merge(other, uniquingKeysWith: { (old, new) in old + new });
            if base.count != 3 { return 8 }
            if base(1).unwrap() != 10 { return 9 }
            if base(2).unwrap() != 220 { return 10 }
            if base(3).unwrap() != 300 { return 11 }

            // Test retain(matching:)
            var dict2 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict2.insert(1, 10);
            let _ = dict2.insert(2, 20);
            let _ = dict2.insert(3, 30);
            let _ = dict2.insert(4, 40);
            dict2.retain(matching: { (k, v) in v > 15 });
            if dict2.count != 3 { return 12 }
            if dict2.contains(1) { return 13 }
            if dict2.contains(2) == false { return 14 }

            // Test removeAll(matching:)
            var dict3 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict3.insert(1, 10);
            let _ = dict3.insert(2, 20);
            let _ = dict3.insert(3, 30);
            dict3.removeAll(matching: { (k, v) in v >= 20 });
            if dict3.count != 1 { return 15 }
            if dict3(1).unwrap() != 10 { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_querying() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // Test keys - iterate and count
            var keyCount: std.num.Int64 = 0;
            var keyIter = dict.keys.iter();
            while let .Some(_) = keyIter.next() {
                keyCount = keyCount + 1;
            }
            if keyCount != 3 { return 1 }

            // Test values - iterate and sum
            var valSum: std.num.Int64 = 0;
            var valIter = dict.values.iter();
            while let .Some(v) = valIter.next() {
                valSum = valSum + v;
            }
            if valSum != 60 { return 2 }

            // Test contains(matching:) - true case
            let hasLargeValue = dict.contains(matching: { (k, v) in v > 25 });
            if hasLargeValue == false { return 3 }

            // Test contains(matching:) - false case
            let hasHugeValue = dict.contains(matching: { (k, v) in v > 100 });
            if hasHugeValue { return 4 }

            // Test all(satisfy:) - true case
            let allPositive = dict.all(satisfy: { (k, v) in v > 0 });
            if allPositive == false { return 5 }

            // Test all(satisfy:) - false case
            let allBig = dict.all(satisfy: { (k, v) in v > 15 });
            if allBig { return 6 }

            // Test any(satisfy:)
            let anyTwenty = dict.any(satisfy: { (k, v) in v == 20 });
            if anyTwenty == false { return 7 }

            let anyHundred = dict.any(satisfy: { (k, v) in v == 100 });
            if anyHundred { return 8 }

            // Test countWhere()
            let countAbove15 = dict.countWhere({ (k, v) in v > 15 });
            if countAbove15 != 2 { return 9 }

            // Test first(matching:)
            let found = dict.first(matching: { (k, v) in v == 20 });
            if found.isNone() { return 10 }
            let (fk, fv) = found.unwrap();
            if fk != 2 { return 11 }
            if fv != 20 { return 12 }

            // Test first(matching:) - not found
            let notFound = dict.first(matching: { (k, v) in v == 999 });
            if notFound.isSome() { return 13 }

            // Test all(satisfy:) on empty dictionary - vacuous truth
            var emptyDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let vacuousAll = emptyDict.all(satisfy: { (k, v) in false });
            if vacuousAll == false { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_transforms() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // Test mapValues()
            let doubled = dict.mapValues({ (v) in v * 2 });
            if doubled.count != 3 { return 1 }
            if doubled(1).unwrap() != 20 { return 2 }
            if doubled(2).unwrap() != 40 { return 3 }
            if doubled(3).unwrap() != 60 { return 4 }

            // Test compactMapValues()
            // Map values: keep only values > 15 by returning Some/None
            let compacted = dict.compactMapValues({ (v) in
                if v > 15 { .Some(v * 10) } else { .None }
            });
            if compacted.count != 2 { return 5 }
            if compacted.contains(1) { return 6 }
            if compacted(2).unwrap() != 200 { return 7 }
            if compacted(3).unwrap() != 300 { return 8 }

            // Test filter(matching:)
            let filtered = dict.filter(matching: { (k, v) in v >= 20 });
            if filtered.count != 2 { return 9 }
            if filtered.contains(1) { return 10 }
            if filtered(2).unwrap() != 20 { return 11 }
            if filtered(3).unwrap() != 30 { return 12 }

            // Test merging() - non-mutating
            var other = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = other.insert(3, 300);
            let _ = other.insert(4, 400);
            let merged = dict.merging(other, uniquingKeysWith: { (old, new) in new });
            if merged.count != 4 { return 13 }
            if merged(1).unwrap() != 10 { return 14 }
            if merged(3).unwrap() != 300 { return 15 }
            if merged(4).unwrap() != 400 { return 16 }

            // Original dict unchanged
            if dict.count != 3 { return 17 }
            if dict(3).unwrap() != 30 { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // iter() - iterate over all key-value pairs
            var count: std.num.Int64 = 0;
            var keySum: std.num.Int64 = 0;
            var valSum: std.num.Int64 = 0;
            var iter = dict.iter();
            while let .Some(pair) = iter.next() {
                count = count + 1;
                keySum = keySum + pair.0;
                valSum = valSum + pair.1;
            }
            if count != 3 { return 1 }
            if keySum != 6 { return 2 }
            if valSum != 60 { return 3 }

            // iter() on empty dictionary
            let emptyDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            var emptyIter = emptyDict.iter();
            let first = emptyIter.next();
            if first.isSome() { return 4 }

            // iter() - verify all expected keys are present
            var found1 = false;
            var found2 = false;
            var found3 = false;
            var iter2 = dict.iter();
            while let .Some(pair) = iter2.next() {
                if pair.0 == 1 and pair.1 == 10 { found1 = true }
                if pair.0 == 2 and pair.1 == 20 { found2 = true }
                if pair.0 == 3 and pair.1 == 30 { found3 = true }
            }
            if found1 == false { return 5 }
            if found2 == false { return 6 }
            if found3 == false { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_equatable_extensions() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // equals(other:) - same dictionaries
            var a = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = a.insert(1, 10);
            let _ = a.insert(2, 20);
            let _ = a.insert(3, 30);

            var b = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = b.insert(1, 10);
            let _ = b.insert(2, 20);
            let _ = b.insert(3, 30);

            if a.equals(b) == false { return 1 }

            // equals - different values
            var c = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = c.insert(1, 10);
            let _ = c.insert(2, 99);
            let _ = c.insert(3, 30);
            if a.equals(c) { return 2 }

            // equals - different count
            var d = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = d.insert(1, 10);
            let _ = d.insert(2, 20);
            if a.equals(d) { return 3 }

            // equals - empty dictionaries
            let e1 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let e2 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            if e1.equals(e2) == false { return 4 }

            // containsValue(value:) - value exists
            if a.containsValue(20) == false { return 5 }

            // containsValue(value:) - value not found
            if a.containsValue(999) { return 6 }

            // firstKey(forValue:) - value exists
            let fk = a.firstKey(forValue: 20);
            if fk.isNone() { return 7 }
            if fk.unwrap() != 2 { return 8 }

            // firstKey(forValue:) - value not found
            let fkNone = a.firstKey(forValue: 999);
            if fkNone.isSome() { return 9 }

            // allKeys(forValue:) - single match
            let keys10 = a.allKeys(forValue: 10);
            if keys10.count != 1 { return 10 }
            if keys10(0) != 1 { return 11 }

            // allKeys(forValue:) - multiple matches
            var multi = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = multi.insert(1, 100);
            let _ = multi.insert(2, 200);
            let _ = multi.insert(3, 100);
            let _ = multi.insert(4, 300);
            let _ = multi.insert(5, 100);
            let keys100 = multi.allKeys(forValue: 100);
            if keys100.count != 3 { return 12 }
            // All keys with value 100 should be present (1, 3, 5)
            if keys100.contains(1) == false { return 13 }
            if keys100.contains(3) == false { return 14 }
            if keys100.contains(5) == false { return 15 }

            // allKeys(forValue:) - no match
            let keysNone = a.allKeys(forValue: 999);
            if keysNone.count != 0 { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_capacity_management() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // capacity property
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 32);
            if dict.capacity < 32 { return 1 }
            if dict.count != 0 { return 2 }

            // reserveCapacity
            var dict2 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            dict2.reserveCapacity(64);
            if dict2.capacity < 64 { return 3 }
            if dict2.count != 0 { return 4 }

            // Insert elements and verify they work after reserveCapacity
            let _ = dict2.insert(1, 10);
            let _ = dict2.insert(2, 20);
            let _ = dict2.insert(3, 30);
            if dict2.count != 3 { return 5 }
            if dict2(1).unwrap() != 10 { return 6 }
            if dict2(2).unwrap() != 20 { return 7 }
            if dict2(3).unwrap() != 30 { return 8 }

            // shrinkToFit - reduces capacity
            var dict3 = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 256);
            let _ = dict3.insert(1, 10);
            let _ = dict3.insert(2, 20);
            let capBefore = dict3.capacity;
            dict3.shrinkToFit();
            let capAfter = dict3.capacity;
            if capAfter >= capBefore { return 9 }
            // Data should be preserved
            if dict3.count != 2 { return 10 }
            if dict3(1).unwrap() != 10 { return 11 }
            if dict3(2).unwrap() != 20 { return 12 }

            // shrinkToFit on empty dictionary
            var dict4 = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 64);
            dict4.shrinkToFit();
            if dict4.count != 0 { return 13 }

            // reserveCapacity when already sufficient - no-op
            var dict5 = std.collections.Dictionary[std.num.Int64, std.num.Int64](capacity: 128);
            let capPrev = dict5.capacity;
            dict5.reserveCapacity(16);
            if dict5.capacity != capPrev { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: mergeFrom(pairs:uniquingKeysWith:) hits codegen limitation with
// AssociatedTypeProjection for Iterable.Item on tuple iterables. Same issue as
// init(from:) and other generic-iterable-of-tuples methods.
#[test]
fn dictionary_merge_from_pairs() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // mergeFrom with another dictionary's iter
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);

            var other = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = other.insert(2, 200);
            let _ = other.insert(3, 300);

            dict.mergeFrom(other, uniquingKeysWith: { (old, new) in old + new });
            if dict.count != 3 { return 1 }
            if dict(1).unwrap() != 10 { return 2 }
            if dict(2).unwrap() != 220 { return 3 }
            if dict(3).unwrap() != 300 { return 4 }

            // mergeFrom with "take new" strategy
            var dict2 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict2.insert(1, 10);

            var src = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = src.insert(1, 99);
            let _ = src.insert(2, 200);

            dict2.mergeFrom(src, uniquingKeysWith: { (old, new) in new });
            if dict2.count != 2 { return 5 }
            if dict2(1).unwrap() != 99 { return 6 }
            if dict2(2).unwrap() != 200 { return 7 }

            // mergeFrom with empty source - no change
            var dict3 = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict3.insert(1, 10);
            let emptySrc = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            dict3.mergeFrom(emptySrc, uniquingKeysWith: { (old, new) in new });
            if dict3.count != 1 { return 8 }
            if dict3(1).unwrap() != 10 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn dictionary_deep_clone() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 100);
            let _ = dict.insert(2, 200);
            let _ = dict.insert(3, 300);

            // deepClone creates a fully independent copy
            let cloned = dict.deepClone();
            if cloned.count != 3 { return 1 }
            if cloned(1).unwrap() != 100 { return 2 }
            if cloned(2).unwrap() != 200 { return 3 }
            if cloned(3).unwrap() != 300 { return 4 }

            // Mutating original should not affect clone
            let _ = dict.insert(4, 400);
            if dict.count != 4 { return 5 }
            if cloned.count != 3 { return 6 }

            // deepClone on empty dictionary
            let emptyDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let emptyClone = emptyDict.deepClone();
            if emptyClone.count != 0 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - sumValues() requires Addable + Defaultable on V.
// Int64 may not satisfy Addable constraint resolution during codegen.
#[test]
fn dictionary_sum_values() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var dict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = dict.insert(1, 10);
            let _ = dict.insert(2, 20);
            let _ = dict.insert(3, 30);

            // sumValues() returns sum of all values
            let total = dict.sumValues();
            if total != 60 { return 1 }

            // sumValues on empty dictionary returns default (0 for Int64)
            let emptyDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let emptySum = emptyDict.sumValues();
            if emptySum != 0 { return 2 }

            // sumValues on single entry
            var singleDict = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
            let _ = singleDict.insert(1, 42);
            if singleDict.sumValues() != 42 { return 3 }

            // sumValues after mutation
            let _ = dict.insert(4, 40);
            if dict.sumValues() != 100 { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
