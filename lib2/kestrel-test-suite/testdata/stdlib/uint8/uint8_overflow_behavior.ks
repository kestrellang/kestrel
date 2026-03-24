// test: execution
// stdlib: true

module Test

        func main() -> lang.i64 {
            let maxVal = std.num.UInt8.maxValue;
            let minVal = std.num.UInt8.minValue;
            let one: std.num.UInt8 = 1;
            let two: std.num.UInt8 = 2;
            let hundred: std.num.UInt8 = 100;

            // addChecked — normal case
            let addNorm = one.addChecked(two);
            if addNorm.isNone() { return 1 }
            let three: std.num.UInt8 = 3;
            if addNorm.unwrap().equals(three) == false { return 2 }

            // addChecked — overflow at 255
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
            let mulThree: std.num.UInt8 = 3;
            let mulNorm = two.multiplyChecked(mulThree);
            if mulNorm.isNone() { return 7 }
            let six: std.num.UInt8 = 6;
            if mulNorm.unwrap().equals(six) == false { return 8 }

            // multiplyChecked — overflow near 255
            let mulOverThree: std.num.UInt8 = 3;
            let mulOver = hundred.multiplyChecked(mulOverThree);
            if mulOver.isSome() { return 9 }

            // addSaturating — clamps to maxValue
            let ten: std.num.UInt8 = 10;
            let addSat = maxVal.addSaturating(ten);
            if addSat.equals(maxVal) == false { return 10 }

            // addSaturating — normal case
            let addSatNorm = one.addSaturating(two);
            let addSatThree: std.num.UInt8 = 3;
            if addSatNorm.equals(addSatThree) == false { return 11 }

            // subtractSaturating — clamps to 0 (no negative)
            let subSat = minVal.subtractSaturating(one);
            if subSat.equals(std.num.UInt8.zero) == false { return 12 }

            // subtractSaturating — normal case
            let subSatNorm = two.subtractSaturating(one);
            if subSatNorm.equals(one) == false { return 13 }

            // multiplySaturating — clamps to maxValue
            let mulSatThree: std.num.UInt8 = 3;
            let mulSat = hundred.multiplySaturating(mulSatThree);
            if mulSat.equals(maxVal) == false { return 14 }

            // multiplySaturating — normal case
            let mulSatNormThree: std.num.UInt8 = 3;
            let mulSatNorm = two.multiplySaturating(mulSatNormThree);
            let mulSatSix: std.num.UInt8 = 6;
            if mulSatNorm.equals(mulSatSix) == false { return 15 }

            // Subtraction wrapping behavior: 0 - 1 wraps to 255
            let wrapped = minVal.subtract(one);
            if wrapped.equals(maxVal) == false { return 16 }

            0
        }
