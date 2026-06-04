// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        @main
        func main() -> lang.i64 {
            // floor
            let a: std.numeric.Float64 = 3.7;
            if approxEqual(a.floor(), 3.0) == false { return 1 }
            let b: std.numeric.Float64 = -3.2;
            if approxEqual(b.floor(), -4.0) == false { return 2 }

            // ceil
            let c: std.numeric.Float64 = 3.2;
            if approxEqual(c.ceil(), 4.0) == false { return 3 }
            let d: std.numeric.Float64 = -3.7;
            if approxEqual(d.ceil(), -3.0) == false { return 4 }

            // round
            let e: std.numeric.Float64 = 3.5;
            if approxEqual(e.round(), 4.0) == false { return 5 }
            let f: std.numeric.Float64 = 3.4;
            if approxEqual(f.round(), 3.0) == false { return 6 }
            let g: std.numeric.Float64 = -3.5;
            if approxEqual(g.round(), -4.0) == false { return 7 }

            // trunc
            let h: std.numeric.Float64 = 3.9;
            if approxEqual(h.trunc(), 3.0) == false { return 8 }
            let i: std.numeric.Float64 = -3.9;
            if approxEqual(i.trunc(), -3.0) == false { return 9 }

            // fract
            let j: std.numeric.Float64 = 3.75;
            if approxEqual(j.fract(), 0.75) == false { return 10 }
            let k: std.numeric.Float64 = -3.75;
            if approxEqual(k.fract(), -0.75) == false { return 11 }

            // sqrt
            let four: std.numeric.Float64 = 4.0;
            if approxEqual(four.sqrt(), 2.0) == false { return 12 }
            let two: std.numeric.Float64 = 2.0;
            if approxEqual(two.sqrt(), std.numeric.Float64.sqrt2) == false { return 13 }
            // sqrt of negative is NaN
            let negOne: std.numeric.Float64 = -1.0;
            if negOne.sqrt().isNaN == false { return 14 }

            // cbrt
            let eight: std.numeric.Float64 = 8.0;
            if approxEqual(eight.cbrt(), 2.0) == false { return 15 }
            let negEight: std.numeric.Float64 = -8.0;
            if approxEqual(negEight.cbrt(), -2.0) == false { return 16 }
            let twentySeven: std.numeric.Float64 = 27.0;
            if approxEqual(twentySeven.cbrt(), 3.0) == false { return 17 }

            // hypot
            let three: std.numeric.Float64 = 3.0;
            if approxEqual(three.hypot(4.0), 5.0) == false { return 18 }

            0
        }
