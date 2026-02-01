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
                if arr.isEmpty() == false { return 1 }

                // Test append and count
                arr.append(10);
                arr.append(20);
                arr.append(30);
                if arr.count() != 3 { return 2 }
                if arr.isEmpty() { return 3 }

                // Test first and last
                if arr.first().unwrap() != 10 { return 4 }
                if arr.last().unwrap() != 30 { return 5 }

                // Test pop
                let popped = arr.pop();
                if popped.unwrap() != 30 { return 6 }
                if arr.count() != 2 { return 7 }

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

                // Test getValue (safe access)
                let val = arr.getValue(at: 1);
                if val.isNone() { return 1 }
                if val.unwrap() != 20 { return 2 }

                // Test out of bounds returns None
                let oob = arr.getValue(at: 100);
                if oob.isSome() { return 3 }

                // Test getUnchecked
                if arr.getUnchecked(0) != 10 { return 4 }
                if arr.getUnchecked(2) != 30 { return 5 }

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
                if dict.isEmpty() == false { return 1 }

                // Test insert and count
                let _ = dict.insert(1, 100);
                let _ = dict.insert(2, 200);
                if dict.count() != 2 { return 2 }

                // Test contains
                if dict.contains(1) == false { return 3 }
                if dict.contains(999) { return 4 }

                // Test getValue
                let val = dict.getValue(2);
                if val.isNone() { return 5 }
                if val.unwrap() != 200 { return 6 }

                // Test remove
                let removed = dict.remove(1);
                if removed.isNone() { return 7 }
                if removed.unwrap() != 100 { return 8 }
                if dict.count() != 1 { return 9 }

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

                // Test andValue
                let andResult = some.andValue(other);
                if andResult.unwrap() != 20 { return 1 }
                let andNone = none.andValue(other);
                if andNone.isSome() { return 2 }

                // Test orValue
                let orResult = none.orValue(other);
                if orResult.unwrap() != 20 { return 3 }
                let orSome = some.orValue(other);
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
