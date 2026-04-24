// test: execution
// stdlib: true

module Test

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
