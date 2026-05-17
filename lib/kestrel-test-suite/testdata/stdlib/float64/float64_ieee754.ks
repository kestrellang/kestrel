// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        func main() -> lang.i64 {
            // fma: (2.0 * 3.0) + 4.0 = 10.0
            let two: std.numeric.Float64 = 2.0;
            let result = two.fma(3.0, 4.0);
            if approxEqual(result, 10.0) == false { return 1 }

            // fma: (5.0 * 0.0) + 1.0 = 1.0
            let five: std.numeric.Float64 = 5.0;
            if approxEqual(five.fma(0.0, 1.0), 1.0) == false { return 2 }

            // copysign: magnitude of 3.14, sign of -1.0
            let val: std.numeric.Float64 = 3.14;
            let negCopy = val.copysign(from: -1.0);
            if approxEqual(negCopy, -3.14) == false { return 3 }
            // copysign: magnitude of -3.14, sign of 1.0
            let negVal: std.numeric.Float64 = -3.14;
            let posCopy = negVal.copysign(from: 1.0);
            if approxEqual(posCopy, 3.14) == false { return 4 }

            // nextUp: 1.0.nextUp() should be slightly greater than 1.0
            let one: std.numeric.Float64 = 1.0;
            let up = one.nextUp();
            if up > one == false { return 5 }
            // Difference should be very small (epsilon-scale)
            let upDiff = up.subtract(one);
            if upDiff > std.numeric.Float64.epsilon { return 6 }

            // nextDown: 1.0.nextDown() should be slightly less than 1.0
            let down = one.nextDown();
            if down < one == false { return 7 }

            // nextUp and nextDown should be inverses near 1.0
            let roundTrip = one.nextUp().nextDown();
            if roundTrip.isEqual(to: one) == false { return 8 }

            // remainder: IEEE 754 remainder of 5.0 / 3.0 = -1.0
            let fiveF: std.numeric.Float64 = 5.0;
            let rem = fiveF.remainder(dividingBy: 3.0);
            if approxEqual(rem, -1.0) == false { return 9 }

            // remainder: 7.0 / 4.0 = -1.0 (IEEE 754: rounds quotient to nearest)
            let seven: std.numeric.Float64 = 7.0;
            if approxEqual(seven.remainder(dividingBy: 4.0), -1.0) == false { return 10 }

            0
        }
