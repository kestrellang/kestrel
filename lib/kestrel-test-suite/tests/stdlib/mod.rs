//! Standard library tests
//!
//! Tests for the Kestrel standard library types and functions.
//! Each test validates multiple methods to provide comprehensive coverage.

use kestrel_test_suite::*;

mod array {
    use super::*;

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
}

mod set {
    use super::*;

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
}

mod string {
    use super::*;

    #[test]
    fn search_operations() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let s: std.text.String = "hello world";

                // Test contains
                if s.contains("world") == false { return 1 }
                if s.contains("xyz") { return 2 }

                // Test find
                let pos = s.find("world");
                if pos.isNone() { return 3 }
                if pos.unwrap() != 6 { return 4 }

                // Test starts/ends with
                if s.starts(with: "hello") == false { return 5 }
                if s.starts(with: "world") { return 6 }

                // Test ends with
                if s.ends(with: "world") == false { return 7 }
                if s.ends(with: "hello") { return 8 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn manipulation() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                // Test append
                var s = std.text.String();
                s.append("hello");
                s.append(" world");
                if s.byteCount != 11 { return 1 }

                // Test trim
                let padded: std.text.String = "  hello  ";
                let trimmed = padded.trimmed();
                if trimmed.byteCount != 5 { return 2 }

                // Test lowercase/uppercase
                let mixed: std.text.String = "HeLLo";
                let lower = mixed.lowercased();
                let upper = mixed.uppercased();
                if lower.equals("hello") == false { return 3 }
                if upper.equals("HELLO") == false { return 4 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod numeric {
    use super::*;

    #[test]
    fn int64_operations() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let a: std.num.Int64 = 10;
                let b: std.num.Int64 = 3;

                // Test arithmetic
                if a.add(b) != 13 { return 1 }
                if a.subtract(b) != 7 { return 2 }
                if a.multiply(b) != 30 { return 3 }
                if a.divide(b) != 3 { return 4 }
                if a.modulo(b) != 1 { return 5 }

                // Test negate and abs
                let neg: std.num.Int64 = -5;
                if neg.negate() != 5 { return 6 }
                if neg.abs() != 5 { return 7 }

                // Test comparison - a > b so compare should return Greater
                let cmp = a.compare(b);
                match cmp {
                    .Greater => 0,
                    _ => 8
                }
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod boolean {
    use super::*;

    #[test]
    fn operations() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let t: std.core.Bool = true;
                let f: std.core.Bool = false;

                // Test and (uses closure-based logicalAnd)
                if (t and t) == false { return 1 }
                if t and f { return 2 }

                // Test or (uses closure-based logicalOr)
                if (t or f) == false { return 3 }
                if f or f { return 4 }

                // Test logicalNot
                if t.logicalNot() { return 5 }
                if f.logicalNot() == false { return 6 }

                // Test equals
                if t.equals(t) == false { return 7 }
                if t.equals(f) { return 8 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod dictionary {
    use super::*;

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
}

mod rcbox {
    use super::*;

    #[test]
    fn reference_counting() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                // Create RcBox
                let box1 = std.memory.RcBox[std.num.Int64](42);

                // Test getValue
                if box1.getValue() != 42 { return 1 }

                // Test initial refCount is 1
                if box1.refCount() != 1 { return 2 }

                // Test isUnique
                if box1.isUnique() == false { return 3 }

                // Test clone increments refCount
                let box2 = box1.clone();
                if box1.refCount() != 2 { return 4 }
                if box1.isUnique() { return 5 }

                // Both share the same value
                if box2.getValue() != 42 { return 6 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod optional {
    use super::*;

    #[test]
    fn basic_operations() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let some: std.result.Optional[std.num.Int64] = .Some(42);
                let none: std.result.Optional[std.num.Int64] = .None;

                // Test isSome/isNone
                if some.isSome() == false { return 1 }
                if some.isNone() { return 2 }
                if none.isSome() { return 3 }
                if none.isNone() == false { return 4 }

                // Test unwrap
                if some.unwrap() != 42 { return 5 }

                // Test unwrapOr
                if some.unwrapOr(0) != 42 { return 6 }
                if none.unwrapOr(99) != 99 { return 7 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn combinators() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let some: std.result.Optional[std.num.Int64] = .Some(10);
                let none: std.result.Optional[std.num.Int64] = .None;
                let other: std.result.Optional[std.num.Int64] = .Some(20);

                // Test then (and combinator)
                let andResult = some.then(other);
                if andResult.unwrap() != 20 { return 1 }
                let andNone = none.then(other);
                if andNone.isSome() { return 2 }

                // Test orElse (without closure capture to avoid codegen bug)
                let orResult: std.result.Optional[std.num.Int64] = none.orElse({ () in .Some(99) });
                if orResult.unwrap() != 99 { return 3 }
                let orSome: std.result.Optional[std.num.Int64] = some.orElse({ () in .Some(99) });
                if orSome.unwrap() != 10 { return 4 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }

    #[test]
    fn higher_order_combinators() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                let some: std.result.Optional[std.num.Int64] = .Some(10);

                // Test map
                let mapped = some.map({ (x) in x * 2 });
                if mapped.unwrap() != 20 { return 1 }

                // Test filter
                let filtered = some.filter({ (x) in x > 5 });
                if filtered.isNone() { return 2 }
                let filteredOut = some.filter({ (x) in x > 100 });
                if filteredOut.isSome() { return 3 }

                // Test flatMap
                let flatMapped = some.flatMap[std.num.Int64]({ (x) in .Some(x + 1) });
                if flatMapped.unwrap() != 11 { return 4 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod range {
    use super::*;

    #[test]
    fn iteration() {
        Test::new(
            r#"module Test

            func main() -> lang.i64 {
                // Create range 0..<5
                let r = std.core.Range[std.num.Int64](0, 5);

                // Iterate and sum
                var sum: std.num.Int64 = 0;
                var iter = r.iter();
                var done: std.core.Bool = false;
                while done == false {
                    let next = iter.next();
                    if next.isSome() {
                        sum = sum + next.unwrap()
                    } else {
                        done = true
                    }
                }

                // 0 + 1 + 2 + 3 + 4 = 10
                if sum != 10 { return 1 }

                0
            }
        "#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(Runs);
    }
}

mod array_iteration {
    use super::*;

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
}

mod iterator_adapters {
    use super::*;

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
        .expect(Compiles)
        .expect(Runs);
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
}
