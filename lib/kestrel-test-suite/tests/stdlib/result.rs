use kestrel_test_suite::*;

#[test]
fn result_basic() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // Test isOk
            if ok.isOk() == false { return 1 }
            if err.isOk() { return 2 }

            // Test isErr
            if ok.isErr() { return 3 }
            if err.isErr() == false { return 4 }

            // Test unwrap on Ok
            if ok.unwrap() != 42 { return 5 }

            // Test unwrapOr on Ok (returns contained value)
            if ok.unwrapOr(0) != 42 { return 6 }

            // Test unwrapOr on Err (returns default)
            if err.unwrapOr(0) != 0 { return 7 }

            // Test unwrapErr on Err
            if err.unwrapErr() != 99 { return 8 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_transforms() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(10);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(5);

            // Test map on Ok - transforms value
            let mapped = ok.map({ (x) in x * 2 });
            if mapped.unwrap() != 20 { return 1 }

            // Test map on Err - passes through error
            let mappedErr = err.map({ (x) in x * 2 });
            if mappedErr.isOk() { return 2 }
            if mappedErr.unwrapErr() != 5 { return 3 }

            // Test flatMap on Ok
            let flatMapped = ok.flatMap[std.num.Int64]({ (x) in .Ok(x + 1) });
            if flatMapped.unwrap() != 11 { return 4 }

            // Test flatMap on Ok returning Err
            let flatMappedErr: std.result.Result[std.num.Int64, std.num.Int64] = ok.flatMap[std.num.Int64]({ (x) in .Err(x) });
            if flatMappedErr.isOk() { return 5 }
            if flatMappedErr.unwrapErr() != 10 { return 6 }

            // Test flatMap on Err - passes through error
            let flatMappedOnErr = err.flatMap[std.num.Int64]({ (x) in .Ok(x + 1) });
            if flatMappedOnErr.isOk() { return 7 }
            if flatMappedOnErr.unwrapErr() != 5 { return 8 }

            // Test mapErr on Err - transforms error
            let mappedErrVal = err.mapErr({ (e) in e * 10 });
            if mappedErrVal.isOk() { return 9 }
            if mappedErrVal.unwrapErr() != 50 { return 10 }

            // Test mapErr on Ok - passes through value
            let mappedErrOk = ok.mapErr({ (e) in e * 10 });
            if mappedErrOk.unwrap() != 10 { return 11 }

            // Test flatMapErr on Err - transforms error with Result-returning fn
            let flatMappedErrVal: std.result.Result[std.num.Int64, std.num.Int64] = err.flatMapErr[std.num.Int64]({ (e) in .Err(e * 10) });
            if flatMappedErrVal.isOk() { return 12 }
            if flatMappedErrVal.unwrapErr() != 50 { return 13 }

            // Test flatMapErr on Err recovering to Ok
            let recovered = err.flatMapErr[std.num.Int64]({ (e) in .Ok(e + 100) });
            if recovered.isErr() { return 14 }
            if recovered.unwrap() != 105 { return 15 }

            // Test flatMapErr on Ok - passes through value
            let flatMappedErrOk = ok.flatMapErr[std.num.Int64]({ (e) in .Err(e * 10) });
            if flatMappedErrOk.unwrap() != 10 { return 16 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_combinators() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(10);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(5);
            let other_ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(20);
            let other_err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // Test andValue - Ok and Ok = second Ok
            let andOkOk = ok.andValue(other_ok);
            if andOkOk.unwrap() != 20 { return 1 }

            // Test andValue - Ok and Err = Err
            let andOkErr = ok.andValue(other_err);
            if andOkErr.isOk() { return 2 }
            if andOkErr.unwrapErr() != 99 { return 3 }

            // Test andValue - Err and Ok = Err (original error)
            let andErrOk = err.andValue(other_ok);
            if andErrOk.isOk() { return 4 }
            if andErrOk.unwrapErr() != 5 { return 5 }

            // Test andThen - Ok with transform
            let andThenOk = ok.andThen[std.num.Int64]({ (x) in .Ok(x * 3) });
            if andThenOk.unwrap() != 30 { return 6 }

            // Test andThen - Err passes through
            let andThenErr = err.andThen[std.num.Int64]({ (x) in .Ok(x * 3) });
            if andThenErr.isOk() { return 7 }
            if andThenErr.unwrapErr() != 5 { return 8 }

            // Test orValue - Ok returns self
            let orOk = ok.orValue(other_ok);
            if orOk.unwrap() != 10 { return 9 }

            // Test orValue - Err returns other
            let orErr = err.orValue(other_ok);
            if orErr.unwrap() != 20 { return 10 }

            // Test orElse - Ok returns self
            let orElseOk: std.result.Result[std.num.Int64, std.num.Int64] = ok.orElse({ (e) in .Ok(e + 100) });
            if orElseOk.unwrap() != 10 { return 11 }

            // Test orElse - Err calls alternative
            let orElseErr: std.result.Result[std.num.Int64, std.num.Int64] = err.orElse({ (e) in .Ok(e + 100) });
            if orElseErr.unwrap() != 105 { return 12 }

            // Test orElse - Err returning new Err
            let orElseNewErr: std.result.Result[std.num.Int64, std.num.Int64] = err.orElse({ (e) in .Err(e * 10) });
            if orElseNewErr.isOk() { return 13 }
            if orElseNewErr.unwrapErr() != 50 { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_conversions() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // Test ok() on Ok - returns Some(value)
            let okOpt = ok.ok();
            if okOpt.isNone() { return 1 }
            if okOpt.unwrap() != 42 { return 2 }

            // Test ok() on Err - returns None
            let errOpt = err.ok();
            if errOpt.isSome() { return 3 }

            // Test err() on Err - returns Some(error)
            let errVal = err.err();
            if errVal.isNone() { return 4 }
            if errVal.unwrap() != 99 { return 5 }

            // Test err() on Ok - returns None
            let okErr = ok.err();
            if okErr.isSome() { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_unwrap_or_else() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(5);

            // unwrap(orElse:) on Ok - returns contained value, doesn't call function
            let okVal = ok.unwrap(orElse: { (e) in e * 100 });
            if okVal != 42 { return 1 }

            // unwrap(orElse:) on Err - calls function with error
            let errVal = err.unwrap(orElse: { (e) in e * 100 });
            if errVal != 500 { return 2 }

            // unwrap(orElse:) on Err with recovery to fixed value
            let recovered = err.unwrap(orElse: { (e) in 0 });
            if recovered != 0 { return 3 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_iter() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // iter on Ok - yields 1 element
            var okIter = ok.iter();
            let first = okIter.next();
            if first.isNone() { return 1 }
            if first.unwrap() != 42 { return 2 }

            // Second call returns None
            let second = okIter.next();
            if second.isSome() { return 3 }

            // iter on Err - yields 0 elements
            var errIter = err.iter();
            let errFirst = errIter.next();
            if errFirst.isSome() { return 4 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_equals() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok1: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let ok2: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let ok3: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(99);
            let err1: std.result.Result[std.num.Int64, std.num.Int64] = .Err(5);
            let err2: std.result.Result[std.num.Int64, std.num.Int64] = .Err(5);
            let err3: std.result.Result[std.num.Int64, std.num.Int64] = .Err(10);

            // Ok(42) == Ok(42)
            if ok1.equals(ok2) == false { return 1 }

            // Ok(42) != Ok(99)
            if ok1.equals(ok3) { return 2 }

            // Err(5) == Err(5)
            if err1.equals(err2) == false { return 3 }

            // Err(5) != Err(10)
            if err1.equals(err3) { return 4 }

            // Ok(42) != Err(5)
            if ok1.equals(err1) { return 5 }

            // Err(5) != Ok(42)
            if err1.equals(ok1) { return 6 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn result_format() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let ok: std.result.Result[std.num.Int64, std.num.Int64] = .Ok(42);
            let err: std.result.Result[std.num.Int64, std.num.Int64] = .Err(99);

            // Format Ok
            let okStr = ok.format();
            if okStr.equals("Ok(42)") == false { return 1 }

            // Format Err
            let errStr = err.format();
            if errStr.equals("Err(99)") == false { return 2 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
