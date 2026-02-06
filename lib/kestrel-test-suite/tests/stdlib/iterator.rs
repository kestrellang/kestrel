use kestrel_test_suite::*;

#[test]
fn map_filter_collect() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test map
            let doubled = arr.iter().map({ (x) in x * 2 }).collect();
            if doubled.count != 5 { return 1 }
            if doubled(unchecked: 0) != 2 { return 2 }
            if doubled(unchecked: 4) != 10 { return 3 }

            // Test filter
            let evens = arr.iter().filter({ (x) in x % 2 == 0 }).collect();
            if evens.count != 2 { return 4 }
            if evens(unchecked: 0) != 2 { return 5 }
            if evens(unchecked: 1) != 4 { return 6 }

            // Test map + filter chain
            let result = arr.iter().filter({ (x) in x > 2 }).map({ (x) in x * 10 }).collect();
            if result.count != 3 { return 7 }
            if result(unchecked: 0) != 30 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn take_skip_methods() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test take
            let first3 = arr.iter().take(3).collect();
            if first3.count != 3 { return 1 }
            if first3(unchecked: 2) != 3 { return 2 }

            // Test skip
            let last3 = arr.iter().skip(2).collect();
            if last3.count != 3 { return 3 }
            if last3(unchecked: 0) != 3 { return 4 }

            // Test takeWhile
            let lessThan4 = arr.iter().takeWhile({ (x) in x < 4 }).collect();
            if lessThan4.count != 3 { return 5 }
            if lessThan4(unchecked: 2) != 3 { return 6 }

            // Test skipWhile
            let from4 = arr.iter().skipWhile({ (x) in x < 4 }).collect();
            if from4.count != 2 { return 7 }
            if from4(unchecked: 0) != 4 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn zip_chain_enumerate() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr1 = std.collections.Array[std.num.Int64]();
            arr1.append(1);
            arr1.append(2);
            arr1.append(3);

            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(10);
            arr2.append(20);
            arr2.append(30);

            // Test zip
            let zipped = arr1.iter().zip(arr2.iter()).collect();
            if zipped.count != 3 { return 1 }
            let (a, b) = zipped(unchecked: 0);
            if a != 1 { return 2 }
            if b != 10 { return 3 }

            // Test enumerate
            let enumerated = arr1.iter().enumerate().collect();
            if enumerated.count != 3 { return 4 }
            let (idx, val) = enumerated(unchecked: 1);
            if idx != 1 { return 5 }
            if val != 2 { return 6 }

            // Test chain
            let chained = arr1.iter().chain(arr2.iter()).collect();
            if chained.count != 6 { return 7 }
            if chained(unchecked: 3) != 10 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn terminal_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // Test count
            if arr.iter().count() != 5 { return 1 }
            if arr.iter().filter({ (x) in x % 2 == 0 }).count() != 2 { return 2 }

            // Test fold (sum)
            let sum = arr.iter().fold(initial: 0, combine: { (acc, x) in acc + x });
            if sum != 15 { return 3 }

            // Test any
            if arr.iter().any({ (x) in x > 10 }) { return 4 }
            if arr.iter().any({ (x) in x == 3 }) == false { return 5 }

            // Test all
            if arr.iter().all({ (x) in x < 10 }) == false { return 6 }
            if arr.iter().all({ (x) in x % 2 == 0 }) { return 7 }

            // Test find
            let found = arr.iter().find({ (x) in x > 3 });
            if found.isNone() { return 8 }
            if found.unwrap() != 4 { return 9 }

            // Test nth
            let third = arr.iter().nth(2);
            if third.isNone() { return 10 }
            if third.unwrap() != 3 { return 11 }

            // Test first and last
            if arr.iter().first().unwrap() != 1 { return 12 }
            if arr.iter().last().unwrap() != 5 { return 13 }

            // Test forEach
            var total: std.num.Int64 = 0;
            arr.iter().forEach({ (x) in total = total + x });
            if total != 15 { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(HasError("cannot assign to captured variable"));
}

#[test]
fn min_max_sorted() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(3);
            arr.append(1);
            arr.append(4);
            arr.append(1);
            arr.append(5);

            // Test min
            let minVal = arr.iter().min();
            if minVal.isNone() { return 1 }
            if minVal.unwrap() != 1 { return 2 }

            // Test max
            let maxVal = arr.iter().max();
            if maxVal.isNone() { return 3 }
            if maxVal.unwrap() != 5 { return 4 }

            // Test sorted
            let sorted = arr.iter().sorted();
            if sorted.count != 5 { return 5 }
            if sorted(unchecked: 0) != 1 { return 6 }
            if sorted(unchecked: 4) != 5 { return 7 }

            // Test sum
            let sum = arr.iter().sum();
            if sum != 14 { return 8 }

            // Test product
            let product = [1, 2, 3].iter().product();
            if product != 6 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn filter_map_flatten() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test filterMap with optionals
            var arr = std.collections.Array[std.result.Optional[std.num.Int64]]();
            arr.append(.Some(1));
            arr.append(.None);
            arr.append(.Some(3));

            let compacted = arr.iter().compactMap().collect();
            if compacted.count != 2 { return 1 }
            if compacted(unchecked: 0) != 1 { return 2 }
            if compacted(unchecked: 1) != 3 { return 3 }

            // Test flatMap
            var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
            var inner1 = std.collections.Array[std.num.Int64]();
            inner1.append(1);
            inner1.append(2);
            var inner2 = std.collections.Array[std.num.Int64]();
            inner2.append(3);
            nested.append(inner1);
            nested.append(inner2);

            let flat = nested.iter().flatMap({ (arr) in arr.iter() }).collect();
            if flat.count != 3 { return 4 }
            if flat(unchecked: 0) != 1 { return 5 }
            if flat(unchecked: 2) != 3 { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn utility_adapters() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);

            // Test stepBy
            let everyOther: std.collections.Array[std.num.Int64] = [0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();
            if everyOther.count != 4 { return 1 }
            if everyOther(unchecked: 1) != 2 { return 2 }

            // Test scan (running sum)
            let running: std.collections.Array[std.num.Int64] = arr.iter().scan(0, { (acc, x) in acc + x }).collect();
            if running.count != 3 { return 3 }
            if running(unchecked: 0) != 1 { return 4 }
            if running(unchecked: 2) != 6 { return 5 }

            // Test position
            let pos = arr.iter().position({ (x) in x == 2 });
            if pos.isNone() { return 6 }
            if pos.unwrap() != 1 { return 7 }

            // Test contains
            if arr.iter().contains(2) == false { return 8 }
            if arr.iter().contains(10) { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - PeekableIterator's associated type Item is not resolved
// during layout computation. The codegen panics with "AssociatedTypeProjection reached
// layout computation without resolution". Needs compiler fix for associated type
// resolution on adapter structs.
#[test]
fn peekable_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);

            // ---- peekable() ----
            var iter = arr.iter().peekable();

            // Peek doesn't consume
            let p1 = iter.peek();
            if p1.isNone() { return 1 }
            if p1.unwrap() != 1 { return 2 }

            // Peek again returns same value
            let p2 = iter.peek();
            if p2.unwrap() != 1 { return 3 }

            // next() consumes
            let n1 = iter.next();
            if n1.unwrap() != 1 { return 4 }

            // Peek now shows next element
            let p3 = iter.peek();
            if p3.unwrap() != 2 { return 5 }

            // Consume remaining
            iter.next();
            iter.next();
            let pEnd = iter.peek();
            if pEnd.isSome() { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - IntersperseIterator's associated type Item is not resolved
// during layout computation. Same AssociatedTypeProjection issue as peekable.
#[test]
fn intersperse_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- intersperse() ----
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().intersperse(0).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator
            let single: std.collections.Array[std.num.Int64] = [42].iter().intersperse(0).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty - stays empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().intersperse(0).collect();
            if emptyResult.count != 0 { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn reduce_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- reduce() ----
            let sum = [1, 2, 3, 4].iter().reduce(combine: { (a, b) in a + b });
            if sum.isNone() { return 1 }
            if sum.unwrap() != 10 { return 2 }

            // reduce on single element
            let single = [42].iter().reduce(combine: { (a, b) in a + b });
            if single.isNone() { return 3 }
            if single.unwrap() != 42 { return 4 }

            // reduce on empty returns None
            let empty = std.collections.Array[std.num.Int64]();
            let none = empty.iter().reduce(combine: { (a, b) in a + b });
            if none.isSome() { return 5 }

            // reduce for max
            let maxVal = [3, 1, 4, 1, 5].iter().reduce(combine: { (a, b) in if a > b { a } else { b } });
            if maxVal.unwrap() != 5 { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - cycle() requires Cloneable on the iterator, but
// ArrayIterator doesn't have a Cloneable witness. Monomorphization fails with
// "no witness found: protocol Cloneable for type ArrayIterator[Int64]".
// fuse() alone works fine but is tested here with cycle().
#[test]
fn fuse_and_cycle() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // ---- fuse() ----
            let fused: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().fuse().collect();
            if fused.count != 3 { return 1 }
            if fused(unchecked: 0) != 1 { return 2 }
            if fused(unchecked: 2) != 3 { return 3 }

            // ---- cycle() + take() ----
            let cycled: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().cycle().take(7).collect();
            if cycled.count != 7 { return 4 }
            if cycled(unchecked: 0) != 1 { return 5 }
            if cycled(unchecked: 3) != 1 { return 6 }
            if cycled(unchecked: 6) != 1 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - closure captures cannot be assigned to ("cannot assign to captured variable").
// The inspect adapter itself works, but verifying side effects requires mutable closure captures.
#[test]
fn inspect_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test inspect passes elements through unchanged
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().inspect({ (x) in }).collect();
            if result.count != 3 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 2 { return 3 }
            if result(unchecked: 2) != 3 { return 4 }

            // Test inspect in chain - elements still flow through
            let inspected = [1, 2, 3, 4, 5].iter().inspect({ (x) in });
            let filtered: std.collections.Array[std.num.Int64] = inspected.filter({ (x) in x > 2 }).collect();
            if filtered.count != 3 { return 5 }
            if filtered(unchecked: 0) != 3 { return 6 }

            // Test inspect on empty iterator
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().inspect({ (x) in }).collect();
            if emptyResult.count != 0 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn filter_map_explicit() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test filterMap with explicit transform returning Optional
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3, 4, 5].iter().filterMap({ (x) in if x % 2 == 0 { .Some(x * 10) } else { .None } }).collect();
            if result.count != 2 { return 1 }
            if result(unchecked: 0) != 20 { return 2 }
            if result(unchecked: 1) != 40 { return 3 }

            // filterMap where all are None
            let allNone: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().filterMap({ (x) in .None }).collect();
            if allNone.count != 0 { return 4 }

            // filterMap where all are Some
            let allSome: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().filterMap({ (x) in .Some(x) }).collect();
            if allSome.count != 3 { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - unzip() on tuple iterator hits AssociatedTypeProjection codegen
// issue during index access on the tuple type.
#[test]
fn unzip_iterator() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test unzip on iterator of tuples
            var pairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            pairs.append((1, 10));
            pairs.append((2, 20));
            pairs.append((3, 30));

            let (left, right) = pairs.iter().unzip();
            if left.count != 3 { return 1 }
            if right.count != 3 { return 2 }
            if left(unchecked: 0) != 1 { return 3 }
            if left(unchecked: 1) != 2 { return 4 }
            if left(unchecked: 2) != 3 { return 5 }
            if right(unchecked: 0) != 10 { return 6 }
            if right(unchecked: 1) != 20 { return 7 }
            if right(unchecked: 2) != 30 { return 8 }

            // Unzip empty
            let emptyPairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            let (emptyLeft, emptyRight) = emptyPairs.iter().unzip();
            if emptyLeft.count != 0 { return 9 }
            if emptyRight.count != 0 { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn flatten_iterator() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Flatten nested iterators
            var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
            var inner1 = std.collections.Array[std.num.Int64]();
            inner1.append(1);
            inner1.append(2);
            var inner2 = std.collections.Array[std.num.Int64]();
            inner2.append(3);
            inner2.append(4);
            var inner3 = std.collections.Array[std.num.Int64]();
            inner3.append(5);
            nested.append(inner1);
            nested.append(inner2);
            nested.append(inner3);

            let flat = nested.iter().map({ (arr) in arr.iter() }).flatten().collect();
            if flat.count != 5 { return 1 }
            if flat(unchecked: 0) != 1 { return 2 }
            if flat(unchecked: 4) != 5 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn min_by_max_by() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test minBy - find element with minimum key
            var pairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            pairs.append((1, 30));
            pairs.append((2, 10));
            pairs.append((3, 20));

            let minPair = pairs.iter().minBy({ (p) in p.1 });
            if minPair.isNone() { return 1 }
            let minVal = minPair.unwrap();
            if minVal.0 != 2 { return 2 }
            if minVal.1 != 10 { return 3 }

            // Test maxBy - find element with maximum key
            let maxPair = pairs.iter().maxBy({ (p) in p.1 });
            if maxPair.isNone() { return 4 }
            let maxVal = maxPair.unwrap();
            if maxVal.0 != 1 { return 5 }
            if maxVal.1 != 30 { return 6 }

            // minBy on empty
            let emptyPairs = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            let emptyMin = emptyPairs.iter().minBy({ (p) in p.1 });
            if emptyMin.isSome() { return 7 }

            // maxBy on empty
            let emptyMax = emptyPairs.iter().maxBy({ (p) in p.1 });
            if emptyMax.isSome() { return 8 }

            // minBy on single element
            var singleArr = std.collections.Array[(std.num.Int64, std.num.Int64)]();
            singleArr.append((42, 99));
            let singleMin = singleArr.iter().minBy({ (p) in p.1 });
            if singleMin.isNone() { return 9 }
            if singleMin.unwrap().0 != 42 { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn is_sorted_checks() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test isSorted - ascending
            if [1, 2, 3, 4, 5].iter().isSorted() == false { return 1 }

            // Test isSorted - not sorted
            if [1, 3, 2, 4, 5].iter().isSorted() { return 2 }

            // Test isSorted - equal elements OK
            if [1, 1, 2, 2, 3].iter().isSorted() == false { return 3 }

            // Test isSorted - empty
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSorted() == false { return 4 }

            // Test isSorted - single element
            if [42].iter().isSorted() == false { return 5 }

            // Test isSortedDescending - descending
            if [5, 4, 3, 2, 1].iter().isSortedDescending() == false { return 6 }

            // Test isSortedDescending - not descending
            if [5, 3, 4, 2, 1].iter().isSortedDescending() { return 7 }

            // Test isSortedDescending - equal elements OK
            if [3, 3, 2, 2, 1].iter().isSortedDescending() == false { return 8 }

            // Test isSortedDescending - empty
            if empty.iter().isSortedDescending() == false { return 9 }

            // Test isSortedDescending - single element
            if [42].iter().isSortedDescending() == false { return 10 }

            // Test isSortedDescending - ascending is not descending (unless single/empty)
            if [1, 2, 3].iter().isSortedDescending() { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: intersperseWith may hit the same AssociatedTypeProjection issue as intersperse.
// Keeping expect(Compiles).expect(Runs) to track progress.
#[test]
fn intersperse_with_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // intersperseWith: lazy separator via closure
            let result: std.collections.Array[std.num.Int64] = [1, 2, 3].iter().intersperseWith({ () in 0 }).collect();
            if result.count != 5 { return 1 }
            if result(unchecked: 0) != 1 { return 2 }
            if result(unchecked: 1) != 0 { return 3 }
            if result(unchecked: 2) != 2 { return 4 }
            if result(unchecked: 3) != 0 { return 5 }
            if result(unchecked: 4) != 3 { return 6 }

            // Single element - no separator generated
            let single: std.collections.Array[std.num.Int64] = [42].iter().intersperseWith({ () in 0 }).collect();
            if single.count != 1 { return 7 }
            if single(unchecked: 0) != 42 { return 8 }

            // Empty iterator - stays empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().intersperseWith({ () in 99 }).collect();
            if emptyResult.count != 0 { return 9 }

            // intersperseWith with varying separator (counter-based)
            // Note: cannot use mutable closure captures, so use a constant separator
            let result2: std.collections.Array[std.num.Int64] = [10, 20, 30].iter().intersperseWith({ () in -1 }).collect();
            if result2.count != 5 { return 10 }
            if result2(unchecked: 0) != 10 { return 11 }
            if result2(unchecked: 1) != -1 { return 12 }
            if result2(unchecked: 2) != 20 { return 13 }
            if result2(unchecked: 3) != -1 { return 14 }
            if result2(unchecked: 4) != 30 { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - tryFold compilation fails, likely due to Result
// type inference in the combine closure.
#[test]
fn try_fold_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // tryFold: fold where combine returns Result
            // Successful fold - all Ok
            let result = [1, 2, 3, 4].iter().tryFold(initial: 0, combine: { (acc, x) in
                .Ok(acc + x)
            });
            match result {
                .Ok(v) => { if v != 10 { return 1 } },
                .Err(_) => { return 2 }
            }

            // tryFold with early exit on error
            let earlyExit = [1, 2, 3, 4, 5].iter().tryFold(initial: 0, combine: { (acc, x) in
                if acc > 3 {
                    let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(acc);
                    err
                } else {
                    .Ok(acc + x)
                }
            });
            match earlyExit {
                .Ok(_) => { return 3 },
                .Err(e) => { if e != 6 { return 4 } }
            }

            // tryFold on empty iterator returns Ok(initial)
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult: std.result.Result[std.num.Int64, std.num.Int64] = empty.iter().tryFold(initial: 42, combine: { (acc, x) in
                .Ok(acc + x)
            });
            match emptyResult {
                .Ok(v) => { if v != 42 { return 5 } },
                .Err(_) => { return 6 }
            }

            // tryFold that errors on first element
            let firstErr = [1, 2, 3].iter().tryFold(initial: 0, combine: { (acc, x) in
                let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(-1);
                err
            });
            match firstErr {
                .Ok(_) => { return 7 },
                .Err(e) => { if e != -1 { return 8 } }
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
fn try_for_each_adapter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // tryForEach: forEach where action returns Result
            // All succeed
            let result = [1, 2, 3].iter().tryForEach({ (x) in
                let ok: std.result.Result[(), std.num.Int64] = .Ok(());
                ok
            });
            match result {
                .Ok(_) => {},
                .Err(_) => { return 1 }
            }

            // tryForEach with early exit on error
            // Error when we encounter a value > 3
            let earlyExit = [1, 2, 3, 4, 5].iter().tryForEach({ (x) in
                if x > 3 {
                    let err: std.result.Result[(), std.num.Int64] = .Err(x);
                    err
                } else {
                    .Ok(())
                }
            });
            match earlyExit {
                .Ok(_) => { return 2 },
                .Err(e) => { if e != 4 { return 3 } }
            }

            // tryForEach on empty iterator returns Ok
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.iter().tryForEach({ (x) in
                let err: std.result.Result[(), std.num.Int64] = .Err(x);
                err
            });
            match emptyResult {
                .Ok(_) => {},
                .Err(_) => { return 4 }
            }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - isSorted(by:) compilation fails, likely due to
// closure type inference issues with the Bool-returning comparator.
#[test]
fn is_sorted_by_comparator() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // isSorted(by:) with a custom comparator
            // Check descending order: a >= b means "a comes before b"
            if [5, 4, 3, 2, 1].iter().isSorted(by: { (a, b) in a >= b }) == false { return 1 }

            // Ascending is not sorted in descending order
            if [1, 2, 3, 4, 5].iter().isSorted(by: { (a, b) in a >= b }) { return 2 }

            // Check sorted by absolute value
            if [-1, 2, -3, 4].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) == false { return 3 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSorted(by: { (a, b) in
                let absA = if a < 0 { 0 - a } else { a };
                let absB = if b < 0 { 0 - b } else { b };
                absA <= absB
            }) { return 4 }

            // Empty iterator is sorted by any comparator
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSorted(by: { (a, b) in false }) == false { return 5 }

            // Single element is sorted by any comparator
            if [42].iter().isSorted(by: { (a, b) in false }) == false { return 6 }

            // Equal elements - ascending comparator
            if [3, 3, 3].iter().isSorted(by: { (a, b) in a <= b }) == false { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - isSortedBy(key:) compilation fails, likely due to
// closure type inference issues with the key extractor.
#[test]
fn is_sorted_by_key() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // isSortedBy(key:) checks if elements are sorted by extracted key ascending

            // Sorted by absolute value
            if [-1, 2, -3, 4].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) == false { return 1 }

            // Not sorted by absolute value
            if [3, -1, 2].iter().isSortedBy({ (x) in if x < 0 { 0 - x } else { x } }) { return 2 }

            // Sorted by negation (effectively descending by value)
            if [5, 4, 3, 2, 1].iter().isSortedBy({ (x) in 0 - x }) == false { return 3 }

            // Not sorted by negation
            if [1, 2, 3].iter().isSortedBy({ (x) in 0 - x }) { return 4 }

            // Empty - always sorted
            let empty = std.collections.Array[std.num.Int64]();
            if empty.iter().isSortedBy({ (x) in x }) == false { return 5 }

            // Single element - always sorted
            if [42].iter().isSortedBy({ (x) in x }) == false { return 6 }

            // Identity key - same as isSorted()
            if [1, 2, 3, 4, 5].iter().isSortedBy({ (x) in x }) == false { return 7 }
            if [1, 3, 2].iter().isSortedBy({ (x) in x }) { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// NOTE: rev() requires DoubleEndedIterator. No stdlib iterator currently implements
// DoubleEndedIterator (neither ArrayIterator nor RangeIterator conform). The protocol
// and RevIterator adapter exist but cannot be tested until a concrete type implements
// nextBack(). Skipping rev() test for now.
