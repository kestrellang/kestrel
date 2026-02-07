use kestrel_test_suite::*;

#[test]
fn basic_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();

            // Test isEmpty on empty array
            if arr.isEmpty == false { return 1 }

            // Test append and count
            arr.append(10);
            arr.append(20);
            arr.append(30);
            if arr.count != 3 { return 2 }
            if arr.isEmpty { return 3 }

            // Test first and last
            if arr.first().unwrap() != 10 { return 4 }
            if arr.last().unwrap() != 30 { return 5 }

            // Test pop
            let popped = arr.pop();
            if popped.unwrap() != 30 { return 6 }
            if arr.count != 2 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn access_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test checked subscript (safe access)
            let val = arr(checked: 1);
            if val.isNone() { return 1 }
            if val.unwrap() != 20 { return 2 }

            // Test out of bounds returns None
            let oob = arr(checked: 100);
            if oob.isSome() { return 3 }

            // Test getUnchecked
            if arr(unchecked: 0) != 10 { return 4 }
            if arr(unchecked: 2) != 30 { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn iteration() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test iteration
            var sum: std.num.Int64 = 0;
            var iter = arr.iter();
            var done: std.core.Bool = false;
            while done == false {
                let next = iter.next();
                if next.isSome() {
                    sum = sum + next.unwrap()
                } else {
                    done = true
                }
            }
            if sum != 60 { return 1 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn constructors() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test init(capacity:)
            var arr = std.collections.Array[std.num.Int64](capacity: 10);
            if arr.count != 0 { return 1 }
            if arr.capacity < 10 { return 2 }
            arr.append(42);
            if arr.count != 1 { return 3 }
            if arr(unchecked: 0) != 42 { return 4 }

            // NOTE: init(repeating:count:) requires T: Cloneable, but Int64 does not
            // implement Cloneable, causing monomorphization failure. See init_repeating test.

            // Test init(from:) with a range
            let fromRange = std.collections.Array[std.num.Int64](from: std.core.Range[std.num.Int64](0, 5));
            if fromRange.count != 5 { return 5 }
            if fromRange(unchecked: 0) != 0 { return 6 }
            if fromRange(unchecked: 4) != 4 { return 7 }

            // NOTE: init(count:generator:) cannot be tested because its positional
            // signature Array(Int64, (Int64) -> T) collides with the internal
            // array literal init Array(lang.ptr[T], lang.i64). See init_count_generator test.

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn extended_access() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);

            // Test subscript(index:) get
            if arr(0) != 10 { return 1 }
            if arr(1) != 20 { return 2 }
            if arr(2) != 30 { return 3 }

            // Test subscript set via setUnchecked (arr(1) = 25 syntax not yet supported)
            // TODO: subscript assignment syntax arr(index) = value gives "cannot assign to temporary value"
            arr.setUnchecked(1, 25);
            if arr(1) != 25 { return 4 }

            // Test subscript(wrapping:) with negative index
            let wrapLast = arr(wrapping: -1);
            if wrapLast.isNone() { return 5 }
            if wrapLast.unwrap() != 30 { return 6 }

            // Test subscript(wrapping:) with -2
            let wrapSecond = arr(wrapping: -2);
            if wrapSecond.isNone() { return 7 }
            if wrapSecond.unwrap() != 25 { return 8 }

            // Test subscript(wrapping:) with overflow
            let wrapOver = arr(wrapping: 3);
            if wrapOver.isNone() { return 9 }
            if wrapOver.unwrap() != 10 { return 10 }

            // Test subscript(wrapping:) on empty array
            let emptyArr = std.collections.Array[std.num.Int64]();
            let wrapEmpty = emptyArr(wrapping: 0);
            if wrapEmpty.isSome() { return 11 }

            // Test subscript(clamping:) with negative index
            let clampNeg = arr(clamping: -5);
            if clampNeg.isNone() { return 12 }
            if clampNeg.unwrap() != 10 { return 13 }

            // Test subscript(clamping:) with over index
            let clampOver = arr(clamping: 100);
            if clampOver.isNone() { return 14 }
            if clampOver.unwrap() != 30 { return 15 }

            // Test subscript(clamping:) with normal index
            let clampNormal = arr(clamping: 1);
            if clampNormal.isNone() { return 16 }
            if clampNormal.unwrap() != 25 { return 17 }

            // Test subscript(clamping:) on empty array
            let clampEmpty = emptyArr(clamping: 0);
            if clampEmpty.isSome() { return 18 }

            // Test isValidIndex
            if arr.isValidIndex(0) == false { return 19 }
            if arr.isValidIndex(2) == false { return 20 }
            if arr.isValidIndex(3) { return 21 }
            if arr.isValidIndex(-1) { return 22 }

            // Test setUnchecked
            arr.setUnchecked(0, 99);
            if arr(0) != 99 { return 23 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn mutation_operations() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test append(contentsOf:)
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            var other = std.collections.Array[std.num.Int64]();
            other.append(3);
            other.append(4);
            arr.append(contentsOf: other);
            if arr.count != 4 { return 1 }
            if arr(unchecked: 2) != 3 { return 2 }
            if arr(unchecked: 3) != 4 { return 3 }

            // Test append(contentsOf:) with empty array
            let empty = std.collections.Array[std.num.Int64]();
            arr.append(contentsOf: empty);
            if arr.count != 4 { return 4 }

            // Test insert(element:at:) at beginning
            arr.insert(0, at: 0);
            if arr.count != 5 { return 5 }
            if arr(unchecked: 0) != 0 { return 6 }
            if arr(unchecked: 1) != 1 { return 7 }

            // Test insert(element:at:) in middle
            arr.insert(99, at: 3);
            if arr.count != 6 { return 8 }
            if arr(unchecked: 3) != 99 { return 9 }
            if arr(unchecked: 4) != 3 { return 10 }

            // Test insert(element:at:) at end (append)
            arr.insert(100, at: 6);
            if arr.count != 7 { return 11 }
            if arr(unchecked: 6) != 100 { return 12 }

            // Test popFirst()
            let first = arr.popFirst();
            if first.isNone() { return 13 }
            if first.unwrap() != 0 { return 14 }
            if arr.count != 6 { return 15 }
            if arr(unchecked: 0) != 1 { return 16 }

            // Test popFirst() on empty
            var emptyArr = std.collections.Array[std.num.Int64]();
            let emptyFirst = emptyArr.popFirst();
            if emptyFirst.isSome() { return 17 }

            // Test remove(at:)
            // arr is now [1, 2, 99, 3, 4, 100]
            let removed = arr.remove(at: 2);
            if removed != 99 { return 18 }
            if arr.count != 5 { return 19 }
            if arr(unchecked: 2) != 3 { return 20 }

            // Test removeSubrange
            // arr is now [1, 2, 3, 4, 100]
            arr.removeSubrange(std.core.Range[std.num.Int64](1, 3));
            if arr.count != 3 { return 21 }
            if arr(unchecked: 0) != 1 { return 22 }
            if arr(unchecked: 1) != 4 { return 23 }
            if arr(unchecked: 2) != 100 { return 24 }

            // Test clear()
            arr.clear();
            if arr.count != 0 { return 25 }
            if arr.isEmpty == false { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn reordering() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test swap
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr.append(30);
            arr.swap(at: 0, with: 2);
            if arr(unchecked: 0) != 30 { return 1 }
            if arr(unchecked: 1) != 20 { return 2 }
            if arr(unchecked: 2) != 10 { return 3 }

            // Test swap same index (no-op)
            arr.swap(at: 1, with: 1);
            if arr(unchecked: 1) != 20 { return 4 }

            // Test reverse
            arr.reverse();
            if arr(unchecked: 0) != 10 { return 5 }
            if arr(unchecked: 1) != 20 { return 6 }
            if arr(unchecked: 2) != 30 { return 7 }

            // Test reversed (returns new array, original unchanged)
            let rev = arr.reversed();
            if rev(unchecked: 0) != 30 { return 8 }
            if rev(unchecked: 1) != 20 { return 9 }
            if rev(unchecked: 2) != 10 { return 10 }
            // Original should be unchanged
            if arr(unchecked: 0) != 10 { return 11 }

            // Test rotate left by 2
            var rotArr = std.collections.Array[std.num.Int64]();
            rotArr.append(1);
            rotArr.append(2);
            rotArr.append(3);
            rotArr.append(4);
            rotArr.append(5);
            rotArr.rotate(by: 2);
            // [1, 2, 3, 4, 5] rotated left by 2 = [3, 4, 5, 1, 2]
            if rotArr(unchecked: 0) != 3 { return 12 }
            if rotArr(unchecked: 1) != 4 { return 13 }
            if rotArr(unchecked: 2) != 5 { return 14 }
            if rotArr(unchecked: 3) != 1 { return 15 }
            if rotArr(unchecked: 4) != 2 { return 16 }

            // Test replaceSubrange
            var repArr = std.collections.Array[std.num.Int64]();
            repArr.append(1);
            repArr.append(2);
            repArr.append(3);
            repArr.append(4);
            repArr.append(5);
            var replacement = std.collections.Array[std.num.Int64]();
            replacement.append(20);
            replacement.append(30);
            // Replace range 1..<4 ([2,3,4]) with [20,30]
            repArr.replaceSubrange(std.core.Range[std.num.Int64](1, 4), with: replacement);
            // Result should be [1, 20, 30, 5]
            if repArr.count != 4 { return 17 }
            if repArr(unchecked: 0) != 1 { return 18 }
            if repArr(unchecked: 1) != 20 { return 19 }
            if repArr(unchecked: 2) != 30 { return 20 }
            if repArr(unchecked: 3) != 5 { return 21 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn capacity_management() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test reserveCapacity
            var arr = std.collections.Array[std.num.Int64]();
            arr.reserveCapacity(100);
            if arr.capacity < 100 { return 1 }
            if arr.count != 0 { return 2 }

            // Adding elements should not reallocate while under capacity
            arr.append(1);
            arr.append(2);
            arr.append(3);
            if arr.count != 3 { return 3 }
            if arr.capacity < 100 { return 4 }

            // Test shrinkToFit
            arr.shrinkToFit();
            if arr.count != 3 { return 5 }
            if arr.capacity != 3 { return 6 }
            if arr(unchecked: 0) != 1 { return 7 }
            if arr(unchecked: 1) != 2 { return 8 }
            if arr(unchecked: 2) != 3 { return 9 }

            // Test shrinkToFit on empty array with capacity
            var emptyWithCap = std.collections.Array[std.num.Int64](capacity: 50);
            if emptyWithCap.capacity < 50 { return 10 }
            emptyWithCap.shrinkToFit();
            if emptyWithCap.count != 0 { return 11 }

            // Test capacity property via init(capacity:)
            let preallocated = std.collections.Array[std.num.Int64](capacity: 16);
            if preallocated.capacity < 16 { return 12 }
            if preallocated.count != 0 { return 13 }

            // Test that capacity grows after appending beyond initial
            var growing = std.collections.Array[std.num.Int64](capacity: 2);
            growing.append(1);
            growing.append(2);
            growing.append(3);
            if growing.count != 3 { return 14 }
            if growing.capacity < 3 { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: init(count:generator:) signature Array(Int64, (Int64) -> T) collides with the internal
// array literal init Array(lang.ptr[T], lang.i64) during overload resolution. The compiler
// matches the wrong init. This test should use .expect(Compiles).expect(Runs) once the
// overload resolution is fixed.
#[test]
fn init_count_generator() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let generated = std.collections.Array[std.num.Int64](4, { (i) in i * i });
            if generated.count != 4 { return 1 }
            0
        }
    "#,
    )
    .with_stdlib()
    .expect(HasError("does not conform to protocol"));
}

// TODO: subscript assignment syntax arr(index) = value is not yet supported.
// The compiler reports "cannot assign to temporary value". This test should use
// .expect(Compiles).expect(Runs) once subscript setter assignment is implemented.
#[test]
fn subscript_assignment() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10);
            arr.append(20);
            arr(0) = 99;
            if arr(0) != 99 { return 1 }
            0
        }
    "#,
    )
    .with_stdlib()
    .expect(HasError("cannot assign to temporary value"));
}

#[test]
fn predicate_searching() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1);
            arr.append(2);
            arr.append(3);
            arr.append(4);
            arr.append(5);

            // firstIndex(matching:)
            let fi = arr.firstIndex(matching: { (x) in x > 3 });
            if fi.isNone() { return 1 }
            if fi.unwrap() != 3 { return 2 }

            // firstIndex(matching:) - no match
            let fiNone = arr.firstIndex(matching: { (x) in x > 10 });
            if fiNone.isSome() { return 3 }

            // lastIndex(matching:)
            let li = arr.lastIndex(matching: { (x) in x < 4 });
            if li.isNone() { return 4 }
            if li.unwrap() != 2 { return 5 }

            // lastIndex(matching:) - no match
            let liNone = arr.lastIndex(matching: { (x) in x > 10 });
            if liNone.isSome() { return 6 }

            // first(matching:)
            let fm = arr.first(matching: { (x) in x > 3 });
            if fm.isNone() { return 7 }
            if fm.unwrap() != 4 { return 8 }

            // first(matching:) - no match
            let fmNone = arr.first(matching: { (x) in x > 10 });
            if fmNone.isSome() { return 9 }

            // last(matching:)
            let lm = arr.last(matching: { (x) in x < 4 });
            if lm.isNone() { return 10 }
            if lm.unwrap() != 3 { return 11 }

            // last(matching:) - no match
            let lmNone = arr.last(matching: { (x) in x > 10 });
            if lmNone.isSome() { return 12 }

            // all(satisfy:)
            let allPos = arr.all(satisfy: { (x) in x > 0 });
            if allPos == false { return 13 }

            let allBig = arr.all(satisfy: { (x) in x > 3 });
            if allBig { return 14 }

            // all(satisfy:) on empty array - vacuous truth
            let empty = std.collections.Array[std.num.Int64]();
            let allEmpty = empty.all(satisfy: { (x) in false });
            if allEmpty == false { return 15 }

            // any(satisfy:)
            let anyBig = arr.any(satisfy: { (x) in x > 4 });
            if anyBig == false { return 16 }

            let anyHuge = arr.any(satisfy: { (x) in x > 10 });
            if anyHuge { return 17 }

            // any(satisfy:) on empty array
            let anyEmpty = empty.any(satisfy: { (x) in true });
            if anyEmpty { return 18 }

            // countWhere(predicate:) - positional single-name param
            let cw = arr.countWhere({ (x) in x % 2 == 0 });
            if cw != 2 { return 19 }

            let cwNone = arr.countWhere({ (x) in x > 10 });
            if cwNone != 0 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn retain_and_removeall() {
    Test::new(
        r#"module Test

        func makeArray5() -> std.collections.Array[std.num.Int64] {
            var a = std.collections.Array[std.num.Int64]();
            a.append(1); a.append(2); a.append(3); a.append(4); a.append(5);
            a
        }

        func main() -> lang.i64 {
            // Test retain(matching:)
            var arr = makeArray5();
            arr.retain(matching: { (x) in x % 2 == 0 });
            if arr.count != 2 { return 1 }
            if arr(0) != 2 { return 2 }
            if arr(1) != 4 { return 3 }

            // Test removeAll(matching:)
            var arr2 = makeArray5();
            arr2.removeAll(matching: { (x) in x % 2 == 0 });
            if arr2.count != 3 { return 4 }
            if arr2(0) != 1 { return 5 }
            if arr2(1) != 3 { return 6 }
            if arr2(2) != 5 { return 7 }

            // retain all - keeps everything
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(10); arr3.append(20); arr3.append(30);
            arr3.retain(matching: { (x) in true });
            if arr3.count != 3 { return 8 }

            // retain none - empties array
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(10); arr4.append(20); arr4.append(30);
            arr4.retain(matching: { (x) in false });
            if arr4.count != 0 { return 9 }

            // removeAll on empty array
            var arr5 = std.collections.Array[std.num.Int64]();
            arr5.removeAll(matching: { (x) in true });
            if arr5.count != 0 { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn slicing() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30); arr.append(40); arr.append(50);

            // prefix(count:) - positional single-name param
            let pre = arr.prefix(3);
            if pre.count != 3 { return 1 }
            if pre(unchecked: 0) != 10 { return 2 }
            if pre(unchecked: 1) != 20 { return 3 }
            if pre(unchecked: 2) != 30 { return 4 }

            // prefix(count: 0) - empty
            let preEmpty = arr.prefix(0);
            if preEmpty.count != 0 { return 5 }

            // suffix(count:) - positional single-name param
            let suf = arr.suffix(2);
            if suf.count != 2 { return 6 }
            if suf(unchecked: 0) != 40 { return 7 }
            if suf(unchecked: 1) != 50 { return 8 }

            // drop(first:)
            let df = arr.drop(first: 2);
            if df.count != 3 { return 9 }
            if df(unchecked: 0) != 30 { return 10 }
            if df(unchecked: 1) != 40 { return 11 }
            if df(unchecked: 2) != 50 { return 12 }

            // drop(last:)
            let dl = arr.drop(last: 2);
            if dl.count != 3 { return 13 }
            if dl(unchecked: 0) != 10 { return 14 }
            if dl(unchecked: 1) != 20 { return 15 }
            if dl(unchecked: 2) != 30 { return 16 }

            // drop(first: 0) - keeps everything
            let dfAll = arr.drop(first: 0);
            if dfAll.count != 5 { return 17 }

            // drop(last: 0) - keeps everything
            let dlAll = arr.drop(last: 0);
            if dlAll.count != 5 { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn chunking() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // chunks(of:)
            var chunkIter = arr.chunks(of: 2);

            // First chunk: [1, 2]
            let c1 = chunkIter.next();
            if c1.isNone() { return 1 }
            let chunk1 = c1.unwrap();
            if chunk1.count != 2 { return 2 }
            if chunk1(unchecked: 0) != 1 { return 3 }
            if chunk1(unchecked: 1) != 2 { return 4 }

            // Second chunk: [3, 4]
            let c2 = chunkIter.next();
            if c2.isNone() { return 5 }
            let chunk2 = c2.unwrap();
            if chunk2.count != 2 { return 6 }
            if chunk2(unchecked: 0) != 3 { return 7 }
            if chunk2(unchecked: 1) != 4 { return 8 }

            // Third chunk: [5] (smaller last chunk)
            let c3 = chunkIter.next();
            if c3.isNone() { return 9 }
            let chunk3 = c3.unwrap();
            if chunk3.count != 1 { return 10 }
            if chunk3(unchecked: 0) != 5 { return 11 }

            // No more chunks
            let c4 = chunkIter.next();
            if c4.isSome() { return 12 }

            // windows(of:)
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(4);
            var winIter = arr2.windows(of: 2);

            // Window 1: [1, 2]
            let w1 = winIter.next();
            if w1.isNone() { return 13 }
            let win1 = w1.unwrap();
            if win1.count != 2 { return 14 }
            if win1(unchecked: 0) != 1 { return 15 }
            if win1(unchecked: 1) != 2 { return 16 }

            // Window 2: [2, 3]
            let w2 = winIter.next();
            if w2.isNone() { return 17 }
            let win2 = w2.unwrap();
            if win2(unchecked: 0) != 2 { return 18 }
            if win2(unchecked: 1) != 3 { return 19 }

            // Window 3: [3, 4]
            let w3 = winIter.next();
            if w3.isNone() { return 20 }
            let win3 = w3.unwrap();
            if win3(unchecked: 0) != 3 { return 21 }
            if win3(unchecked: 1) != 4 { return 22 }

            // No more windows
            let w4 = winIter.next();
            if w4.isSome() { return 23 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn partitioning() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test partition(by:) - in-place, returns pivot index
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);
            let pivot = arr.partition(by: { (x) in x % 2 == 0 });
            // After partition: even elements come first, pivot is the count of even elements
            if pivot != 2 { return 1 }
            // The first `pivot` elements should all be even
            var i: std.num.Int64 = 0;
            while i < pivot {
                if arr(i) % 2 != 0 { return 2 }
                i = i + 1
            }
            // The remaining elements should all be odd
            while i < arr.count {
                if arr(i) % 2 == 0 { return 3 }
                i = i + 1
            }

            // Test partitioned(by:) - returns two arrays, preserves order
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(4); arr2.append(5);
            let (evens, odds) = arr2.partitioned(by: { (x) in x % 2 == 0 });
            if evens.count != 2 { return 4 }
            if odds.count != 3 { return 5 }
            if evens(0) != 2 { return 6 }
            if evens(1) != 4 { return 7 }
            if odds(0) != 1 { return 8 }
            if odds(1) != 3 { return 9 }
            if odds(2) != 5 { return 10 }

            // partitioned on empty array
            let empty = std.collections.Array[std.num.Int64]();
            let (emptyMatch, emptyNot) = empty.partitioned(by: { (x) in true });
            if emptyMatch.count != 0 { return 11 }
            if emptyNot.count != 0 { return 12 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn equatable_ops() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var a = std.collections.Array[std.num.Int64]();
            a.append(1); a.append(2); a.append(3);
            var b = std.collections.Array[std.num.Int64]();
            b.append(1); b.append(2); b.append(3);
            var c = std.collections.Array[std.num.Int64]();
            c.append(1); c.append(2);
            var d = std.collections.Array[std.num.Int64]();
            d.append(3); d.append(2); d.append(1);

            // equals(other:) - positional single-name param
            if a.equals(b) == false { return 1 }
            if a.equals(c) { return 2 }
            if a.equals(d) { return 3 }

            // empty arrays are equal
            let e1 = std.collections.Array[std.num.Int64]();
            let e2 = std.collections.Array[std.num.Int64]();
            if e1.equals(e2) == false { return 4 }

            // contains(element:) - positional single-name param
            if a.contains(2) == false { return 5 }
            if a.contains(10) { return 6 }

            // firstIndex(of:)
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(2); arr.append(1);
            let fi = arr.firstIndex(of: 2);
            if fi.isNone() { return 7 }
            if fi.unwrap() != 1 { return 8 }

            let fiNone = arr.firstIndex(of: 10);
            if fiNone.isSome() { return 9 }

            // lastIndex(of:)
            let li = arr.lastIndex(of: 2);
            if li.isNone() { return 10 }
            if li.unwrap() != 3 { return 11 }

            let liNone = arr.lastIndex(of: 10);
            if liNone.isSome() { return 12 }

            // starts(with:)
            var prefix12 = std.collections.Array[std.num.Int64]();
            prefix12.append(1); prefix12.append(2);
            if a.starts(with: prefix12) == false { return 13 }
            if a.starts(with: a) == false { return 14 }
            var prefix23 = std.collections.Array[std.num.Int64]();
            prefix23.append(2); prefix23.append(3);
            if a.starts(with: prefix23) { return 15 }
            // empty prefix always matches
            let emptyArr = std.collections.Array[std.num.Int64]();
            if a.starts(with: emptyArr) == false { return 16 }
            // prefix longer than array
            if c.starts(with: a) { return 17 }

            // ends(with:)
            if a.ends(with: prefix23) == false { return 18 }
            if a.ends(with: a) == false { return 19 }
            if a.ends(with: prefix12) { return 20 }
            if a.ends(with: emptyArr) == false { return 21 }

            // split(separator:) - positional single-name param
            var splitArr = std.collections.Array[std.num.Int64]();
            splitArr.append(1); splitArr.append(0); splitArr.append(2); splitArr.append(0); splitArr.append(3);
            let parts = splitArr.split(0);
            if parts.count != 3 { return 22 }
            if parts(0).count != 1 { return 23 }
            if parts(0)(unchecked: 0) != 1 { return 24 }
            if parts(1).count != 1 { return 25 }
            if parts(1)(unchecked: 0) != 2 { return 26 }
            if parts(2).count != 1 { return 27 }
            if parts(2)(unchecked: 0) != 3 { return 28 }

            // split with no separator found
            let noSepParts = a.split(0);
            if noSepParts.count != 1 { return 29 }
            if noSepParts(0).count != 3 { return 30 }

            // dedup() - removes consecutive duplicates
            var dedArr = std.collections.Array[std.num.Int64]();
            dedArr.append(1); dedArr.append(1); dedArr.append(2); dedArr.append(2);
            dedArr.append(2); dedArr.append(3); dedArr.append(1); dedArr.append(1);
            dedArr.dedup();
            if dedArr.count != 4 { return 31 }
            if dedArr(0) != 1 { return 32 }
            if dedArr(1) != 2 { return 33 }
            if dedArr(2) != 3 { return 34 }
            if dedArr(3) != 1 { return 35 }

            // deduped() - returns new array
            var dedSrc = std.collections.Array[std.num.Int64]();
            dedSrc.append(1); dedSrc.append(1); dedSrc.append(2); dedSrc.append(2); dedSrc.append(3);
            let dedResult = dedSrc.deduped();
            if dedResult.count != 3 { return 36 }
            if dedResult(0) != 1 { return 37 }
            if dedResult(1) != 2 { return 38 }
            if dedResult(2) != 3 { return 39 }
            // original unchanged
            if dedSrc.count != 5 { return 40 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn comparable_ops() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // sort() - in-place ascending
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(3); arr.append(1); arr.append(4); arr.append(1);
            arr.append(5); arr.append(9); arr.append(2); arr.append(6);
            arr.sort();
            if arr(0) != 1 { return 1 }
            if arr(1) != 1 { return 2 }
            if arr(2) != 2 { return 3 }
            if arr(3) != 3 { return 4 }
            if arr(4) != 4 { return 5 }
            if arr(5) != 5 { return 6 }
            if arr(6) != 6 { return 7 }
            if arr(7) != 9 { return 8 }

            // sorted() - returns new sorted array
            var unsorted = std.collections.Array[std.num.Int64]();
            unsorted.append(5); unsorted.append(3); unsorted.append(1);
            unsorted.append(4); unsorted.append(2);
            let s = unsorted.sorted();
            if s(0) != 1 { return 9 }
            if s(1) != 2 { return 10 }
            if s(2) != 3 { return 11 }
            if s(3) != 4 { return 12 }
            if s(4) != 5 { return 13 }
            // original unchanged
            if unsorted(0) != 5 { return 14 }

            // min()
            let minVal = unsorted.min();
            if minVal.isNone() { return 15 }
            if minVal.unwrap() != 1 { return 16 }

            // min() on empty
            let empty = std.collections.Array[std.num.Int64]();
            let minEmpty = empty.min();
            if minEmpty.isSome() { return 17 }

            // max()
            let maxVal = unsorted.max();
            if maxVal.isNone() { return 18 }
            if maxVal.unwrap() != 5 { return 19 }

            // max() on empty
            let maxEmpty = empty.max();
            if maxEmpty.isSome() { return 20 }

            // isSorted()
            if s.isSorted() == false { return 21 }
            if unsorted.isSorted() { return 22 }
            // empty is sorted
            if empty.isSorted() == false { return 23 }
            // single element is sorted
            var single = std.collections.Array[std.num.Int64]();
            single.append(42);
            if single.isSorted() == false { return 24 }
            // equal elements are sorted
            var eq = std.collections.Array[std.num.Int64]();
            eq.append(3); eq.append(3); eq.append(3);
            if eq.isSorted() == false { return 25 }

            // binarySearch(element:) - positional single-name param
            var sorted = std.collections.Array[std.num.Int64]();
            sorted.append(1); sorted.append(2); sorted.append(3);
            sorted.append(4); sorted.append(5);
            let bs = sorted.binarySearch(3);
            if bs.isNone() { return 26 }
            if bs.unwrap() != 2 { return 27 }

            // binarySearch - not found
            let bsNone = sorted.binarySearch(6);
            if bsNone.isSome() { return 28 }

            // binarySearch - first element
            let bsFirst = sorted.binarySearch(1);
            if bsFirst.isNone() { return 29 }
            if bsFirst.unwrap() != 0 { return 30 }

            // binarySearch - last element
            let bsLast = sorted.binarySearch(5);
            if bsLast.isNone() { return 31 }
            if bsLast.unwrap() != 4 { return 32 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn hash_ops() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // unique() - returns new array with duplicates removed, preserving first occurrence order
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(1); arr.append(3); arr.append(2); arr.append(4);
            let u = arr.unique();
            if u.count != 4 { return 1 }
            if u(0) != 1 { return 2 }
            if u(1) != 2 { return 3 }
            if u(2) != 3 { return 4 }
            if u(3) != 4 { return 5 }
            // original unchanged
            if arr.count != 6 { return 6 }

            // unique() on array with no duplicates
            var noDups = std.collections.Array[std.num.Int64]();
            noDups.append(1); noDups.append(2); noDups.append(3);
            let noDupsU = noDups.unique();
            if noDupsU.count != 3 { return 7 }

            // unique() on empty
            let empty = std.collections.Array[std.num.Int64]();
            let emptyU = empty.unique();
            if emptyU.count != 0 { return 8 }

            // removeDuplicates() - in place
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(1); arr2.append(3); arr2.append(2);
            arr2.removeDuplicates();
            if arr2.count != 3 { return 9 }
            if arr2(0) != 1 { return 10 }
            if arr2(1) != 2 { return 11 }
            if arr2(2) != 3 { return 12 }

            // removeDuplicates on all same
            var allSame = std.collections.Array[std.num.Int64]();
            allSame.append(5); allSame.append(5); allSame.append(5); allSame.append(5);
            allSame.removeDuplicates();
            if allSame.count != 1 { return 13 }
            if allSame(0) != 5 { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn custom_sort() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // sort(by:) - descending order
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(5); arr.append(3); arr.append(2); arr.append(4);
            arr.sort(by: { (a, b) in a > b });
            if arr(0) != 5 { return 1 }
            if arr(1) != 4 { return 2 }
            if arr(2) != 3 { return 3 }
            if arr(3) != 2 { return 4 }
            if arr(4) != 1 { return 5 }

            // sorted(by:) - returns new array
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(3); arr2.append(1); arr2.append(4); arr2.append(1); arr2.append(5);
            let desc = arr2.sorted(by: { (a, b) in a > b });
            if desc(0) != 5 { return 6 }
            if desc(1) != 4 { return 7 }
            if desc(2) != 3 { return 8 }
            if desc(3) != 1 { return 9 }
            if desc(4) != 1 { return 10 }
            // original unchanged
            if arr2(0) != 3 { return 11 }

            // sort(byKey:) - sort by absolute value (using negative values)
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(3); arr3.append(-1); arr3.append(4); arr3.append(-5); arr3.append(2);
            arr3.sort(byKey: { (x) in if x < 0 { 0 - x } else { x } });
            if arr3(0) != -1 { return 12 }
            if arr3(1) != 2 { return 13 }
            if arr3(2) != 3 { return 14 }
            if arr3(3) != 4 { return 15 }
            if arr3(4) != -5 { return 16 }

            // sorted(byKey:) - returns new array
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(-3); arr4.append(1); arr4.append(-2);
            let byAbs = arr4.sorted(byKey: { (x) in if x < 0 { 0 - x } else { x } });
            if byAbs(0) != 1 { return 17 }
            if byAbs(1) != -2 { return 18 }
            if byAbs(2) != -3 { return 19 }
            // original unchanged
            if arr4(0) != -3 { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn misc_extensions() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // flatten() - nested arrays
            var inner1 = std.collections.Array[std.num.Int64]();
            inner1.append(1); inner1.append(2);
            var inner2 = std.collections.Array[std.num.Int64]();
            inner2.append(3); inner2.append(4);
            var inner3 = std.collections.Array[std.num.Int64]();
            inner3.append(5);
            var nested = std.collections.Array[std.collections.Array[std.num.Int64]]();
            nested.append(inner1); nested.append(inner2); nested.append(inner3);
            let flat = nested.flatten();
            if flat.count != 5 { return 1 }
            if flat(0) != 1 { return 2 }
            if flat(1) != 2 { return 3 }
            if flat(2) != 3 { return 4 }
            if flat(3) != 4 { return 5 }
            if flat(4) != 5 { return 6 }

            // flatten with empty inner arrays
            var mixedInner1 = std.collections.Array[std.num.Int64]();
            mixedInner1.append(1);
            let mixedInner2 = std.collections.Array[std.num.Int64]();
            var mixedInner3 = std.collections.Array[std.num.Int64]();
            mixedInner3.append(2); mixedInner3.append(3);
            var mixed = std.collections.Array[std.collections.Array[std.num.Int64]]();
            mixed.append(mixedInner1); mixed.append(mixedInner2); mixed.append(mixedInner3);
            let flatMixed = mixed.flatten();
            if flatMixed.count != 3 { return 7 }
            if flatMixed(0) != 1 { return 8 }
            if flatMixed(1) != 2 { return 9 }
            if flatMixed(2) != 3 { return 10 }

            // flatten empty outer array
            let emptyOuter = std.collections.Array[std.collections.Array[std.num.Int64]]();
            let flatEmpty = emptyOuter.flatten();
            if flatEmpty.count != 0 { return 11 }

            // joined(separator:) - positional single-name param
            var nums = std.collections.Array[std.num.Int64]();
            nums.append(1); nums.append(2); nums.append(3);
            let j = nums.joined(", ");
            if j != "1, 2, 3" { return 12 }

            // joined with empty separator (default)
            let j2 = nums.joined();
            if j2 != "123" { return 13 }

            // joined on empty array
            let emptyNums = std.collections.Array[std.num.Int64]();
            let jEmpty = emptyNums.joined(", ");
            if jEmpty != "" { return 14 }

            // joined single element
            var single = std.collections.Array[std.num.Int64]();
            single.append(42);
            let jSingle = single.joined("-");
            if jSingle != "42" { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn shuffled_returns_same_elements() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // shuffled() returns a new array with same count
            let result = arr.shuffled();
            if result.count != 5 { return 1 }

            // Original unchanged
            if arr(0) != 1 { return 2 }
            if arr(1) != 2 { return 3 }
            if arr(2) != 3 { return 4 }
            if arr(3) != 4 { return 5 }
            if arr(4) != 5 { return 6 }

            // shuffled result contains all original elements
            if result.contains(1) == false { return 7 }
            if result.contains(2) == false { return 8 }
            if result.contains(3) == false { return 9 }
            if result.contains(4) == false { return 10 }
            if result.contains(5) == false { return 11 }

            // shuffle() mutating - same count and same elements
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(10); arr2.append(20); arr2.append(30);
            arr2.shuffle();
            if arr2.count != 3 { return 12 }
            if arr2.contains(10) == false { return 13 }
            if arr2.contains(20) == false { return 14 }
            if arr2.contains(30) == false { return 15 }

            // shuffled on empty array
            let empty = std.collections.Array[std.num.Int64]();
            let emptyResult = empty.shuffled();
            if emptyResult.count != 0 { return 16 }

            // shuffled on single element array
            var single = std.collections.Array[std.num.Int64]();
            single.append(42);
            let singleResult = single.shuffled();
            if singleResult.count != 1 { return 17 }
            if singleResult(0) != 42 { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn range_subscripts() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30); arr.append(40); arr.append(50);

            // subscript(range:) - basic slice
            let slice = arr(range: std.core.Range[std.num.Int64](1, 4));
            if slice.count != 3 { return 1 }
            if slice(unchecked: 0) != 20 { return 2 }
            if slice(unchecked: 1) != 30 { return 3 }
            if slice(unchecked: 2) != 40 { return 4 }

            // subscript(range:) - full range
            let full = arr(range: std.core.Range[std.num.Int64](0, 5));
            if full.count != 5 { return 5 }
            if full(unchecked: 0) != 10 { return 6 }
            if full(unchecked: 4) != 50 { return 7 }

            // subscript(range:) - empty range
            let emptySlice = arr(range: std.core.Range[std.num.Int64](2, 2));
            if emptySlice.count != 0 { return 8 }

            // subscript(checkedRange:) - valid range
            let checked = arr(checkedRange: std.core.Range[std.num.Int64](0, 3));
            if checked.isNone() { return 9 }
            let checkedSlice = checked.unwrap();
            if checkedSlice.count != 3 { return 10 }
            if checkedSlice(unchecked: 0) != 10 { return 11 }
            if checkedSlice(unchecked: 2) != 30 { return 12 }

            // subscript(checkedRange:) - invalid range (end out of bounds)
            let oob = arr(checkedRange: std.core.Range[std.num.Int64](0, 10));
            if oob.isSome() { return 13 }

            // subscript(checkedRange:) - negative start
            let negStart = arr(checkedRange: std.core.Range[std.num.Int64](-1, 3));
            if negStart.isSome() { return 14 }

            // subscript(uncheckedRange:) - valid range
            let unchecked = arr(uncheckedRange: std.core.Range[std.num.Int64](2, 4));
            if unchecked.count != 2 { return 15 }
            if unchecked(unchecked: 0) != 30 { return 16 }
            if unchecked(unchecked: 1) != 40 { return 17 }

            // subscript(clampingRange:) - range fully within bounds
            let clamped = arr(clampingRange: std.core.Range[std.num.Int64](1, 3));
            if clamped.count != 2 { return 18 }
            if clamped(unchecked: 0) != 20 { return 19 }
            if clamped(unchecked: 1) != 30 { return 20 }

            // subscript(clampingRange:) - out of bounds range clamped
            let clampedWide = arr(clampingRange: std.core.Range[std.num.Int64](-5, 100));
            if clampedWide.count != 5 { return 21 }
            if clampedWide(unchecked: 0) != 10 { return 22 }
            if clampedWide(unchecked: 4) != 50 { return 23 }

            // subscript(clampingRange:) - both indices past end
            let clampedPast = arr(clampingRange: std.core.Range[std.num.Int64](10, 20));
            if clampedPast.count != 0 { return 24 }

            // subscript(clampingRange:) - negative range clamped to start
            let clampedNeg = arr(clampingRange: std.core.Range[std.num.Int64](-5, 1));
            if clampedNeg.count != 1 { return 25 }
            if clampedNeg(unchecked: 0) != 10 { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn equatable_remove() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // remove(element:) - removes first occurrence, returns true
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(2); arr.append(4);
            let removed = arr.remove(2);
            if removed == false { return 1 }
            if arr.count != 4 { return 2 }
            // First 2 removed, second 2 remains
            if arr(0) != 1 { return 3 }
            if arr(1) != 3 { return 4 }
            if arr(2) != 2 { return 5 }
            if arr(3) != 4 { return 6 }

            // remove(element:) - element not found, returns false
            let notRemoved = arr.remove(99);
            if notRemoved { return 7 }
            if arr.count != 4 { return 8 }

            // remove(element:) on empty array
            var emptyArr = std.collections.Array[std.num.Int64]();
            let emptyRemoved = emptyArr.remove(1);
            if emptyRemoved { return 9 }

            // removeAll(element:) - removes all occurrences
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(1); arr2.append(2); arr2.append(3); arr2.append(2); arr2.append(4); arr2.append(2);
            arr2.removeAll(2);
            if arr2.count != 3 { return 10 }
            if arr2(0) != 1 { return 11 }
            if arr2(1) != 3 { return 12 }
            if arr2(2) != 4 { return 13 }

            // removeAll(element:) - element not present
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(1); arr3.append(2); arr3.append(3);
            arr3.removeAll(99);
            if arr3.count != 3 { return 14 }

            // removeAll(element:) - remove all elements (all same)
            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(5); arr4.append(5); arr4.append(5);
            arr4.removeAll(5);
            if arr4.count != 0 { return 15 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn append_from_iterable() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // appendFrom(iterable:) with a range
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2);
            arr.appendFrom(std.core.Range[std.num.Int64](3, 6));
            if arr.count != 5 { return 1 }
            if arr(0) != 1 { return 2 }
            if arr(1) != 2 { return 3 }
            if arr(2) != 3 { return 4 }
            if arr(3) != 4 { return 5 }
            if arr(4) != 5 { return 6 }

            // appendFrom with empty range
            arr.appendFrom(std.core.Range[std.num.Int64](0, 0));
            if arr.count != 5 { return 7 }

            // appendFrom on empty array
            var emptyArr = std.collections.Array[std.num.Int64]();
            emptyArr.appendFrom(std.core.Range[std.num.Int64](10, 13));
            if emptyArr.count != 3 { return 8 }
            if emptyArr(0) != 10 { return 9 }
            if emptyArr(1) != 11 { return 10 }
            if emptyArr(2) != 12 { return 11 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn indices_property() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // indices should return Range(0, count)
            let idx = arr.indices;
            if idx.start != 0 { return 1 }
            if idx.end != 3 { return 2 }

            // Iterate over indices and access elements
            var sum: std.num.Int64 = 0;
            for i in arr.indices {
                sum = sum + arr(i)
            }
            if sum != 60 { return 3 }

            // indices on empty array
            let empty = std.collections.Array[std.num.Int64]();
            let emptyIdx = empty.indices;
            if emptyIdx.start != 0 { return 4 }
            if emptyIdx.end != 0 { return 5 }

            // indices on single element array
            var single = std.collections.Array[std.num.Int64]();
            single.append(42);
            let singleIdx = single.indices;
            if singleIdx.start != 0 { return 6 }
            if singleIdx.end != 1 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn as_pointer() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // asPointer() returns a pointer to the internal buffer
            let ptr = arr.asPointer();

            // Read through the pointer to verify it points to the array data
            let val0 = ptr.read();
            if val0 != 10 { return 1 }

            let val1 = ptr.offset(by: 1).read();
            if val1 != 20 { return 2 }

            let val2 = ptr.offset(by: 2).read();
            if val2 != 30 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn as_slice() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(10); arr.append(20); arr.append(30);

            // asSlice() returns a Slice view of the entire array
            let slice = arr.asSlice();
            if slice.count != 3 { return 1 }
            if slice(unchecked: 0) != 10 { return 2 }
            if slice(unchecked: 1) != 20 { return 3 }
            if slice(unchecked: 2) != 30 { return 4 }

            // asSlice on empty array
            let empty = std.collections.Array[std.num.Int64]();
            let emptySlice = empty.asSlice();
            if emptySlice.count != 0 { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: init(repeating:count:) requires T: Cloneable. Int64 does not implement Cloneable,
// so this will likely fail at monomorphization. Keeping expect(Compiles).expect(Runs)
// to track when Int64 gains Cloneable support.
#[test]
fn init_repeating() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let arr = std.collections.Array[std.num.Int64](repeating: 7, 5);
            if arr.count != 5 { return 1 }
            if arr(0) != 7 { return 2 }
            if arr(1) != 7 { return 3 }
            if arr(4) != 7 { return 4 }

            // repeating with count 0
            let empty = std.collections.Array[std.num.Int64](repeating: 42, 0);
            if empty.count != 0 { return 5 }

            // repeating with count 1
            let single = std.collections.Array[std.num.Int64](repeating: 99, 1);
            if single.count != 1 { return 6 }
            if single(0) != 99 { return 7 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn shuffle_with_rng() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            var arr = std.collections.Array[std.num.Int64]();
            arr.append(1); arr.append(2); arr.append(3); arr.append(4); arr.append(5);

            // shuffle(using:) with a seeded RNG for deterministic results
            var rng = std.num.Lcg64(seed: 42);
            arr.shuffle(using: rng);

            // After shuffle, same count and same elements
            if arr.count != 5 { return 1 }
            if arr.contains(1) == false { return 2 }
            if arr.contains(2) == false { return 3 }
            if arr.contains(3) == false { return 4 }
            if arr.contains(4) == false { return 5 }
            if arr.contains(5) == false { return 6 }

            // shuffled(using:) returns new array, original unchanged
            var arr2 = std.collections.Array[std.num.Int64]();
            arr2.append(10); arr2.append(20); arr2.append(30);
            var rng2 = std.num.Lcg64(seed: 123);
            let result = arr2.shuffled(using: rng2);
            if result.count != 3 { return 7 }
            if result.contains(10) == false { return 8 }
            if result.contains(20) == false { return 9 }
            if result.contains(30) == false { return 10 }

            // Original unchanged
            if arr2(0) != 10 { return 11 }
            if arr2(1) != 20 { return 12 }
            if arr2(2) != 30 { return 13 }

            // Deterministic: same seed gives same result
            var arr3 = std.collections.Array[std.num.Int64]();
            arr3.append(1); arr3.append(2); arr3.append(3); arr3.append(4); arr3.append(5);
            var rng3a = std.num.Lcg64(seed: 999);
            arr3.shuffle(using: rng3a);

            var arr4 = std.collections.Array[std.num.Int64]();
            arr4.append(1); arr4.append(2); arr4.append(3); arr4.append(4); arr4.append(5);
            var rng3b = std.num.Lcg64(seed: 999);
            arr4.shuffle(using: rng3b);

            if arr3.equals(arr4) == false { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
