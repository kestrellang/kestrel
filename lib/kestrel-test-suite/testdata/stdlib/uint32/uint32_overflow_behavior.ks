// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let maxVal = std.numeric.UInt32.maxValue;
            let minVal = std.numeric.UInt32.minValue;
            let one: std.numeric.UInt32 = 1;
            let two: std.numeric.UInt32 = 2;
            let large: std.numeric.UInt32 = 3000000000;

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            let three: std.numeric.UInt32 = 3;
            if addNorm.unwrap().equals(three) == false { return 2 }

            // addChecked — overflow at 4294967295
            let addOver = maxVal.addChecked(one);
            if addOver.isSome() { return 3 }

            // subtractChecked — normal case
            let subNorm = two.subtractChecked(one);
            if subNorm.isNone() { return 4 }
            if subNorm.unwrap().equals(one) == false { return 5 }

            // subtractChecked — underflow at 0
            let subUnder = minVal.subtractChecked(one);
            if subUnder.isSome() { return 6 }

            // multiplyChecked — normal case
            let mulThree: std.numeric.UInt32 = 3;
            let mulNorm = two.multiplyChecked(mulThree);
            if mulNorm.isNone() { return 7 }
            let six: std.numeric.UInt32 = 6;
            if mulNorm.unwrap().equals(six) == false { return 8 }

            // multiplyChecked — overflow near 4294967295
            let mulOver = large.multiplyChecked(two);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let hundred: std.numeric.UInt32 = 100;
            let addSat = maxVal.addSaturating(hundred);
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            let addSatThree: std.numeric.UInt32 = 3;
            if addSatNorm.equals(addSatThree) == false { return 11 }

            // subtractSaturating — clamps to 0
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.numeric.UInt32.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSat = large.multiplySaturating(two);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatThree: std.numeric.UInt32 = 3;
            let mulSatNorm = two.multiplySaturating(mulSatThree);
            let mulSatSix: std.numeric.UInt32 = 6;
            if mulSatNorm.equals(mulSatSix) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to 4294967295
            let wrapped = minVal.subtract(one);
            if wrapped.equals(maxVal) == false { return 16 }

            0
        }
