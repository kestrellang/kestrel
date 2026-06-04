// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let pos: std.numeric.Float64 = 3.14;
            let neg: std.numeric.Float64 = -2.5;
            let zero = std.numeric.Float64.zero;

            // sign
            let posSign = pos.sign;
            if posSign.isEqual(to: 1.0) == false { return 1 }
            let negSign = neg.sign;
            if negSign.isEqual(to: -1.0) == false { return 2 }
            let zeroSign = zero.sign;
            if zeroSign.isEqual(to: 0.0) == false { return 3 }

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
            let a: std.numeric.Float64 = 1.5;
            let b: std.numeric.Float64 = 1.5;
            let c: std.numeric.Float64 = 2.5;
            if a.isEqual(to: b) == false { return 12 }
            if a.isEqual(to: c) { return 13 }

            // NaN not equal to itself
            let nan = std.numeric.Float64.nan;
            if nan.isEqual(to: nan) { return 14 }

            // compare
            let one: std.numeric.Float64 = 1.0;
            let two: std.numeric.Float64 = 2.0;
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
            let inf = std.numeric.Float64.infinity;
            if inf.isPositive == false { return 18 }
            let negInf = inf.negate();
            if negInf.isNegative == false { return 19 }

            0
        }
