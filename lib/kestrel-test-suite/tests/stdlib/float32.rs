use kestrel_test_suite::*;

#[test]
fn float32_boundary_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // maxValue — 3.4028235e38
            let maxVal = std.num.Float32.maxValue;
            if maxVal.isFinite == false { return 1 }
            if maxVal.isPositive == false { return 2 }
            // maxValue should be greater than 3.0e38
            let threshold: std.num.Float32 = 3.0e38;
            if maxVal < threshold { return 3 }

            // minValue — -3.4028235e38
            let minVal = std.num.Float32.minValue;
            if minVal.isFinite == false { return 4 }
            if minVal.isNegative == false { return 5 }
            // minValue should be the negation of maxValue
            if minVal.negate().equals(maxVal) == false { return 6 }

            // minPositive — 1.17549435e-38 (smallest normal)
            let minPos = std.num.Float32.minPositive;
            if minPos.isPositive == false { return 7 }
            if minPos.isNormal == false { return 8 }

            // A value just below minPositive should be subnormal
            let two: std.num.Float32 = 2.0;
            let halfMinPos = minPos.divide(two);
            if halfMinPos.isSubnormal == false { return 9 }
            if halfMinPos.isNormal { return 10 }

            // epsilon — 1.1920929e-7
            let eps = std.num.Float32.epsilon;
            if eps.isPositive == false { return 11 }
            // 1.0 + epsilon should not equal 1.0
            let one: std.num.Float32 = 1.0;
            let onePlusEps = one.add(eps);
            if onePlusEps.equals(one) { return 12 }

            // Classification near subnormal boundary
            // minPositive itself is normal
            if minPos.isNormal == false { return 13 }
            if minPos.isSubnormal { return 14 }
            // nextDown from minPositive should be subnormal
            let belowMinPos = minPos.nextDown();
            if belowMinPos.isSubnormal == false { return 15 }
            if belowMinPos.isNormal { return 16 }
            // The subnormal value should still be finite and positive
            if belowMinPos.isFinite == false { return 17 }
            if belowMinPos.isPositive == false { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float32_precision_and_rounding() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // round() near maxValue — should not overflow
            let maxVal = std.num.Float32.maxValue;
            let rounded = maxVal.round();
            if rounded.isFinite == false { return 1 }
            if rounded.equals(maxVal) == false { return 2 }

            // trunc() near maxValue — should return maxValue (already integer)
            let truncated = maxVal.trunc();
            if truncated.isFinite == false { return 3 }
            if truncated.equals(maxVal) == false { return 4 }

            // trunc() on a large positive float
            let large: std.num.Float32 = 1.0e30;
            let truncLarge = large.trunc();
            if truncLarge.equals(large) == false { return 5 }

            // Precision in trig: sin(pi) should be very close to 0 but may not be exactly 0
            let pi = std.num.Float32.pi;
            let sinPi = pi.sin();
            // For Float32, sin(pi) is approximate. Check that abs < epsilon-ish (1e-6)
            let tolerance: std.num.Float32 = 1.0e-6;
            if sinPi.abs() > tolerance { return 6 }

            // cos(0) should be exactly 1.0
            let zero: std.num.Float32 = 0.0;
            let cosZero = zero.cos();
            let oneF: std.num.Float32 = 1.0;
            if cosZero.equals(oneF) == false { return 7 }

            // Float32 precision: adding a small value to a large value
            // 1e7 + 1.0 should still work in Float32 (within 24-bit mantissa)
            let bigVal: std.num.Float32 = 1.0e7;
            let bigOneF: std.num.Float32 = 1.0;
            let bigPlusOne = bigVal.add(bigOneF);
            let expected: std.num.Float32 = 10000001.0;
            if bigPlusOne.equals(expected) == false { return 8 }

            // But 1e8 + 1.0 may lose the +1 due to Float32 precision limits
            let veryBig: std.num.Float32 = 1.0e8;
            let veryBigOneF: std.num.Float32 = 1.0;
            let veryBigPlusOne = veryBig.add(veryBigOneF);
            // The 1.0 is lost in Float32 precision, so result == veryBig
            if veryBigPlusOne.equals(veryBig) == false { return 9 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float32_conversion() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // init(from: Float64) — narrowing conversion
            let f64val: std.num.Float64 = 3.14;
            let f32fromF64 = std.num.Float32(from: f64val);
            // Should be approximately 3.14 (Float32 precision)
            let f32ThreePtOneFour: std.num.Float32 = 3.14;
            let diff = f32fromF64.subtract(f32ThreePtOneFour).abs();
            let tolerance: std.num.Float32 = 1.0e-6;
            if diff > tolerance { return 1 }

            // init(from: Int64)
            let i64val: std.num.Int64 = 42;
            let f32fromI64 = std.num.Float32(from: i64val);
            let fortyTwoF: std.num.Float32 = 42.0;
            if f32fromI64.equals(fortyTwoF) == false { return 2 }

            // init(from: Int64) with negative
            let negI64: std.num.Int64 = -100;
            let f32fromNeg = std.num.Float32(from: negI64);
            let negHundredF: std.num.Float32 = -100.0;
            if f32fromNeg.equals(negHundredF) == false { return 3 }

            // toInt64 — normal case
            let f32val: std.num.Float32 = 3.7;
            let asInt = f32val.toInt64();
            if asInt.isNone() { return 4 }
            if asInt.unwrap() != 3 { return 5 }

            // toInt64 — negative truncation toward zero
            let negFloat: std.num.Float32 = -3.7;
            let negAsInt = negFloat.toInt64();
            if negAsInt.isNone() { return 6 }
            if negAsInt.unwrap() != -3 { return 7 }

            // toInt64 — NaN returns None
            let nan = std.num.Float32.nan;
            let nanAsInt = nan.toInt64();
            if nanAsInt.isSome() { return 8 }

            // toInt64 — infinity returns None
            let inf = std.num.Float32.infinity;
            let infAsInt = inf.toInt64();
            if infAsInt.isSome() { return 9 }

            // toFloat64 — widening conversion
            let small: std.num.Float32 = 1.5;
            let asF64 = small.toFloat64();
            // 1.5 is exactly representable in both Float32 and Float64
            let onePointFiveF64: std.num.Float64 = 1.5;
            if asF64.equals(onePointFiveF64) == false { return 10 }

            // parse — valid float
            let parsed = std.num.Float32.parse("3.14");
            if parsed.isNone() { return 11 }
            let parsedVal = parsed.unwrap();
            let parseThreePtOneFour: std.num.Float32 = 3.14;
            let parseDiff = parsedVal.subtract(parseThreePtOneFour).abs();
            if parseDiff > tolerance { return 12 }

            // parse — negative value
            let parsedNeg = std.num.Float32.parse("-2.5");
            if parsedNeg.isNone() { return 13 }
            let parsedNegVal = parsedNeg.unwrap();
            let negTwoPointFive: std.num.Float32 = -2.5;
            if parsedNegVal.equals(negTwoPointFive) == false { return 14 }

            // parse — scientific notation
            let parsedSci = std.num.Float32.parse("1.5e2");
            if parsedSci.isNone() { return 15 }
            let oneHundredFifty: std.num.Float32 = 150.0;
            if parsedSci.unwrap().equals(oneHundredFifty) == false { return 16 }

            // parse — "nan"
            let parsedNan = std.num.Float32.parse("nan");
            if parsedNan.isNone() { return 17 }
            if parsedNan.unwrap().isNaN == false { return 18 }

            // parse — "inf"
            let parsedInf = std.num.Float32.parse("inf");
            if parsedInf.isNone() { return 19 }
            if parsedInf.unwrap().isInfinite == false { return 20 }

            // parse — invalid string
            let parsedBad = std.num.Float32.parse("abc");
            if parsedBad.isSome() { return 21 }

            // parse — empty string
            let parsedEmpty = std.num.Float32.parse("");
            if parsedEmpty.isSome() { return 22 }

            // parse — integer string
            let parsedInt = std.num.Float32.parse("42");
            if parsedInt.isNone() { return 23 }
            let parsedFortyTwo: std.num.Float32 = 42.0;
            if parsedInt.unwrap().equals(parsedFortyTwo) == false { return 24 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
