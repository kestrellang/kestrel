use kestrel_test_suite::*;

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

#[test]
fn optional_extended() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Test isSomeAnd - Some with true predicate
            if some.isSomeAnd({ (x) in x > 0 }) == false { return 1 }

            // Test isSomeAnd - Some with false predicate
            if some.isSomeAnd({ (x) in x < 0 }) { return 2 }

            // Test isSomeAnd - None always false
            if none.isSomeAnd({ (x) in x > 0 }) { return 3 }

            // Test expect on Some (should not panic, returns value)
            if some.expect("should have value") != 42 { return 4 }

            // Test unwrap(orElse:) on Some (should return contained value, not call closure)
            if some.unwrap(orElse: { () in 99 }) != 42 { return 5 }

            // Test unwrap(orElse:) on None (should call closure)
            if none.unwrap(orElse: { () in 99 }) != 99 { return 6 }

            // Test inspect on Some (returns self unchanged)
            // Note: cannot modify captured variables in closures, so just verify return value
            let inspected = some.inspect({ (x) in });
            if inspected.unwrap() != 42 { return 7 }

            // Test inspect on None (returns None)
            let inspectedNone = none.inspect({ (x) in });
            if inspectedNone.isSome() { return 8 }

            // Test xor - Some xor None = Some
            let xorResult1 = some.xor(.None);
            if xorResult1.unwrap() != 42 { return 9 }

            // Test xor - None xor Some = Some
            let xorResult2 = none.xor(.Some(99));
            if xorResult2.unwrap() != 99 { return 10 }

            // Test xor - Some xor Some = None
            let xorResult3 = some.xor(.Some(99));
            if xorResult3.isSome() { return 11 }

            // Test xor - None xor None = None
            let xorResult4 = none.xor(.None);
            if xorResult4.isSome() { return 12 }

            // Test zip - Some zip Some = Some tuple
            let zipped = some.zip(with: .Some(100));
            if zipped.isNone() { return 13 }
            let pair = zipped.unwrap();
            if pair.0 != 42 { return 14 }
            if pair.1 != 100 { return 15 }

            // Test zip - Some zip None = None
            let zippedNone: std.result.Optional[(std.num.Int64, std.num.Int64)] = some.zip(with: .None);
            if zippedNone.isSome() { return 16 }

            // Test zip - None zip Some = None
            let noneZipped: std.result.Optional[(std.num.Int64, std.num.Int64)] = none.zip(with: .Some(100));
            if noneZipped.isSome() { return 17 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_mutation() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test take - takes value out, leaving None
            var opt1: std.result.Optional[std.num.Int64] = .Some(42);
            let taken = opt1.take();
            if taken.unwrap() != 42 { return 1 }
            if opt1.isSome() { return 2 }

            // Test take on None
            var opt2: std.result.Optional[std.num.Int64] = .None;
            let takenNone = opt2.take();
            if takenNone.isSome() { return 3 }
            if opt2.isSome() { return 4 }

            // Test replace - replaces value, returns old
            var opt3: std.result.Optional[std.num.Int64] = .Some(10);
            let old = opt3.replace(20);
            if old.unwrap() != 10 { return 5 }
            if opt3.unwrap() != 20 { return 6 }

            // Test replace on None - returns None, sets to Some
            var opt4: std.result.Optional[std.num.Int64] = .None;
            let oldNone = opt4.replace(50);
            if oldNone.isSome() { return 7 }
            if opt4.unwrap() != 50 { return 8 }

            // Test takeIf - predicate true, takes value
            var opt5: std.result.Optional[std.num.Int64] = .Some(42);
            let takenIf = opt5.takeIf({ (x) in x > 0 });
            if takenIf.unwrap() != 42 { return 9 }
            if opt5.isSome() { return 10 }

            // Test takeIf - predicate false, leaves value
            var opt6: std.result.Optional[std.num.Int64] = .Some(42);
            let notTaken = opt6.takeIf({ (x) in x < 0 });
            if notTaken.isSome() { return 11 }
            if opt6.isNone() { return 12 }
            if opt6.unwrap() != 42 { return 13 }

            // Test takeIf on None
            var opt7: std.result.Optional[std.num.Int64] = .None;
            let takeIfNone = opt7.takeIf({ (x) in x > 0 });
            if takeIfNone.isSome() { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_flatten() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test flatten on Some(Some(value))
            let nested: std.result.Optional[std.result.Optional[std.num.Int64]] = .Some(.Some(42));
            let flat = nested.flatten();
            if flat.isNone() { return 1 }
            if flat.unwrap() != 42 { return 2 }

            // Test flatten on Some(None)
            let someNone: std.result.Optional[std.result.Optional[std.num.Int64]] = .Some(.None);
            let flat2 = someNone.flatten();
            if flat2.isSome() { return 3 }

            // Test flatten on None
            let none: std.result.Optional[std.result.Optional[std.num.Int64]] = .None;
            let flat3 = none.flatten();
            if flat3.isSome() { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Known limitation - okOr(error:) and okOrElse(error:) methods cannot be resolved
// on Optional[Int64]. The method lookup fails even with explicit Result type annotation.
#[test]
fn optional_ok_or() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Test okOr on Some - returns Ok(value)
            let okResult: std.result.Result[std.num.Int64, std.num.Int64] = some.okOr( 99);
            if okResult.isOk() == false { return 1 }
            if okResult.unwrap() != 42 { return 2 }

            // Test okOr on None - returns Err(error)
            let errResult: std.result.Result[std.num.Int64, std.num.Int64] = none.okOr( 99);
            if errResult.isErr() == false { return 3 }
            if errResult.unwrapErr() != 99 { return 4 }

            // Test okOrElse on Some - returns Ok(value), no call
            let okResult2: std.result.Result[std.num.Int64, std.num.Int64] = some.okOrElse({ () in 99 });
            if okResult2.isOk() == false { return 5 }
            if okResult2.unwrap() != 42 { return 6 }

            // Test okOrElse on None - calls function, returns Err
            let errResult2: std.result.Result[std.num.Int64, std.num.Int64] = none.okOrElse({ () in 99 });
            if errResult2.isErr() == false { return 7 }
            if errResult2.unwrapErr() != 99 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Test iter on Some - yields 1 element
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            var iter = some.iter();
            let first = iter.next();
            if first.isNone() { return 1 }
            if first.unwrap() != 42 { return 2 }

            // Second call should return None
            let second = iter.next();
            if second.isSome() { return 3 }

            // Test iter on None - yields 0 elements
            let none: std.result.Optional[std.num.Int64] = .None;
            var iter2 = none.iter();
            let noneFirst = iter2.next();
            if noneFirst.isSome() { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_equals() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let a: std.result.Optional[std.num.Int64] = .Some(42);
            let b: std.result.Optional[std.num.Int64] = .Some(42);
            let c: std.result.Optional[std.num.Int64] = .Some(99);
            let none1: std.result.Optional[std.num.Int64] = .None;
            let none2: std.result.Optional[std.num.Int64] = .None;

            // Some(42) == Some(42)
            if a.equals(b) == false { return 1 }

            // Some(42) != Some(99)
            if a.equals(c) { return 2 }

            // Some(42) != None
            if a.equals(none1) { return 3 }

            // None != Some(42)
            if none1.equals(a) { return 4 }

            // None == None
            if none1.equals(none2) == false { return 5 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_contains() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Some(42) contains 42
            if some.contains(42) == false { return 1 }

            // Some(42) does not contain 99
            if some.contains(99) { return 2 }

            // None does not contain anything
            if none.contains(42) { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_compare() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let none: std.result.Optional[std.num.Int64] = .None;
            let some1: std.result.Optional[std.num.Int64] = .Some(1);
            let some2: std.result.Optional[std.num.Int64] = .Some(2);
            let some1b: std.result.Optional[std.num.Int64] = .Some(1);

            // None < Some(x) for any x
            if none.compare(some1) != std.core.Ordering.Less { return 1 }

            // Some(x) > None
            if some1.compare(none) != std.core.Ordering.Greater { return 2 }

            // None == None
            if none.compare(none) != std.core.Ordering.Equal { return 3 }

            // Some(1) < Some(2)
            if some1.compare(some2) != std.core.Ordering.Less { return 4 }

            // Some(2) > Some(1)
            if some2.compare(some1) != std.core.Ordering.Greater { return 5 }

            // Some(1) == Some(1)
            if some1.compare(some1b) != std.core.Ordering.Equal { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_clone() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Clone of Some
            let clonedSome = some.clone();
            if clonedSome.isNone() { return 1 }
            if clonedSome.unwrap() != 42 { return 2 }

            // Clone of None
            let clonedNone = none.clone();
            if clonedNone.isSome() { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_format() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let some: std.result.Optional[std.num.Int64] = .Some(42);
            let none: std.result.Optional[std.num.Int64] = .None;

            // Format Some
            let someStr = some.format();
            if someStr.equals("Some(42)") == false { return 1 }

            // Format None
            let noneStr = none.format();
            if noneStr.equals("None") == false { return 2 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn optional_hash() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // Hash test: Some(42) and Some(42) should hash the same
            // None and None should hash the same
            // Some(42) and None should hash differently (very likely)
            let a: std.result.Optional[std.num.Int64] = .Some(42);
            let b: std.result.Optional[std.num.Int64] = .Some(42);
            let c: std.result.Optional[std.num.Int64] = .None;

            var hasherA = std.collections.DefaultHasher();
            a.hash(into: hasherA);
            let hashA = hasherA.finish();

            var hasherB = std.collections.DefaultHasher();
            b.hash(into: hasherB);
            let hashB = hasherB.finish();

            var hasherC = std.collections.DefaultHasher();
            c.hash(into: hasherC);
            let hashC = hasherC.finish();

            // Equal values should produce equal hashes
            if hashA != hashB { return 1 }

            // Some and None should differ (in practice, always true)
            if hashA == hashC { return 2 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
