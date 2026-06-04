// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let maxVal = std.numeric.UInt64.maxValue;
            let minVal = std.numeric.UInt64.minValue;
            let one: std.numeric.UInt64 = 1;
            let two: std.numeric.UInt64 = 2;
            let large: std.numeric.UInt64 = 10000000000000000000;

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            let three: std.numeric.UInt64 = 3;
            if addNorm.unwrap().isEqual(to: three) == false { return 2 }

            // addChecked — overflow at max
            let addOver = maxVal.addChecked(one);
            if addOver.isSome() { return 3 }

            // subtractChecked — normal case
            let subNorm = two.subtractChecked(one);
            if subNorm.isNone() { return 4 }
            if subNorm.unwrap().isEqual(to: one) == false { return 5 }

            // subtractChecked — underflow at 0
            let subUnder = minVal.subtractChecked(one);
            if subUnder.isSome() { return 6 }

            // multiplyChecked — normal case
            let mulThree: std.numeric.UInt64 = 3;
            let mulNorm = two.multiplyChecked(mulThree);
            if mulNorm.isNone() { return 7 }
            let six: std.numeric.UInt64 = 6;
            if mulNorm.unwrap().isEqual(to: six) == false { return 8 }

            // multiplyChecked — overflow near max
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let hundred: std.numeric.UInt64 = 100;
            let addSat = maxVal.addSaturating(hundred);
            if addSat.isEqual(to: maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            let addSatThree: std.numeric.UInt64 = 3;
            if addSatNorm.isEqual(to: addSatThree) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.isEqual(to: std.numeric.UInt64.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.isEqual(to: one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.isEqual(to: maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatThree: std.numeric.UInt64 = 3;
            let mulSatNorm = two.multiplySaturating(mulSatThree);
            let mulSatSix: std.numeric.UInt64 = 6;
            if mulSatNorm.isEqual(to: mulSatSix) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to maxValue
            let wrapped = minVal.subtract(one);
            if wrapped.isEqual(to: maxVal) == false { return 16 }

            0
        }
