// test: execution
// stdlib: true

module Test

        func approxEqual(a: std.numeric.Float64, b: std.numeric.Float64) -> std.core.Bool {
            let diff = a.subtract(b).abs();
            diff < 0.0000001
        }

        @main
        func main() -> lang.i64 {
            // clamp: within range
            let val: std.numeric.Float64 = 0.5;
            if approxEqual(val.clamp(0.0, 1.0), 0.5) == false { return 1 }
            // clamp: below min
            let low: std.numeric.Float64 = -0.5;
            if approxEqual(low.clamp(0.0, 1.0), 0.0) == false { return 2 }
            // clamp: above max
            let high: std.numeric.Float64 = 1.5;
            if approxEqual(high.clamp(0.0, 1.0), 1.0) == false { return 3 }
            // clamp: NaN stays NaN
            let nan = std.numeric.Float64.nan;
            if nan.clamp(0.0, 1.0).isNaN == false { return 4 }

            // lerp: t=0 returns self
            let a: std.numeric.Float64 = 0.0;
            if approxEqual(a.lerp(to: 10.0, 0.0), 0.0) == false { return 5 }
            // lerp: t=1 returns other
            if approxEqual(a.lerp(to: 10.0, 1.0), 10.0) == false { return 6 }
            // lerp: t=0.5 returns midpoint
            if approxEqual(a.lerp(to: 10.0, 0.5), 5.0) == false { return 7 }
            // lerp: t=0.25
            if approxEqual(a.lerp(to: 10.0, 0.25), 2.5) == false { return 8 }

            // toInt64: truncates toward zero
            let pos: std.numeric.Float64 = 3.7;
            let intResult = pos.toInt64();
            if intResult.isNone() { return 9 }
            if intResult.unwrap() != 3 { return 10 }
            let neg: std.numeric.Float64 = -3.7;
            let negIntResult = neg.toInt64();
            if negIntResult.isNone() { return 11 }
            if negIntResult.unwrap() != -3 { return 12 }
            // toInt64: NaN returns None
            if nan.toInt64().isSome() { return 13 }
            // toInt64: infinity returns None
            let inf = std.numeric.Float64.infinity;
            if inf.toInt64().isSome() { return 14 }

            // toFloat32
            let f64val: std.numeric.Float64 = 3.14;
            let f32val = f64val.toFloat32();
            // Verify it's approximately correct by converting back
            let backToF64 = std.numeric.Float64(from: f32val);
            let convDiff = backToF64.subtract(f64val).abs();
            if convDiff > 0.001 { return 15 }

            // parse: valid decimal
            let parsed = std.numeric.Float64(parsing: "3.14");
            if parsed.isNone() { return 16 }
            if approxEqual(parsed.unwrap(), 3.14) == false { return 17 }

            // parse: negative
            let parsedNeg = std.numeric.Float64(parsing: "-2.5");
            if parsedNeg.isNone() { return 18 }
            if approxEqual(parsedNeg.unwrap(), -2.5) == false { return 19 }

            // parse: integer string
            let parsedInt = std.numeric.Float64(parsing: "42");
            if parsedInt.isNone() { return 20 }
            if approxEqual(parsedInt.unwrap(), 42.0) == false { return 21 }

            // parse: scientific notation
            let parsedSci = std.numeric.Float64(parsing: "1.5e2");
            if parsedSci.isNone() { return 22 }
            if approxEqual(parsedSci.unwrap(), 150.0) == false { return 23 }

            // parse: "nan"
            let parsedNaN = std.numeric.Float64(parsing: "nan");
            if parsedNaN.isNone() { return 24 }
            if parsedNaN.unwrap().isNaN == false { return 25 }

            // parse: "inf"
            let parsedInf = std.numeric.Float64(parsing: "inf");
            if parsedInf.isNone() { return 26 }
            if parsedInf.unwrap().isInfinite == false { return 27 }

            // parse: invalid
            let parsedBad = std.numeric.Float64(parsing: "abc");
            if parsedBad.isSome() { return 28 }

            // parse: empty string
            let parsedEmpty = std.numeric.Float64(parsing: "");
            if parsedEmpty.isSome() { return 29 }

            // format: default
            let fmtVal: std.numeric.Float64 = 3.14;
            let fmtStr = fmtVal.formatted();
            if fmtStr.contains("3.14") == false { return 30 }

            0
        }
