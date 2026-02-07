use kestrel_test_suite::*;

#[test]
fn float64_constructors_and_constants() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            // init() - default should be zero
            let defaultVal = std.num.Float64();
            if defaultVal.isZero == false { return 1 }

            // init(floatLiteral:)
            let fromLiteral: std.num.Float64 = 3.14;
            if fromLiteral.isZero { return 2 }

            // init(intLiteral:)
            let fromInt: std.num.Float64 = 42;
            let expected42: std.num.Float64 = 42.0;
            if fromInt.equals(expected42) == false { return 3 }

            // init(from: Int64)
            let intVal: std.num.Int64 = 100;
            let fromInt64 = std.num.Float64(from: intVal);
            let expected100: std.num.Float64 = 100.0;
            if fromInt64.equals(expected100) == false { return 4 }

            // init(from: Float32)
            let f32val: std.num.Float32 = 2.5;
            let fromF32 = std.num.Float64(from: f32val);
            let expected25: std.num.Float64 = 2.5;
            if fromF32.equals(expected25) == false { return 5 }

            // zero
            let z = std.num.Float64.zero;
            if z.isZero == false { return 6 }

            // one
            let one = std.num.Float64.one;
            let expectedOne: std.num.Float64 = 1.0;
            if one.equals(expectedOne) == false { return 7 }

            // pi - check approximate value
            let pi = std.num.Float64.pi;
            let piLow: std.num.Float64 = 3.14159;
            let piHigh: std.num.Float64 = 3.14160;
            if pi < piLow { return 8 }
            if pi > piHigh { return 9 }

            // e - check approximate value
            let e = std.num.Float64.e;
            let eLow: std.num.Float64 = 2.71828;
            let eHigh: std.num.Float64 = 2.71829;
            if e < eLow { return 10 }
            if e > eHigh { return 11 }

            // tau should be approximately 2*pi
            let tau = std.num.Float64.tau;
            let twoPi = pi.multiply(2.0);
            let diff = tau.subtract(twoPi).abs();
            let eps: std.num.Float64 = 0.0000001;
            if diff > eps { return 12 }

            // ln2
            let ln2 = std.num.Float64.ln2;
            let ln2Low: std.num.Float64 = 0.693147;
            let ln2High: std.num.Float64 = 0.693148;
            if ln2 < ln2Low { return 13 }
            if ln2 > ln2High { return 14 }

            // ln10
            let ln10 = std.num.Float64.ln10;
            let ln10Low: std.num.Float64 = 2.302585;
            let ln10High: std.num.Float64 = 2.302586;
            if ln10 < ln10Low { return 15 }
            if ln10 > ln10High { return 16 }

            // sqrt2
            let sqrt2 = std.num.Float64.sqrt2;
            let sqrt2Low: std.num.Float64 = 1.414213;
            let sqrt2High: std.num.Float64 = 1.414214;
            if sqrt2 < sqrt2Low { return 17 }
            if sqrt2 > sqrt2High { return 18 }

            // minValue should be negative
            let minV = std.num.Float64.minValue;
            if minV.isNegative == false { return 19 }

            // maxValue should be positive
            let maxV = std.num.Float64.maxValue;
            if maxV.isPositive == false { return 20 }

            // minPositive should be very small but positive
            let minPos = std.num.Float64.minPositive;
            if minPos.isPositive == false { return 21 }
            if minPos > one { return 22 }

            // epsilon should be very small but positive
            let epsConst = std.num.Float64.epsilon;
            if epsConst.isPositive == false { return 23 }
            if epsConst > one { return 24 }

            // infinity
            let inf = std.num.Float64.infinity;
            if inf.isInfinite == false { return 25 }

            // nan
            let nan = std.num.Float64.nan;
            if nan.isNaN == false { return 26 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float64_classification() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let normal: std.num.Float64 = 1.0;
            let zero = std.num.Float64.zero;
            let inf = std.num.Float64.infinity;
            let nan = std.num.Float64.nan;
            let negInf = inf.negate();

            // isNaN
            if nan.isNaN == false { return 1 }
            if normal.isNaN { return 2 }
            if inf.isNaN { return 3 }
            if zero.isNaN { return 4 }

            // isInfinite
            if inf.isInfinite == false { return 5 }
            if negInf.isInfinite == false { return 6 }
            if normal.isInfinite { return 7 }
            if nan.isInfinite { return 8 }

            // isFinite
            if normal.isFinite == false { return 9 }
            if zero.isFinite == false { return 10 }
            if inf.isFinite { return 11 }
            if nan.isFinite { return 12 }

            // isNormal
            if normal.isNormal == false { return 13 }
            if zero.isNormal { return 14 }
            if inf.isNormal { return 15 }
            if nan.isNormal { return 16 }

            // isSubnormal - minPositive / 2 should be subnormal
            let minPos = std.num.Float64.minPositive;
            let subnormal = minPos.divide(2.0);
            if subnormal.isSubnormal == false { return 17 }
            if normal.isSubnormal { return 18 }
            if zero.isSubnormal { return 19 }
            if minPos.isSubnormal { return 20 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float64_sign_and_comparison() {
    Test::new(
        r#"module Test

        func main() -> lang.i64 {
            let pos: std.num.Float64 = 3.14;
            let neg: std.num.Float64 = -2.5;
            let zero = std.num.Float64.zero;

            // sign
            let posSign = pos.sign;
            if posSign.equals(1.0) == false { return 1 }
            let negSign = neg.sign;
            if negSign.equals(-1.0) == false { return 2 }
            let zeroSign = zero.sign;
            if zeroSign.equals(0.0) == false { return 3 }

            // isPositive
            if pos.isPositive == false { return 4 }
            if zero.isPositive { return 5 }
            if neg.isPositive { return 6 }

            // isNegative
            if neg.isNegative == false { return 7 }
            if zero.isNegative { return 8 }
            if pos.isNegative { return 9 }

            // isZero
            if zero.isZero == false { return 10 }
            if pos.isZero { return 11 }

            // equals
            let a: std.num.Float64 = 1.5;
            let b: std.num.Float64 = 1.5;
            let c: std.num.Float64 = 2.5;
            if a.equals(b) == false { return 12 }
            if a.equals(c) { return 13 }

            // NaN not equal to itself
            let nan = std.num.Float64.nan;
            if nan.equals(nan) { return 14 }

            // compare
            let one: std.num.Float64 = 1.0;
            let two: std.num.Float64 = 2.0;
            let cmp1 = one.compare(two);
            match cmp1 {
                .Less => 0,
                _ => return 15
            };
            let cmp2 = two.compare(one);
            match cmp2 {
                .Greater => 0,
                _ => return 16
            };
            let cmp3 = one.compare(1.0);
            match cmp3 {
                .Equal => 0,
                _ => return 17
            };

            // infinity comparison
            let inf = std.num.Float64.infinity;
            if inf.isPositive == false { return 18 }
            let negInf = inf.negate();
            if negInf.isNegative == false { return 19 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float64_arithmetic() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            let a: std.num.Float64 = 10.5;
            let b: std.num.Float64 = 3.2;

            // add
            let sum = a.add(b);
            if approxEqual(sum, 13.7) == false { return 1 }

            // subtract
            let diff = a.subtract(b);
            if approxEqual(diff, 7.3) == false { return 2 }

            // multiply
            let prod = a.multiply(b);
            if approxEqual(prod, 33.6) == false { return 3 }

            // divide
            let quot = a.divide(2.0);
            if approxEqual(quot, 5.25) == false { return 4 }

            // negate
            let negA = a.negate();
            if approxEqual(negA, -10.5) == false { return 5 }
            let doubleNeg = negA.negate();
            if approxEqual(doubleNeg, 10.5) == false { return 6 }

            // abs
            let negVal: std.num.Float64 = -7.5;
            let absVal = negVal.abs();
            if approxEqual(absVal, 7.5) == false { return 7 }
            let posVal: std.num.Float64 = 7.5;
            let absPosVal = posVal.abs();
            if approxEqual(absPosVal, 7.5) == false { return 8 }

            // division by zero produces infinity
            let one: std.num.Float64 = 1.0;
            let zero = std.num.Float64.zero;
            let divByZero = one.divide(zero);
            if divByZero.isInfinite == false { return 9 }

            // 0/0 produces NaN
            let zeroOverZero = zero.divide(zero);
            if zeroOverZero.isNaN == false { return 10 }

            // NaN arithmetic produces NaN
            let nan = std.num.Float64.nan;
            if nan.add(1.0).isNaN == false { return 11 }
            if nan.multiply(2.0).isNaN == false { return 12 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float64_basic_math() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            // floor
            let a: std.num.Float64 = 3.7;
            if approxEqual(a.floor(), 3.0) == false { return 1 }
            let b: std.num.Float64 = -3.2;
            if approxEqual(b.floor(), -4.0) == false { return 2 }

            // ceil
            let c: std.num.Float64 = 3.2;
            if approxEqual(c.ceil(), 4.0) == false { return 3 }
            let d: std.num.Float64 = -3.7;
            if approxEqual(d.ceil(), -3.0) == false { return 4 }

            // round
            let e: std.num.Float64 = 3.5;
            if approxEqual(e.round(), 4.0) == false { return 5 }
            let f: std.num.Float64 = 3.4;
            if approxEqual(f.round(), 3.0) == false { return 6 }
            let g: std.num.Float64 = -3.5;
            if approxEqual(g.round(), -4.0) == false { return 7 }

            // trunc
            let h: std.num.Float64 = 3.9;
            if approxEqual(h.trunc(), 3.0) == false { return 8 }
            let i: std.num.Float64 = -3.9;
            if approxEqual(i.trunc(), -3.0) == false { return 9 }

            // fract
            let j: std.num.Float64 = 3.75;
            if approxEqual(j.fract(), 0.75) == false { return 10 }
            let k: std.num.Float64 = -3.75;
            if approxEqual(k.fract(), -0.75) == false { return 11 }

            // sqrt
            let four: std.num.Float64 = 4.0;
            if approxEqual(four.sqrt(), 2.0) == false { return 12 }
            let two: std.num.Float64 = 2.0;
            if approxEqual(two.sqrt(), std.num.Float64.sqrt2) == false { return 13 }
            // sqrt of negative is NaN
            let negOne: std.num.Float64 = -1.0;
            if negOne.sqrt().isNaN == false { return 14 }

            // cbrt
            let eight: std.num.Float64 = 8.0;
            if approxEqual(eight.cbrt(), 2.0) == false { return 15 }
            let negEight: std.num.Float64 = -8.0;
            if approxEqual(negEight.cbrt(), -2.0) == false { return 16 }
            let twentySeven: std.num.Float64 = 27.0;
            if approxEqual(twentySeven.cbrt(), 3.0) == false { return 17 }

            // hypot
            let three: std.num.Float64 = 3.0;
            if approxEqual(three.hypot(4.0), 5.0) == false { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails due to unary minus requiring Negatable protocol resolution
#[test]
fn float64_exp_and_log() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.000001
        }

        func main() -> lang.i64 {
            // exp: e^0 = 1, e^1 = e
            let zero: std.num.Float64 = 0.0;
            if approxEqual(zero.exp(), 1.0) == false { return 1 }
            let one: std.num.Float64 = 1.0;
            if approxEqual(one.exp(), std.num.Float64.e) == false { return 2 }

            // exp2: 2^0 = 1, 2^3 = 8
            if approxEqual(zero.exp2(), 1.0) == false { return 3 }
            let three: std.num.Float64 = 3.0;
            if approxEqual(three.exp2(), 8.0) == false { return 4 }

            // expm1: e^0 - 1 = 0, e^1 - 1 = e - 1
            if approxEqual(zero.expm1(), 0.0) == false { return 5 }
            let expm1One = one.expm1();
            let eMinusOne = std.num.Float64.e.subtract(1.0);
            if approxEqual(expm1One, eMinusOne) == false { return 6 }

            // ln: ln(1) = 0, ln(e) = 1
            if approxEqual(one.ln(), 0.0) == false { return 7 }
            let e = std.num.Float64.e;
            if approxEqual(e.ln(), 1.0) == false { return 8 }

            // ln1p: ln1p(0) = 0
            if approxEqual(zero.ln1p(), 0.0) == false { return 9 }
            // ln1p(1) = ln(2)
            if approxEqual(one.ln1p(), std.num.Float64.ln2) == false { return 10 }

            // log2: log2(1) = 0, log2(8) = 3
            if approxEqual(one.log2(), 0.0) == false { return 11 }
            let eight: std.num.Float64 = 8.0;
            if approxEqual(eight.log2(), 3.0) == false { return 12 }

            // log10: log10(1) = 0, log10(100) = 2
            if approxEqual(one.log10(), 0.0) == false { return 13 }
            let hundred: std.num.Float64 = 100.0;
            if approxEqual(hundred.log10(), 2.0) == false { return 14 }

            // log(base:): log_2(8) = 3
            let logResult = eight.log(2.0);
            if approxEqual(logResult, 3.0) == false { return 15 }
            // log_3(81) = 4
            let eightyOne: std.num.Float64 = 81.0;
            if approxEqual(eightyOne.log(3.0), 4.0) == false { return 16 }

            // pow: 2^10 = 1024
            let two: std.num.Float64 = 2.0;
            if approxEqual(two.pow(10.0), 1024.0) == false { return 17 }
            // pow: 2^0.5 = sqrt(2)
            if approxEqual(two.pow(0.5), std.num.Float64.sqrt2) == false { return 18 }

            // powi: 2^10 = 1024
            let intTen: std.num.Int64 = 10;
            if approxEqual(two.powi(intTen), 1024.0) == false { return 19 }
            // powi: 2^-1 = 0.5
            let negOneInt: std.num.Int64 = -1;
            if approxEqual(two.powi(negOneInt), 0.5) == false { return 20 }

            // ln of negative number is NaN
            let negOne: std.num.Float64 = -1.0;
            if negOne.ln().isNaN == false { return 21 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails due to unary minus requiring Negatable protocol resolution
#[test]
fn float64_trigonometry() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            let pi = std.num.Float64.pi;
            let halfPi = pi.divide(2.0);
            let quarterPi = pi.divide(4.0);
            let zero: std.num.Float64 = 0.0;
            let one: std.num.Float64 = 1.0;

            // sin: sin(0) = 0, sin(pi/2) = 1
            if approxEqual(zero.sin(), 0.0) == false { return 1 }
            if approxEqual(halfPi.sin(), 1.0) == false { return 2 }

            // cos: cos(0) = 1, cos(pi) = -1
            if approxEqual(zero.cos(), 1.0) == false { return 3 }
            if approxEqual(pi.cos(), -1.0) == false { return 4 }

            // tan: tan(0) = 0, tan(pi/4) = 1
            if approxEqual(zero.tan(), 0.0) == false { return 5 }
            if approxEqual(quarterPi.tan(), 1.0) == false { return 6 }

            // asin: asin(0) = 0, asin(1) = pi/2
            if approxEqual(zero.asin(), 0.0) == false { return 7 }
            if approxEqual(one.asin(), halfPi) == false { return 8 }

            // acos: acos(1) = 0, acos(0) = pi/2
            if approxEqual(one.acos(), 0.0) == false { return 9 }
            if approxEqual(zero.acos(), halfPi) == false { return 10 }

            // acos(-1) = pi
            let negOne: std.num.Float64 = -1.0;
            if approxEqual(negOne.acos(), pi) == false { return 11 }

            // atan: atan(0) = 0, atan(1) = pi/4
            if approxEqual(zero.atan(), 0.0) == false { return 12 }
            if approxEqual(one.atan(), quarterPi) == false { return 13 }

            // atan2: atan2(1, 1) = pi/4
            if approxEqual(one.atan2(1.0), quarterPi) == false { return 14 }
            // atan2(1, -1) = 3*pi/4
            let threePiOverFour = pi.multiply(3.0).divide(4.0);
            if approxEqual(one.atan2(-1.0), threePiOverFour) == false { return 15 }

            // sinCos: sin and cos should match individual calls
            let angle: std.num.Float64 = 1.0;
            let (s, c) = angle.sinCos();
            if approxEqual(s, angle.sin()) == false { return 16 }
            if approxEqual(c, angle.cos()) == false { return 17 }

            // asin(2) should be NaN (out of domain)
            let two: std.num.Float64 = 2.0;
            if two.asin().isNaN == false { return 18 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

#[test]
fn float64_hyperbolic() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.000001
        }

        func main() -> lang.i64 {
            let zero: std.num.Float64 = 0.0;
            let one: std.num.Float64 = 1.0;

            // sinh: sinh(0) = 0
            if approxEqual(zero.sinh(), 0.0) == false { return 1 }
            // sinh(1) ~ 1.1752011936438014
            let sinh1: std.num.Float64 = 1.1752011936438014;
            if approxEqual(one.sinh(), sinh1) == false { return 2 }

            // cosh: cosh(0) = 1
            if approxEqual(zero.cosh(), 1.0) == false { return 3 }
            // cosh(1) ~ 1.5430806348152437
            let cosh1: std.num.Float64 = 1.5430806348152437;
            if approxEqual(one.cosh(), cosh1) == false { return 4 }

            // tanh: tanh(0) = 0
            if approxEqual(zero.tanh(), 0.0) == false { return 5 }
            // tanh(inf) = 1
            let inf = std.num.Float64.infinity;
            if approxEqual(inf.tanh(), 1.0) == false { return 6 }

            // asinh: asinh(0) = 0
            if approxEqual(zero.asinh(), 0.0) == false { return 7 }
            // asinh(sinh(1)) = 1 (round-trip)
            if approxEqual(one.sinh().asinh(), 1.0) == false { return 8 }

            // acosh: acosh(1) = 0
            if approxEqual(one.acosh(), 0.0) == false { return 9 }
            // acosh(cosh(1)) = 1 (round-trip)
            if approxEqual(one.cosh().acosh(), 1.0) == false { return 10 }
            // acosh(0.5) = NaN (domain error)
            let half: std.num.Float64 = 0.5;
            if half.acosh().isNaN == false { return 11 }

            // atanh: atanh(0) = 0
            if approxEqual(zero.atanh(), 0.0) == false { return 12 }
            // atanh(tanh(0.5)) = 0.5 (round-trip)
            if approxEqual(half.tanh().atanh(), 0.5) == false { return 13 }
            // atanh(1) = infinity
            if one.atanh().isInfinite == false { return 14 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails due to unary minus requiring Negatable protocol resolution
#[test]
fn float64_ieee754() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            // fma: (2.0 * 3.0) + 4.0 = 10.0
            let two: std.num.Float64 = 2.0;
            let result = two.fma(3.0, 4.0);
            if approxEqual(result, 10.0) == false { return 1 }

            // fma: (5.0 * 0.0) + 1.0 = 1.0
            let five: std.num.Float64 = 5.0;
            if approxEqual(five.fma(0.0, 1.0), 1.0) == false { return 2 }

            // copysign: magnitude of 3.14, sign of -1.0
            let val: std.num.Float64 = 3.14;
            let negCopy = val.copysign(from: -1.0);
            if approxEqual(negCopy, -3.14) == false { return 3 }
            // copysign: magnitude of -3.14, sign of 1.0
            let negVal: std.num.Float64 = -3.14;
            let posCopy = negVal.copysign(from: 1.0);
            if approxEqual(posCopy, 3.14) == false { return 4 }

            // nextUp: 1.0.nextUp() should be slightly greater than 1.0
            let one: std.num.Float64 = 1.0;
            let up = one.nextUp();
            if up > one == false { return 5 }
            // Difference should be very small (epsilon-scale)
            let upDiff = up.subtract(one);
            if upDiff > std.num.Float64.epsilon { return 6 }

            // nextDown: 1.0.nextDown() should be slightly less than 1.0
            let down = one.nextDown();
            if down < one == false { return 7 }

            // nextUp and nextDown should be inverses near 1.0
            let roundTrip = one.nextUp().nextDown();
            if roundTrip.equals(one) == false { return 8 }

            // remainder: IEEE 754 remainder of 5.0 / 3.0 = -1.0
            let fiveF: std.num.Float64 = 5.0;
            let rem = fiveF.remainder(dividingBy: 3.0);
            if approxEqual(rem, -1.0) == false { return 9 }

            // remainder: 7.0 / 4.0 = -1.0 (IEEE 754: rounds quotient to nearest)
            let seven: std.num.Float64 = 7.0;
            if approxEqual(seven.remainder(dividingBy: 4.0), -1.0) == false { return 10 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}

// TODO: Fails due to unary minus (-x) requiring Negatable protocol resolution on Float64
#[test]
fn float64_clamp_lerp_conversion_format() {
    Test::new(
        r#"module Test

        func approxEqual(a: std.num.Float64, b: std.num.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            // clamp: within range
            let val: std.num.Float64 = 0.5;
            if approxEqual(val.clamp(0.0, 1.0), 0.5) == false { return 1 }
            // clamp: below min
            let low: std.num.Float64 = -0.5;
            if approxEqual(low.clamp(0.0, 1.0), 0.0) == false { return 2 }
            // clamp: above max
            let high: std.num.Float64 = 1.5;
            if approxEqual(high.clamp(0.0, 1.0), 1.0) == false { return 3 }
            // clamp: NaN stays NaN
            let nan = std.num.Float64.nan;
            if nan.clamp(0.0, 1.0).isNaN == false { return 4 }

            // lerp: t=0 returns self
            let a: std.num.Float64 = 0.0;
            if approxEqual(a.lerp(to: 10.0, 0.0), 0.0) == false { return 5 }
            // lerp: t=1 returns other
            if approxEqual(a.lerp(to: 10.0, 1.0), 10.0) == false { return 6 }
            // lerp: t=0.5 returns midpoint
            if approxEqual(a.lerp(to: 10.0, 0.5), 5.0) == false { return 7 }
            // lerp: t=0.25
            if approxEqual(a.lerp(to: 10.0, 0.25), 2.5) == false { return 8 }

            // toInt64: truncates toward zero
            let pos: std.num.Float64 = 3.7;
            let intResult = pos.toInt64();
            if intResult.isNone() { return 9 }
            if intResult.unwrap() != 3 { return 10 }
            let neg: std.num.Float64 = -3.7;
            let negIntResult = neg.toInt64();
            if negIntResult.isNone() { return 11 }
            if negIntResult.unwrap() != -3 { return 12 }
            // toInt64: NaN returns None
            if nan.toInt64().isSome() { return 13 }
            // toInt64: infinity returns None
            let inf = std.num.Float64.infinity;
            if inf.toInt64().isSome() { return 14 }

            // toFloat32
            let f64val: std.num.Float64 = 3.14;
            let f32val = f64val.toFloat32();
            // Verify it's approximately correct by converting back
            let backToF64 = std.num.Float64(from: f32val);
            let convDiff = backToF64.subtract(f64val).abs();
            if convDiff > 0.001 { return 15 }

            // parse: valid decimal
            let parsed = std.num.Float64.parse( "3.14");
            if parsed.isNone() { return 16 }
            if approxEqual(parsed.unwrap(), 3.14) == false { return 17 }

            // parse: negative
            let parsedNeg = std.num.Float64.parse( "-2.5");
            if parsedNeg.isNone() { return 18 }
            if approxEqual(parsedNeg.unwrap(), -2.5) == false { return 19 }

            // parse: integer string
            let parsedInt = std.num.Float64.parse( "42");
            if parsedInt.isNone() { return 20 }
            if approxEqual(parsedInt.unwrap(), 42.0) == false { return 21 }

            // parse: scientific notation
            let parsedSci = std.num.Float64.parse( "1.5e2");
            if parsedSci.isNone() { return 22 }
            if approxEqual(parsedSci.unwrap(), 150.0) == false { return 23 }

            // parse: "nan"
            let parsedNaN = std.num.Float64.parse( "nan");
            if parsedNaN.isNone() { return 24 }
            if parsedNaN.unwrap().isNaN == false { return 25 }

            // parse: "inf"
            let parsedInf = std.num.Float64.parse( "inf");
            if parsedInf.isNone() { return 26 }
            if parsedInf.unwrap().isInfinite == false { return 27 }

            // parse: invalid
            let parsedBad = std.num.Float64.parse( "abc");
            if parsedBad.isSome() { return 28 }

            // parse: empty string
            let parsedEmpty = std.num.Float64.parse( "");
            if parsedEmpty.isSome() { return 29 }

            // format: default
            let fmtVal: std.num.Float64 = 3.14;
            let fmtStr = fmtVal.format();
            if fmtStr.contains("3.14") == false { return 30 }

            0
        }
    "#,
    )
    .with_stdlib()
    .expect(Compiles)
    .expect(Runs);
}
