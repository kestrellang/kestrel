// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            let a: std.numeric.Float64 = 10.5;
            let b: std.numeric.Float64 = 3.2;

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
            let negVal: std.numeric.Float64 = -7.5;
            let absVal = negVal.abs();
            if approxEqual(absVal, 7.5) == false { return 7 }
            let posVal: std.numeric.Float64 = 7.5;
            let absPosVal = posVal.abs();
            if approxEqual(absPosVal, 7.5) == false { return 8 }

            // division by zero produces infinity
            let one: std.numeric.Float64 = 1.0;
            let zero = std.numeric.Float64.zero;
            let divByZero = one.divide(zero);
            if divByZero.isInfinite == false { return 9 }

            // 0/0 produces NaN
            let zeroOverZero = zero.divide(zero);
            if zeroOverZero.isNaN == false { return 10 }

            // NaN arithmetic produces NaN
            let nan = std.numeric.Float64.nan;
            if nan.add(1.0).isNaN == false { return 11 }
            if nan.multiply(2.0).isNaN == false { return 12 }

            0
        }
