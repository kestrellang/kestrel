// test: execution
// stdlib: true

module Test

        @main
        func main() -> lang.i64 {
            let maxVal = std.numeric.Int32.maxValue;
            let minVal = std.numeric.Int32.minValue;
            let one: std.numeric.Int32 = 1;
            let negOne: std.numeric.Int32 = -1;

            // addChecked — overflow at 2147483647
            let addOverflow = maxVal.addChecked(one);
            if addOverflow.isSome() { return 1 }

            // addChecked — normal case
            let thousand: std.numeric.Int32 = 1000;
            let fiveHundred: std.numeric.Int32 = 500;
            let addNormal = thousand.addChecked(fiveHundred);
            if addNormal.isNone() { return 2 }
            let expectedAddNormal: std.numeric.Int32 = 1500;
            if addNormal.unwrap() != expectedAddNormal { return 3 }

            // subtractChecked — underflow at -2147483648
            let subOverflow = minVal.subtractChecked(one);
            if subOverflow.isSome() { return 4 }

            // subtractChecked — normal case
            let subNormal = thousand.subtractChecked(fiveHundred);
            if subNormal.isNone() { return 5 }
            if subNormal.unwrap() != fiveHundred { return 6 }

            // multiplyChecked — overflow near boundaries
            let bigVal: std.numeric.Int32 = 2000000000;
            let two: std.numeric.Int32 = 2;
            let mulOverflow = bigVal.multiplyChecked(two);
            if mulOverflow.isSome() { return 7 }

            // multiplyChecked — normal case
            let ten: std.numeric.Int32 = 10;
            let five: std.numeric.Int32 = 5;
            let mulNormal = ten.multiplyChecked(five);
            if mulNormal.isNone() { return 8 }
            let expectedMulNormal: std.numeric.Int32 = 50;
            if mulNormal.unwrap() != expectedMulNormal { return 9 }

            // negateChecked — overflow at -2147483648
            let negMin = minVal.negateChecked();
            if negMin.isSome() { return 10 }

            // negateChecked — normal case
            let negThousand = thousand.negateChecked();
            if negThousand.isNone() { return 11 }
            let expectedNegThousand: std.numeric.Int32 = -1000;
            if negThousand.unwrap() != expectedNegThousand { return 12 }

            // absChecked — overflow at -2147483648
            let absMin = minVal.absChecked();
            if absMin.isSome() { return 13 }

            // absChecked — normal case
            let negFiveHundred: std.numeric.Int32 = -500;
            let absFiveHundred = negFiveHundred.absChecked();
            if absFiveHundred.isNone() { return 14 }
            if absFiveHundred.unwrap() != fiveHundred { return 15 }

            // addSaturating — clamps to 2147483647
            let addSat = maxVal.addSaturating(one);
            if addSat != maxVal { return 16 }
            let hundredThousand: std.numeric.Int32 = 100000;
            let addSatBig = maxVal.addSaturating(hundredThousand);
            if addSatBig != maxVal { return 17 }

            // addSaturating — clamps to -2147483648
            let addSatNeg = minVal.addSaturating(negOne);
            if addSatNeg != minVal { return 18 }

            // subtractSaturating — clamps to -2147483648
            let subSat = minVal.subtractSaturating(one);
            if subSat != minVal { return 19 }

            // subtractSaturating — clamps to 2147483647
            let subSatPos = maxVal.subtractSaturating(negOne);
            if subSatPos != maxVal { return 20 }

            // multiplySaturating — clamps to 2147483647
            let mulSat = bigVal.multiplySaturating(two);
            if mulSat != maxVal { return 21 }

            // multiplySaturating — clamps to -2147483648 (positive * negative overflow)
            let negBigVal: std.numeric.Int32 = -2000000000;
            let mulSatNeg = negBigVal.multiplySaturating(two);
            if mulSatNeg != minVal { return 22 }

            // negateSaturating — -2147483648 saturates to 2147483647
            let negSatMin = minVal.negateSaturating();
            if negSatMin != maxVal { return 23 }

            // negateSaturating — normal case
            let negSatThousand = thousand.negateSaturating();
            let expectedNegSatThousand: std.numeric.Int32 = -1000;
            if negSatThousand != expectedNegSatThousand { return 24 }

            // absSaturating — -2147483648 saturates to 2147483647
            let absSatMin = minVal.absSaturating();
            if absSatMin != maxVal { return 25 }

            // absSaturating — normal case
            let absSatNeg = negFiveHundred.absSaturating();
            if absSatNeg != fiveHundred { return 26 }

            0
        }
